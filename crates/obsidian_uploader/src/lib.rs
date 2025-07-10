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

use std::fs;

/// メイン実行関数
pub async fn run_main(config: Config) -> Result<()> {
    // 出力ディレクトリの作成
    fs::create_dir_all(&config.output_dir)?;

    // Obsidianファイルをスキャン
    let markdown_files = scanner::scan_obsidian_files(&config.obsidian_dir)?;
    println!("Found {} markdown files", markdown_files.len());

    // is_completed: true のファイルのみフィルタリングして処理
    let processed_files: Result<Vec<_>> = markdown_files
        .into_iter()
        .filter_map(|file_path| {
            match parser::parse_obsidian_file(&file_path) {
                Ok(Some(front_matter)) if front_matter.is_completed => {
                    Some(Ok((file_path, front_matter)))
                }
                Ok(_) => None, // フロントマターなし、またはis_completed: false
                Err(e) => {
                    eprintln!("Error processing {}: {}", file_path.display(), e);
                    None
                }
            }
        })
        .map(|result| {
            result.and_then(|(file_path, front_matter)| {
                let markdown_content = fs::read_to_string(&file_path)?;
                let markdown_body = extract_markdown_body(&markdown_content);
                let markdown_with_links = converter::convert_obsidian_links(&markdown_body);
                let html_body = converter::convert_markdown_to_html(&markdown_with_links)?;
                let relative_path = file_path.strip_prefix(&config.obsidian_dir).map_err(|_| {
                    ObsidianError::PathError(format!(
                        "Failed to strip prefix from {}",
                        file_path.display()
                    ))
                })?;

                let slug =
                    slug::generate_slug(&front_matter.title, relative_path, &front_matter.created)?;

                let output_fm = OutputFrontMatter {
                    title: front_matter.title,
                    tags: front_matter.tags,
                    description: front_matter.summary,
                    priority: front_matter.priority,
                    created: front_matter.created,
                    updated: front_matter.updated,
                    slug: slug.clone(),
                };

                let output_yaml =
                    serde_yaml::to_string(&output_fm).map_err(|e| ObsidianError::YamlError(e))?;
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
            })
        })
        .collect();

    let processed_files = processed_files?;
    let processed_count = processed_files.len();

    println!("Successfully processed {} files", processed_count);
    Ok(())
}

/// Markdownファイルの内容からYAMLフロントマターを除去してボディ部分を抽出
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

    #[rstest]
    #[case::with_frontmatter(
        "---\ntitle: Test\n---\n# Content\n\nBody text",
        "# Content\n\nBody text"
    )]
    #[case::no_frontmatter(
        "# Content\n\nBody text",
        "# Content\n\nBody text"
    )]
    #[case::malformed_frontmatter(
        "---\ntitle: Test\n# Content\n\nBody text",
        "---\ntitle: Test\n# Content\n\nBody text"
    )]
    #[case::empty_body(
        "---\ntitle: Test\n---\n",
        ""
    )]
    #[case::whitespace_handling(
        "   ---\ntitle: Test\n---\n\n# Content",
        "\n# Content"
    )]
    fn test_extract_markdown_body(#[case] input: &str, #[case] expected: &str) {
        let result = extract_markdown_body(input);
        assert_eq!(result, expected);
    }
}
