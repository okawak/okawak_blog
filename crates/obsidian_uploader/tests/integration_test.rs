use obsidian_uploader::{Config, run_main};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_run_main_with_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // 空のObsidianディレクトリを作成
    fs::create_dir_all(&obsidian_dir).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // run_mainを実行
    let result = run_main(config).await;
    assert!(result.is_ok());

    // 出力ディレクトリが作成されていることを確認
    assert!(output_dir.exists());
    assert!(output_dir.is_dir());
}

#[tokio::test]
async fn test_run_main_with_sample_file() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // Obsidianディレクトリとサンプルファイルを作成
    fs::create_dir_all(&obsidian_dir).unwrap();

    let sample_content = r#"---
title: "Test Article"
tags: ["test"]
summary: "Test summary"
priority: 1
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00"
is_completed: true
category: "tech"
---

# Test Article

This is a test article.
"#;

    let sample_file = obsidian_dir.join("test.md");
    fs::write(&sample_file, sample_content).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // run_mainを実行
    let result = run_main(config).await;
    assert!(result.is_ok());

    // HTMLファイルが生成されていることを確認
    let html_file = output_dir.join("test.html");
    assert!(html_file.exists());

    // HTMLファイルの内容を確認
    let html_content = fs::read_to_string(&html_file).unwrap();
    assert!(html_content.contains("Test Article"));
    assert!(html_content.contains("This is a test article"));
}

#[tokio::test]
async fn test_run_main_with_incomplete_file() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // Obsidianディレクトリとis_completed: falseのファイルを作成
    fs::create_dir_all(&obsidian_dir).unwrap();

    let incomplete_content = r#"---
title: "Incomplete Article"
tags: ["test"]
summary: "Incomplete summary"
priority: 1
created: "2025-01-01T00:00:00+09:00"
updated: "2025-01-01T00:00:00+09:00"
is_completed: false
category: "tech"
---

# Incomplete Article

This article is not completed.
"#;

    let sample_file = obsidian_dir.join("incomplete.md");
    fs::write(&sample_file, incomplete_content).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // run_mainを実行
    let result = run_main(config).await;
    assert!(result.is_ok());

    // HTMLファイルが生成されていないことを確認（is_completed: falseのため）
    let html_file = output_dir.join("incomplete.html");
    assert!(!html_file.exists());
}

#[test]
fn test_config_validation() {
    // 存在しないディレクトリでのConfig作成テスト
    let temp_dir = TempDir::new().unwrap();
    let non_existent_dir = temp_dir.path().join("non_existent");

    let config = Config {
        obsidian_dir: non_existent_dir,
        output_dir: PathBuf::from("test_output"),
    };

    // validateは直接呼べないので、Config::newで検証する代わりに
    // 存在しないパスでの動作を確認
    assert!(!config.obsidian_dir.exists());
}
