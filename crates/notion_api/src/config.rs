use crate::error::Result;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub notion_token: String,
    pub database_id: String,
}

pub fn load_config() -> Result<Config> {
    let notion_token = env::var("NOTION_TOKEN")?;
    let database_id = env::var("DATABASE_ID")?;
    Ok(Config {
        notion_token,
        database_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_success() {
        // テスト用に環境変数を一時的に設定
        unsafe {
            env::set_var("NOTION_TOKEN", "test_token");
            env::set_var("DATABASE_ID", "test_db");
        }
        let config = load_config().unwrap();
        assert_eq!(config.notion_token, "test_token");
        assert_eq!(config.database_id, "test_db");
    }

    #[test]
    fn test_load_config_failure() {
        // 環境変数をクリアしてエラーとなることを確認
        unsafe {
            env::remove_var("NOTION_TOKEN");
            env::remove_var("DATABASE_ID");
        }
        let config = load_config();
        assert!(config.is_err());
    }
}
