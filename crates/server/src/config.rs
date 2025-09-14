//! Configuration - アプリケーション設定管理

use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub aws: AwsConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AwsConfig {
    pub region: String,
    pub s3_bucket: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Config {
    pub fn load() -> Result<Self, anyhow::Error> {
        // 環境変数から設定を読み込み
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .unwrap_or(3000);

        let aws_region = env::var("AWS_REGION").unwrap_or_else(|_| "ap-northeast-1".to_string());
        let s3_bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "okawak-blog-content".to_string());

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/blog_dev".to_string());

        Ok(Config {
            server: ServerConfig { host, port },
            aws: AwsConfig {
                region: aws_region,
                s3_bucket,
            },
            database: DatabaseConfig { url: database_url },
        })
    }
}
