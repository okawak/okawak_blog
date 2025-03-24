use reqwest::Client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = notion_api::load_config()?;
    let client = Client::new();

    // データベースクエリを実行、全てのページ情報を取得
    // filterやsort条件はハードコードしている
    let all_pages = notion_api::query_database(&client, &config).await?;
    // 各ページ情報を取得
    for page in all_pages {
        page.query_page(&client, &config).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use notion_api::Config;
    use std::fs;

    fn load_config() -> Result<Config, Box<dyn Error>> {
        let config_str = fs::read_to_string("config.json")?;
        let config: Config = serde_json::from_str(&config_str)?;
        Ok(config)
    }

    #[tokio::test]
    async fn test_main() -> Result<(), Box<dyn Error>> {
        let config = load_config().unwrap();
        let client = Client::new();

        let all_pages = notion_api::query_database(&client, &config).await?;
        for page in all_pages {
            page.query_page(&client, &config).await?;
        }
        Ok(())
    }
    /// GitHub Actionsでの変数受け取りテスト
    /// cargo test --test test_github_env -p notion_api
    #[test]
    fn test_github_env() {
        let config = notion_api::load_config();
        assert!(config.is_ok());
    }
}
