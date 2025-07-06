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

    // is_completed: true のファイルのみフィルタリング
    let mut processed_count = 0;
    for file_path in markdown_files {
        match parser::parse_obsidian_file(&file_path) {
            Ok(Some(front_matter)) => {
                if front_matter.is_completed {
                    // Slug生成
                    let relative_path =
                        file_path.strip_prefix(&config.obsidian_dir).map_err(|_| {
                            ObsidianError::PathError(format!(
                                "Failed to strip prefix from {}",
                                file_path.display()
                            ))
                        })?;

                    let slug = slug::generate_slug(
                        &front_matter.title,
                        relative_path,
                        &front_matter.created,
                    )?;

                    // 出力用フロントマターに変換
                    let output_fm = OutputFrontMatter {
                        title: front_matter.title,
                        tags: front_matter.tags,
                        description: front_matter.summary,
                        priority: front_matter.priority,
                        created: front_matter.created,
                        updated: front_matter.updated,
                        slug,
                    };

                    println!("Processed: {} -> {}", file_path.display(), output_fm.slug);
                    processed_count += 1;
                }
            }
            Ok(None) => {
                // フロントマターが存在しないファイル
                continue;
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", file_path.display(), e);
                continue;
            }
        }
    }

    println!("Successfully processed {} files", processed_count);
    Ok(())
}
