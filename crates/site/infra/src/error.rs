use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("failed to read artifact file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode artifact json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to decode artifact utf-8 content: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("missing artifact configuration: {0}")]
    MissingConfig(&'static str),
    #[error("unsupported artifact source: {0}")]
    UnsupportedSource(String),
    #[error("failed to read s3 object s3://{bucket}/{key}: {source}")]
    S3Read {
        bucket: String,
        key: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub type Result<T> = std::result::Result<T, InfraError>;

impl InfraError {
    pub fn s3_read(
        bucket: impl Into<String>,
        key: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::S3Read {
            bucket: bucket.into(),
            key: key.into(),
            source: Box::new(source),
        }
    }
}
