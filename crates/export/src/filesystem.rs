//! Filesystem traversal, locking and staged replacement, independent of content schemas.
use crate::{ExportError, Result};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};
use walkdir::{DirEntry, WalkDir};

pub(crate) fn locked<T>(output: &Path, build: impl FnOnce() -> Result<T>) -> Result<T> {
    let parent = parent(output);
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .ok_or_else(|| ExportError::invalid_input("output needs a directory name"))?
        .to_string_lossy();
    let backup = parent.join(format!(".{name}.export-backup"));
    let lock = parent.join(format!(".{name}.export-lock"));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)?;
    struct Lock<'a>(&'a Path);
    impl Drop for Lock<'_> {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0);
        }
    }
    let _lock = Lock(&lock);
    if backup.exists() {
        return Err(ExportError::invalid_input(format!(
            "export backup exists; recover it before retrying: {}",
            backup.display()
        )));
    }
    build()
}

pub(crate) fn transaction<T>(output: &Path, build: impl FnOnce(&Path) -> Result<T>) -> Result<T> {
    locked(output, || {
        let parent = parent(output);
        let name = output
            .file_name()
            .expect("validated by lock")
            .to_string_lossy();
        let backup = parent.join(format!(".{name}.export-backup"));
        let stage = tempfile::Builder::new()
            .prefix(".export-stage-")
            .tempdir_in(parent)?;
        if output.exists() {
            copy_tree(output, stage.path())?;
        }
        let result = build(stage.path())?;
        let existed = output.exists();
        if existed && same_tree(output, stage.path())? {
            return Ok(result);
        }
        if existed {
            fs::rename(output, &backup)?;
        }
        if let Err(error) = fs::rename(stage.path(), output) {
            if existed {
                fs::rename(&backup, output)?;
            }
            return Err(error.into());
        }
        if existed {
            fs::remove_dir_all(backup)?;
        }
        Ok(result)
    })
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    // WalkDir visits directories before their children.
    for entry in walk(from, true) {
        let entry = entry?;
        let dest = to.join(entry.path().strip_prefix(from)?);
        if entry.file_type().is_dir() {
            fs::create_dir_all(dest)?;
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn same_tree(left_root: &Path, right_root: &Path) -> Result<bool> {
    let mut left_entries = walk(left_root, true);
    let mut right_entries = walk(right_root, true);
    loop {
        match (
            left_entries.next().transpose()?,
            right_entries.next().transpose()?,
        ) {
            (Some(left), Some(right)) => {
                let kind = left.file_type();
                if left.path().strip_prefix(left_root)? != right.path().strip_prefix(right_root)?
                    || kind != right.file_type()
                    || (kind.is_file() && !same_file(left.path(), right.path())?)
                {
                    return Ok(false);
                }
            }
            (None, None) => return Ok(true),
            _ => return Ok(false),
        }
    }
}

fn same_file(a: &Path, b: &Path) -> io::Result<bool> {
    let mut a = fs::File::open(a)?;
    let mut b = fs::File::open(b)?;
    let mut remaining = a.metadata()?.len();
    if remaining != b.metadata()?.len() {
        return Ok(false);
    }
    let mut left = [0; 8 * 1024];
    let mut right = [0; 8 * 1024];
    while remaining > 0 {
        let len = remaining.min(left.len() as u64) as usize;
        a.read_exact(&mut left[..len])?;
        b.read_exact(&mut right[..len])?;
        if left[..len] != right[..len] {
            return Ok(false);
        }
        remaining -= len as u64;
    }
    Ok(true)
}

/// Collect visible files in a deterministic order for content processing.
pub(crate) fn files(root: &Path) -> Result<Vec<PathBuf>> {
    walk(root, false)
        .filter_map(|entry| match entry {
            Ok(entry) if entry.file_type().is_file() => Some(Ok(entry.into_path())),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

/// Sort within each directory and stream entries, rejecting symlinks instead of following them.
pub(crate) fn walk(root: &Path, hidden: bool) -> impl Iterator<Item = Result<DirEntry>> {
    WalkDir::new(root)
        .follow_links(false)
        .follow_root_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(move |entry| {
            entry.depth() == 0 || hidden || !entry.file_name().to_string_lossy().starts_with('.')
        })
        .map(|entry| {
            let entry = entry.map_err(io::Error::from)?;
            let kind = entry.file_type();
            if kind.is_symlink() {
                return Err(ExportError::invalid_input(format!(
                    "symlink input is not allowed: {}",
                    entry.path().display()
                )));
            }
            if entry.depth() == 0 && !kind.is_dir() {
                return Err(io::Error::from(io::ErrorKind::NotADirectory).into());
            }
            Ok(entry)
        })
}

pub(crate) fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn changed_tree_preserves_hidden_files_and_empty_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("content");
        fs::create_dir_all(output.join(".user/nested/empty")).unwrap();
        fs::write(output.join(".user/notes.txt"), "keep this").unwrap();

        transaction(&output, |stage| {
            assert!(stage.join(".user/nested/empty").is_dir());
            fs::write(stage.join("new.txt"), "new content")?;
            Ok(())
        })
        .unwrap();

        assert!(output.join(".user/nested/empty").is_dir());
        assert_eq!(
            fs::read(output.join(".user/notes.txt")).unwrap(),
            b"keep this"
        );
        assert_eq!(fs::read(output.join("new.txt")).unwrap(), b"new content");
    }

    #[rstest::rstest]
    #[case("unchanged", true)]
    #[case("contents", false)]
    #[case("length", false)]
    #[case("renamed", false)]
    #[case("added", false)]
    #[case("removed", false)]
    #[case("empty_directory", false)]
    #[case("file_to_directory", false)]
    fn tree_comparison_checks_paths_types_and_bytes(#[case] change: &str, #[case] equal: bool) {
        let left = tempfile::tempdir().unwrap();
        let right = tempfile::tempdir().unwrap();
        // Differing creation order must not affect equality. Include several read buffers.
        let contents = vec![b'a'; 128 * 1024 + 1];
        for name in ["nested/a.bin", "nested/z.bin", ".hidden"] {
            let file = left.path().join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, &contents).unwrap();
        }
        for name in [".hidden", "nested/z.bin", "nested/a.bin"] {
            let file = right.path().join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, &contents).unwrap();
        }
        let file = right.path().join("nested/a.bin");
        match change {
            "unchanged" => {}
            "contents" => {
                let mut changed = contents;
                *changed.last_mut().unwrap() = b'b';
                fs::write(file, changed).unwrap();
            }
            "length" => fs::write(file, &contents[..contents.len() - 1]).unwrap(),
            "renamed" => fs::rename(file, right.path().join("nested/b.bin")).unwrap(),
            "added" => fs::write(right.path().join("extra.txt"), "extra").unwrap(),
            "removed" => fs::remove_file(file).unwrap(),
            "empty_directory" => fs::create_dir(right.path().join("empty")).unwrap(),
            "file_to_directory" => {
                fs::remove_file(&file).unwrap();
                fs::create_dir(file).unwrap();
            }
            _ => unreachable!(),
        }

        assert_eq!(same_tree(left.path(), right.path()).unwrap(), equal);
        assert_eq!(same_tree(right.path(), left.path()).unwrap(), equal);
    }

    #[cfg(unix)]
    #[test]
    fn transaction_rejects_existing_symlinks_before_build() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("content");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("file.txt"), "original").unwrap();
        std::os::unix::fs::symlink("missing", output.join(".link")).unwrap();

        let result = transaction(&output, |_| -> Result<()> {
            panic!("build must not run while output contains a symlink");
        });

        assert!(matches!(result, Err(ExportError::InvalidInput(_))));
        assert_eq!(fs::read(output.join("file.txt")).unwrap(), b"original");
        assert!(output.join(".link").is_symlink());
    }

    #[test]
    fn leftover_backup_stops_before_build_and_releases_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("content");
        let backup = tmp.path().join(".content.export-backup");
        fs::create_dir(&backup).unwrap();
        fs::write(backup.join("file.txt"), "recover this").unwrap();

        let result = locked(&output, || -> Result<()> {
            panic!("build must not run while a backup exists");
        });

        assert!(matches!(result, Err(ExportError::InvalidInput(_))));
        assert_eq!(
            fs::read_to_string(backup.join("file.txt")).unwrap(),
            "recover this"
        );
        assert!(!output.exists());
        assert!(!tmp.path().join(".content.export-lock").exists());
    }

    #[test]
    fn failed_build_preserves_output_and_removes_stage_and_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("content");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("file.txt"), "original").unwrap();
        let mut stage_path = None;

        let result: Result<()> = transaction(&output, |stage| {
            stage_path = Some(stage.to_path_buf());
            assert!(tmp.path().join(".content.export-lock").exists());
            fs::write(stage.join("file.txt"), "changed")?;
            fs::write(stage.join("new.txt"), "uncommitted")?;
            Err(ExportError::invalid_input("build failed"))
        });

        assert!(matches!(result, Err(ExportError::InvalidInput(_))));
        assert_eq!(
            fs::read_to_string(output.join("file.txt")).unwrap(),
            "original"
        );
        assert!(!output.join("new.txt").exists());
        assert!(!stage_path.unwrap().exists());
        assert!(!tmp.path().join(".content.export-lock").exists());
        assert!(!tmp.path().join(".content.export-backup").exists());
    }

    #[test]
    fn unchanged_files_keep_original_mtime_and_remove_stage_and_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let output = tmp.path().join("content");
        fs::create_dir(&output).unwrap();
        let file = output.join("file.txt");
        fs::write(&file, "unchanged").unwrap();
        let modified = fs::metadata(&file).unwrap().modified().unwrap();

        let stage = transaction(&output, |stage| {
            fs::File::options()
                .write(true)
                .open(stage.join("file.txt"))?
                .set_modified(modified + Duration::from_secs(60))?;
            Ok(stage.to_path_buf())
        })
        .unwrap();

        assert_eq!(fs::read_to_string(&file).unwrap(), "unchanged");
        assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), modified);
        assert!(!stage.exists());
        assert!(!tmp.path().join(".content.export-lock").exists());
        assert!(!tmp.path().join(".content.export-backup").exists());
    }
}
