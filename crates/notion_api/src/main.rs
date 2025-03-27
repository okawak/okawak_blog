use notion_api::{NotionClient, load_config, to_markdown};
use std::error::Error;
use std::fs;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 環境変数から設定情報を読み込む
    let config = load_config()?;
    // NotionClientを生成（内部でHTTPクライアントを初期化）
    let notion_client = NotionClient::new(config);

    // データベースクエリを実行して全ページ情報を取得
    let pages = notion_client.query_database().await?;
    // 各ページの子ブロックを取得してファイルに出力
    for page in pages {
        let blocks = notion_client.query_page(&page).await?;
        let markdown_str = to_markdown(&page, &blocks)?;

        // ファイル出力
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let file_path = format!("{}/dest/{}.md", manifest_dir, page.id);
        if let Some(parent) = Path::new(&file_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, markdown_str)?;
    }
    Ok(())
}
