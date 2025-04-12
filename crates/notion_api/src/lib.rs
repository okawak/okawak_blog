pub mod client;
pub mod config;
pub mod database;
pub mod markdown;
pub mod models;

pub use client::NotionClient;
pub use config::{Config, load_config};
pub use markdown::to_markdown;

use std::error::Error;
use std::fs;
use std::path::Path;

pub async fn run_main(config: Config) -> Result<(), Box<dyn Error>> {
    // NotionClientを生成（内部でHTTPクライアントを初期化）
    let notion_client = NotionClient::new(config);

    // データベースクエリを実行して全ページ情報を取得
    let pages = notion_client.query_database().await?;
    println!("Retrieved {} pages from Notion API.", pages.len());
    // 各ページの子ブロックを取得してファイルに出力
    for page in pages {
        println!("Processing page: {}", page.title);
        let blocks = notion_client.query_page(&page).await?;
        let markdown_str = to_markdown(&page, &blocks)?;

        // ファイル出力
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let file_path = format!("{}/dist/{}/{}.md", manifest_dir, page.category, page.id);
        if let Some(parent) = Path::new(&file_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, markdown_str)?;
    }
    Ok(())
}
