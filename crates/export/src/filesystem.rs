//! Filesystem traversal, locking and staged replacement, independent of content schemas.
use crate::{ExportError, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn locked<T>(output: &Path, build: impl FnOnce() -> Result<T>) -> Result<T> {
    let parent = parent(output);
    fs::create_dir_all(parent)?;
    let name = output
        .file_name()
        .ok_or_else(|| ExportError::invalid_input("output needs a directory name"))?
        .to_string_lossy();
    let backup = parent.join(format!(".{name}.export-backup"));
    let lock = parent.join(format!(".{name}.export-lock"));
    let lock_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)?;
    struct Lock<'a>(&'a Path, fs::File);
    impl Drop for Lock<'_> {
        fn drop(&mut self) {
            let _ = &self.1;
            let _ = fs::remove_file(self.0);
        }
    }
    let _lock = Lock(&lock, lock_file);
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
        if output.exists() && same_tree(output, stage.path())? {
            return Ok(result);
        }
        let existed = output.exists();
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
    for locale in ["ja", "en", "assets"] {
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
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(ExportError::invalid_input("symlink root is not allowed"));
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !hidden && entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(ExportError::invalid_input(format!(
                "symlink input is not allowed: {}",
                entry.path().display()
            )));
        }
        if kind.is_dir() {
            paths.extend(walk(&entry.path(), hidden)?);
        } else if kind.is_file() {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub(crate) fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}
