use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn collect_html_files(root: &Path) -> Vec<PathBuf> {
    let mut html_files = Vec::new();

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                html_files.extend(collect_html_files(&path));
            } else if path.extension().is_some_and(|ext| ext == "html") {
                html_files.push(path);
            }
        }
    }

    html_files
}
