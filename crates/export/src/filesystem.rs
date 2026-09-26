//! Filesystem traversal, locking and staged replacement, independent of content schemas.
use crate::{ExportError, Result};
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

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
    for path in all_files(from)? {
        let dest = to.join(path.strip_prefix(from)?);
        fs::create_dir_all(dest.parent().unwrap())?;
        fs::copy(path, dest)?;
    }
    // Preserve empty managed directories as well.
    for locale in ["ja", "en"] {
        if from.join(locale).is_dir() {
            fs::create_dir_all(to.join(locale))?;
        }
    }
    Ok(())
}

fn same_tree(a: &Path, b: &Path) -> Result<bool> {
    let aa = all_files(a)?;
    let bb = all_files(b)?;
    if aa.len() != bb.len() {
        return Ok(false);
    }
    for (left, right) in aa.iter().zip(bb.iter()) {
        if left.strip_prefix(a)? != right.strip_prefix(b)? || fs::read(left)? != fs::read(right)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Reject symlinks instead of silently including content outside the selected root.
pub(crate) fn files(root: &Path) -> Result<Vec<PathBuf>> {
    walk(root, false)
}

pub(crate) fn all_files(root: &Path) -> Result<Vec<PathBuf>> {
    walk(root, true)
}

fn walk(root: &Path, hidden: bool) -> Result<Vec<PathBuf>> {
    let entries = WalkDir::new(root)
        .follow_links(false)
        .follow_root_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0 || hidden || !entry.file_name().to_string_lossy().starts_with('.')
        });
    let mut paths = Vec::new();
    for entry in entries {
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
        if kind.is_file() {
            paths.push(entry.into_path());
        }
    }
    paths.sort_unstable();
    Ok(paths)
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
