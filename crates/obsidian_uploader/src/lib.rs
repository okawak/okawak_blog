pub mod config;
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
                Ok(Some(front_matter)) if front_matter.is_completed => Some(Ok((file_path, front_matter))),
                Ok(_) => None, // フロントマターなし、またはis_completed: false
                Err(e) => {
                    eprintln!("Error processing {}: {}", file_path.display(), e);
                    None
                }
            }
        })
        .map(|result| {
            result.and_then(|(file_path, front_matter)| {
                let relative_path = file_path.strip_prefix(&config.obsidian_dir)
                    .map_err(|_| ObsidianError::PathError(format!(
                        "Failed to strip prefix from {}",
                        file_path.display()
                    )))?;

                let slug = slug::generate_slug(
                    &front_matter.title,
                    relative_path,
                    &front_matter.created,
                )?;

                let output_fm = OutputFrontMatter {
                    title: front_matter.title,
                    tags: front_matter.tags,
                    description: front_matter.summary,
                    priority: front_matter.priority,
                    created: front_matter.created,
                    updated: front_matter.updated,
                    slug: slug.clone(),
                };

                println!("Processed: {} -> {}", file_path.display(), slug);
                Ok(output_fm)
            })
        })
        .collect();

    let processed_files = processed_files?;
    let processed_count = processed_files.len();

    println!("Successfully processed {} files", processed_count);
    Ok(())
}
