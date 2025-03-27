use notion_api::{Config, NotionClient};
use std::error::Error;
use std::fs;
use std::path::Path;

fn load_test_config() -> Result<Config, Box<dyn Error>> {
    // notion_apiクレートのルートディレクトリのconfig.jsonを読み込む
    let config_path = Path::new("config.json");
    if !config_path.exists() {
        return Err("config.json not found.".into());
    }
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

#[tokio::test]
async fn integration_test_real_api() -> Result<(), Box<dyn Error>> {
    let config = load_test_config()?;
    let notion_client = NotionClient::new(config);

    // main.rsと同様の処理を行う
    let pages = notion_client.query_database().await?;
    println!("Retrieved {} pages from Notion API.", pages.len());
    for page in pages {
        println!("Processing page: {}", page.title);
        notion_client.query_page(&page).await?;
    }
    Ok(())
}
