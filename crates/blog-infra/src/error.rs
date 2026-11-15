use aws_sdk_s3::{error::SdkError, operation::get_object::GetObjectError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfraError {
    /// One in-flight storage failure shared by concurrent readers.
    #[error(transparent)]
    Shared(std::sync::Arc<InfraError>),
    #[error("invalid artifact contract: {0}")]
    Domain(#[from] blog_domain::DomainError),
    #[error("failed to read artifact file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode artifact json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("missing artifact configuration: {0}")]
    MissingConfig(&'static str),
    #[error("invalid artifact configuration {key}: {value}")]
    InvalidConfig { key: &'static str, value: String },
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

    pub fn is_not_found(&self) -> bool {
        match self {
            Self::Shared(error) => error.is_not_found(),
            Self::Io(error) => error.kind() == std::io::ErrorKind::NotFound,
            Self::S3Read { source, .. } => source
                .downcast_ref::<SdkError<GetObjectError>>()
                .is_some_and(|error| {
                    error
                        .as_service_error()
                        .is_some_and(GetObjectError::is_no_such_key)
                        || error
                            .raw_response()
                            .is_some_and(|response| response.status().as_u16() == 404)
                }),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InfraError;

    #[test]
    fn not_found_messages_and_other_failures_are_distinguished() {
        for message in ["NoSuchKey", "Not Found", "not found", "404"] {
            let error = InfraError::s3_read("bucket", "key", std::io::Error::other(message));
            assert!(!error.is_not_found(), "{message}");
        }
        for error in [
            InfraError::Io(std::io::Error::other("disk failure")),
            InfraError::MissingConfig("bucket"),
            InfraError::s3_read("bucket", "key", std::io::Error::other("AccessDenied")),
            InfraError::Shared(std::sync::Arc::new(InfraError::Io(std::io::Error::other(
                "temporary failure",
            )))),
        ] {
            assert!(!error.is_not_found(), "{error}");
        }
    }

    #[test]
    fn shared_errors_preserve_not_found_classification_and_message() {
        let original = InfraError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing object",
        ));
        let message = original.to_string();
        let shared = InfraError::Shared(std::sync::Arc::new(original));
        assert!(shared.is_not_found());
        assert_eq!(shared.to_string(), message);
    }
}
