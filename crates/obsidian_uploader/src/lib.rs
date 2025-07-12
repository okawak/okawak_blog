pub mod config;
pub mod converter;
pub mod error;
pub mod models;
pub mod parser;
pub mod scanner;
pub mod slug;

pub use config::Config;
pub use error::{ObsidianError, Result};
pub use models::{ObsidianFrontMatter, OutputFrontMatter};

use converter::{FileInfo, FileMapping};
use std::collections::HashMap;
use std::fs;

pub async fn run_main(config: Config) -> Result<()> {
    fs::create_dir_all(&config.output_dir)?;

    let markdown_files = scanner::scan_obsidian_files(&config.obsidian_dir)?;
    println!("Found {} markdown files", markdown_files.len());

    let valid_files: Vec<_> = markdown_files
        .into_iter()
        .filter_map(|file_path| match parser::parse_obsidian_file(&file_path) {
            Ok(Some(front_matter)) if front_matter.is_completed => Some((file_path, front_matter)),
            Ok(_) => None,
            Err(e) => {
                eprintln!("Error processing {}: {}", file_path.display(), e);
                None
            }
        })
        .collect();

    let file_mapping = build_file_mapping(&config, &valid_files)?;

    let processed_files: Result<Vec<_>> = valid_files
        .into_iter()
        .map(|(file_path, front_matter)| {
            process_obsidian_file(&config, file_path, front_matter, &file_mapping)
        })
        .collect();

    let processed_files = processed_files?;
    let processed_count = processed_files.len();

    println!("Successfully processed {} files", processed_count);
    Ok(())
}

fn build_file_mapping(
    config: &Config,
    valid_files: &[(std::path::PathBuf, ObsidianFrontMatter)],
) -> Result<FileMapping> {
    let mut mapping = HashMap::with_capacity(valid_files.len());

    for (file_path, front_matter) in valid_files {
        let relative_path = file_path.strip_prefix(&config.obsidian_dir).map_err(|_| {
            ObsidianError::PathError(format!(
                "Failed to strip prefix from {}",
                file_path.display()
            ))
        })?;

        let slug = slug::generate_slug(&front_matter.title, relative_path, &front_matter.created)?;

        // URL正規化: OS固有のパスセパレータをUnix形式に統一
        let html_path = {
            let html_path_buf = relative_path.with_extension("html");
            let path_str = html_path_buf.to_string_lossy();
            let normalized_path = path_str.replace(std::path::MAIN_SEPARATOR, "/");
            format!("/{}", normalized_path)
        };

        // マッピングキーを相対パス全体に変更（衝突を回避）
        let mapping_key = {
            let key_path_buf = relative_path.with_extension("");
            let path_str = key_path_buf.to_string_lossy();
            path_str.replace(std::path::MAIN_SEPARATOR, "/")
        };

        let file_info = FileInfo {
            relative_path: {
                let rel_path_buf = relative_path.with_extension("");
                let path_str = rel_path_buf.to_string_lossy();
                path_str.replace(std::path::MAIN_SEPARATOR, "/")
            },
            slug,
            html_path,
        };

        mapping.insert(mapping_key, file_info);
    }

    Ok(mapping)
}

fn process_obsidian_file(
    config: &Config,
    file_path: std::path::PathBuf,
    front_matter: ObsidianFrontMatter,
    file_mapping: &FileMapping,
) -> Result<OutputFrontMatter> {
    let markdown_content = fs::read_to_string(&file_path)?;
    let markdown_body = extract_markdown_body(&markdown_content);
    let markdown_with_links = converter::convert_obsidian_links(&markdown_body, file_mapping);
    let html_body = converter::convert_markdown_to_html(&markdown_with_links)?;

    let relative_path = file_path.strip_prefix(&config.obsidian_dir).map_err(|_| {
        ObsidianError::PathError(format!(
            "Failed to strip prefix from {}",
            file_path.display()
        ))
    })?;

    // file_mappingからslug情報を取得（重複処理を回避）
    let mapping_key = {
        let key_path_buf = relative_path.with_extension("");
        let path_str = key_path_buf.to_string_lossy();
        path_str.replace(std::path::MAIN_SEPARATOR, "/")
    };
    
    let slug = file_mapping
        .get(&mapping_key)
        .map(|file_info| file_info.slug.clone())
        .ok_or_else(|| {
            ObsidianError::PathError(format!("File mapping not found for: {}", mapping_key))
        })?;

    let output_fm = OutputFrontMatter {
        title: front_matter.title,
        tags: front_matter.tags,
        description: front_matter.summary,
        priority: front_matter.priority,
        created: front_matter.created,
        updated: front_matter.updated,
        slug: slug.clone(),
    };

    let output_yaml = serde_yaml::to_string(&output_fm).map_err(ObsidianError::YamlError)?;
    let html_file_content = converter::generate_html_file(&output_yaml, &html_body);
    let output_file_path = config.output_dir.join(relative_path.with_extension("html"));

    if let Some(parent) = output_file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_file_path, html_file_content)?;

    println!(
        "Processed: {} -> {} ({})",
        file_path.display(),
        output_file_path.display(),
        slug
    );

    Ok(output_fm)
}

fn extract_markdown_body(content: &str) -> String {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return content.to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    let end_pos = lines.iter().skip(1).position(|&line| line.trim() == "---");

    match end_pos {
        Some(pos) => {
            // フロントマター終了位置の次の行から残りを取得
            let body_lines = &lines[pos + 2..];
            body_lines.join("\n")
        }
        None => content.to_string(), // フロントマターが正しく終了していない場合は全体を返す
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[rstest]
    #[case::with_frontmatter(
        "---\ntitle: Test\n---\n# Content\n\nBody text",
        "# Content\n\nBody text"
    )]
    #[case::no_frontmatter("# Content\n\nBody text", "# Content\n\nBody text")]
    #[case::malformed_frontmatter(
        "---\ntitle: Test\n# Content\n\nBody text",
        "---\ntitle: Test\n# Content\n\nBody text"
    )]
    #[case::empty_body("---\ntitle: Test\n---\n", "")]
    #[case::whitespace_handling("   ---\ntitle: Test\n---\n\n# Content", "\n# Content")]
    #[case::multiple_frontmatter_separators(
        "---\ntitle: Test\n---\n# Section\n---\nMore content",
        "# Section\n---\nMore content"
    )]
    #[case::frontmatter_with_complex_yaml(
        "---\ntitle: \"Complex: Title\"\ntags: [\"tag1\", \"tag2\"]\n---\n## Heading",
        "## Heading"
    )]
    fn test_extract_markdown_body(#[case] input: &str, #[case] expected: &str) {
        let result = extract_markdown_body(input);
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_build_file_mapping_success() {
        use crate::models::ObsidianFrontMatter;

        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        let file_path = temp_dir.path().join("test.md");
        let front_matter = ObsidianFrontMatter {
            title: "Test Article".to_string(),
            tags: Some(vec!["test".to_string()]),
            summary: Some("Test summary".to_string()),
            priority: Some(1),
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-02T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("tech".to_string()),
        };

        let valid_files = vec![(file_path, front_matter)];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.len(), 1);
        assert!(mapping.contains_key("test")); // ファイル名がキーとなる
    }

    #[rstest]
    fn test_build_file_mapping_empty() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        let valid_files = vec![];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.len(), 0);
    }

    #[rstest]
    fn test_build_file_mapping_path_collision() {
        use crate::models::ObsidianFrontMatter;

        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        // 同じファイル名だが異なるディレクトリのファイル
        let file_path1 = temp_dir.path().join("dir1").join("test.md");
        let file_path2 = temp_dir.path().join("dir2").join("test.md");
        
        let front_matter1 = ObsidianFrontMatter {
            title: "Test Article 1".to_string(),
            tags: Some(vec!["test1".to_string()]),
            summary: Some("Test summary 1".to_string()),
            priority: Some(1),
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-02T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("tech".to_string()),
        };

        let front_matter2 = ObsidianFrontMatter {
            title: "Test Article 2".to_string(),
            tags: Some(vec!["test2".to_string()]),
            summary: Some("Test summary 2".to_string()),
            priority: Some(2),
            created: "2025-01-03T00:00:00+09:00".to_string(),
            updated: "2025-01-04T00:00:00+09:00".to_string(),
            is_completed: true,
            category: Some("blog".to_string()),
        };

        let valid_files = vec![(file_path1, front_matter1), (file_path2, front_matter2)];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        // 相対パス全体をキーとするため、衝突せずに2つのエントリが存在
        assert_eq!(mapping.len(), 2);
        assert!(mapping.contains_key("dir1/test"));
        assert!(mapping.contains_key("dir2/test"));
    }

    #[rstest]
    fn test_build_file_mapping_url_normalization() {
        use crate::models::ObsidianFrontMatter;

        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            obsidian_dir: temp_dir.path().to_path_buf(),
            output_dir: PathBuf::from("output"),
        };

        let file_path = temp_dir.path().join("sub").join("dir").join("test.md");
        let front_matter = ObsidianFrontMatter {
            title: "URL Test".to_string(),
            tags: None,
            summary: None,
            priority: None,
            created: "2025-01-01T00:00:00+09:00".to_string(),
            updated: "2025-01-01T00:00:00+09:00".to_string(),
            is_completed: true,
            category: None,
        };

        let valid_files = vec![(file_path, front_matter)];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        let file_info = mapping.get("sub/dir/test").unwrap();
        
        // URL正規化が適用されているかチェック（Unix形式のスラッシュ）
        assert_eq!(file_info.html_path, "/sub/dir/test.html");
        assert_eq!(file_info.relative_path, "sub/dir/test");
    }
}
