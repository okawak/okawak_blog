use notion_api::{NotionClient, load_config};
use std::error::Error;

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
        notion_client.query_page(&page).await?;
    }
    Ok(())
}
