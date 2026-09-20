//! Stage an entire public tree before replacing it. A recoverable backup survives interruption.
use crate::{ExportError, Result};
use std::{fs, path::Path};

pub(crate) fn locked<T>(output: &Path, build: impl FnOnce() -> Result<T>) -> Result<T> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
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

pub(crate) fn transaction(output: &Path, build: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    locked(output, || {
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
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
        build(stage.path())?;
        if output.exists() && same_tree(output, stage.path())? {
            return Ok(());
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
        Ok(())
    })
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    for path in crate::markdown::all_files(from)? {
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
    let aa = crate::markdown::all_files(a)?;
    let bb = crate::markdown::all_files(b)?;
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
