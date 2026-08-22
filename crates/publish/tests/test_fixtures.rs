#![allow(dead_code, reason = "shared by integration test crates")]

use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn collect_html_files(root: &Path) -> Vec<PathBuf> {
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

pub(crate) fn write_about_page(obsidian_dir: &Path) {
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

pub(crate) fn write_tech_category_landing(obsidian_dir: &Path) {
    let category_dir = obsidian_dir.join("tech");
    fs::create_dir_all(&category_dir).unwrap();
    fs::write(
        category_dir.join("category.md"),
        r#"---
title: "Tech"
kind: category
category: tech
summary: "Technology articles"
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00"
is_completed: true
---

# Tech

Technology articles.
"#,
    )
    .unwrap();
}
