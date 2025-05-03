use notion_api::{Config, Result, run_main};
use std::fs;
use std::path::Path;

fn load_test_config() -> Result<Config> {
    // notion_apiクレートのルートディレクトリのconfig.jsonを読み込む
    let config_path = Path::new("config.json");
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = serde_json::from_str(&config_str)?;
    Ok(config)
}

#[tokio::test]
async fn integration_test_real_api() -> anyhow::Result<()> {
    let config = load_test_config()?;
    Ok(run_main(config).await?)
}
