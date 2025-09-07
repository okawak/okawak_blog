pub mod bookmark;
pub mod config;
pub mod converter;
pub mod error;
pub mod models;
pub mod parser;
pub mod scanner;
pub mod slug;

pub use config::Config;
pub use error::{ObsidianError, Result};
pub use models::OutputFrontMatter;
pub use parser::ObsidianFrontMatter;

use converter::FileMapping;
use futures::{StreamExt, TryStreamExt, stream};
use log::{error, info, warn};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// パース済みのファイル情報を保持する構造体
struct ParsedFile {
    file_path: PathBuf,
    slug: String,
    content: String,
    front_matter: ObsidianFrontMatter,
}

pub async fn run_main(config: &Config) -> Result<()> {
    let start_time = std::time::Instant::now();
    info!("=== Obsidian Uploader Started ===");
    info!("Input directory: {}", config.obsidian_dir.display());
    info!("Output directory: {}", config.output_dir.display());

    fs::create_dir_all(&config.output_dir)?;

    let markdown_files = scanner::scan_obsidian_files(&config.obsidian_dir)?;
    info!("Found {} markdown files", markdown_files.len());

    let mut skipped_count = 0;
    let mut error_count = 0;

    let valid_files: Vec<ParsedFile> = markdown_files
        .into_iter()
        .filter_map(|file_path| match parser::parse_obsidian_file(&file_path) {
            Ok(Some((front_matter, content))) if front_matter.is_completed => {
                let relative_path = get_relative_path(&file_path, &config.obsidian_dir).ok()?;
                let slug =
                    slug::generate_slug(&front_matter.title, relative_path, &front_matter.created)
                        .ok()?;
                Some(ParsedFile {
                    file_path,
                    slug,
                    content,
                    front_matter,
                })
            }
            Ok(_) => {
                skipped_count += 1;
                warn!("Skipped (not completed): {}", file_path.display());
                None
            }
            Err(e) => {
                error_count += 1;
                error!("Error processing {}: {}", file_path.display(), e);
                None
            }
        })
        .collect();

    info!("Valid files: {}", valid_files.len());
    info!("Skipped files: {skipped_count}");
    if error_count > 0 {
        warn!("Error files: {error_count}");
    }

    let file_mapping = build_file_mapping(config, &valid_files)?;

    const CONCURRENT_LIMIT: usize = 4;
    let processed_files: Vec<OutputFrontMatter> = stream::iter(valid_files)
        .map(|parsed_file| process_parsed_file(config, parsed_file, &file_mapping))
        .buffer_unordered(CONCURRENT_LIMIT)
        .try_collect()
        .await?;

    let processed_count = processed_files.len();
    let duration = start_time.elapsed();

    // 処理結果サマリーの出力
    info!("=== Processing Summary ===");
    info!("Successfully processed: {processed_count} files");
    info!("  Skipped: {skipped_count} files");
    if error_count > 0 {
        warn!("  Errors: {error_count} files");
    }
    info!("  Processing time: {duration:.2?}");
    info!("Output directory: {}", config.output_dir.display());

    // 処理されたファイルの詳細
    if !processed_files.is_empty() {
        info!("Processed files:");
        for file in &processed_files {
            info!("  • {} ({})", file.title, file.slug);
        }
    }

    info!("=== Obsidian Uploader Completed ===");
    Ok(())
}

/// パスをURL用に正規化（OS固有セパレータをUnix形式に統一）
fn normalize_path_for_url(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    path_str.replace(std::path::MAIN_SEPARATOR, "/")
}

/// 相対パスを取得する共通処理
fn get_relative_path<'a>(file_path: &'a Path, base_dir: &Path) -> Result<&'a Path> {
    file_path.strip_prefix(base_dir).map_err(|_| {
        ObsidianError::Path(format!(
            "Failed to strip prefix from {}",
            file_path.display()
        ))
    })
}

fn build_file_mapping(config: &Config, valid_files: &[ParsedFile]) -> Result<FileMapping> {
    let mut mapping = FileMapping::with_capacity(valid_files.len());

    for parsed_file in valid_files {
        let relative_path = get_relative_path(&parsed_file.file_path, &config.obsidian_dir)?;
        let relative_path_no_ext = relative_path.with_extension("");
        let mapping_key = normalize_path_for_url(&relative_path_no_ext);
        mapping.insert(mapping_key, parsed_file.slug.clone());
    }

    Ok(mapping)
}

async fn process_parsed_file(
    config: &Config,
    parsed_file: ParsedFile,
    file_mapping: &FileMapping,
) -> Result<OutputFrontMatter> {
    let markdown_body = extract_markdown_body(&parsed_file.content);
    let markdown_with_links = converter::convert_obsidian_links(&markdown_body, file_mapping);
    let html_body = converter::convert_markdown_to_html(&markdown_with_links)?;

    // HTMLを生成後、シンプルなbookmarkをリッチブックマークに変換
    let html_with_rich_bookmarks = bookmark::convert_simple_bookmarks_to_rich(&html_body)
        .await
        .unwrap_or_else(|e| {
            warn!("Warning: Failed to convert simple bookmarks to rich bookmarks: {e}");
            html_body
        });

    let relative_path = get_relative_path(&parsed_file.file_path, &config.obsidian_dir)?;

    let output_fm = OutputFrontMatter {
        title: parsed_file.front_matter.title,
        tags: parsed_file.front_matter.tags,
        description: parsed_file.front_matter.summary,
        priority: parsed_file.front_matter.priority,
        created: parsed_file.front_matter.created,
        updated: parsed_file.front_matter.updated,
        slug: parsed_file.slug.clone(),
    };

    let output_yaml = serde_yaml::to_string(&output_fm).map_err(ObsidianError::Yaml)?;
    let html_file_content = converter::generate_html_file(&output_yaml, &html_with_rich_bookmarks);
    let output_file_path = config
        .output_dir
        .join(relative_path.with_file_name(format!("{}.html", parsed_file.slug)));

    if let Some(parent) = output_file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_file_path, html_file_content)?;

    info!("...processed {}", output_file_path.display());

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

        let parsed_file = ParsedFile {
            file_path,
            slug: "slug".to_string(),
            content: "---\ntitle: Test Article\n---\n# Test Content".to_string(),
            front_matter,
        };
        let valid_files = vec![parsed_file];
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

        let valid_files: Vec<ParsedFile> = vec![];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        assert_eq!(mapping.len(), 0);
    }

    #[rstest]
    fn test_build_file_mapping_path_collision() {
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

        let parsed_file1 = ParsedFile {
            file_path: file_path1,
            slug: "slug1".to_string(),
            content: "---\ntitle: Test Article 1\n---\n# Test Content 1".to_string(),
            front_matter: front_matter1,
        };
        let parsed_file2 = ParsedFile {
            file_path: file_path2,
            slug: "slug2".to_string(),
            content: "---\ntitle: Test Article 2\n---\n# Test Content 2".to_string(),
            front_matter: front_matter2,
        };
        let valid_files = vec![parsed_file1, parsed_file2];
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

        let parsed_file = ParsedFile {
            file_path,
            slug: "slug".to_string(),
            content: "---\ntitle: URL Test\n---\n# URL Test Content".to_string(),
            front_matter,
        };
        let valid_files = vec![parsed_file];
        let result = build_file_mapping(&config, &valid_files);

        assert!(result.is_ok());
        let mapping = result.unwrap();
        let slug = mapping.get("sub/dir/test").unwrap();

        // URL正規化が適用されているかチェック（Unix形式のスラッシュ）
        // slugが存在することを確認
        assert!(!slug.is_empty());
    }
}
