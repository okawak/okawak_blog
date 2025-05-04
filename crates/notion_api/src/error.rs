use thiserror::Error;

/// Notion API関連のエラー型
#[derive(Debug, Error)]
pub enum NotionError {
    #[error("API request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("database error: {0}")]
    DatabaseError(String),

    #[error("page error: {0}")]
    PageError(String),

    #[error("markdown conversion error: {0}")]
    MarkdownError(String),

    #[error("Could not read configs from environment variables: {0}")]
    ConfigError(#[from] std::env::VarError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("data format error: {0}")]
    DataError(String),

    #[error("other error: {0}")]
    Other(String),
}

/// Result型のエイリアス
pub type Result<T> = std::result::Result<T, NotionError>;
