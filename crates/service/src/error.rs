use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("Article not found: {id}")]
    ArticleNotFound { id: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("AWS SDK error: {0}")]
    AwsError(String),

    #[error("Domain error: {0}")]
    DomainError(#[from] domain::error::CoreError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<aws_sdk_s3::Error> for ServiceError {
    fn from(err: aws_sdk_s3::Error) -> Self {
        ServiceError::AwsError(err.to_string())
    }
}

impl<E> From<aws_sdk_s3::error::SdkError<E>> for ServiceError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: aws_sdk_s3::error::SdkError<E>) -> Self {
        ServiceError::AwsError(err.to_string())
    }
}
