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

pub fn write_about_page(obsidian_dir: &Path) {
    fs::write(
        obsidian_dir.join("about.md"),
        r#"---
title: "About"
kind: page
page: about
summary: "About this site"
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00"
is_completed: true
---

# About

This page is required for deployment.
"#,
    )
    .unwrap();
}
