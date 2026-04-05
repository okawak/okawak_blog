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

    pub fn is_not_found(&self) -> bool {
        match self {
            Self::Io(error) => error.kind() == std::io::ErrorKind::NotFound,
            Self::S3Read { source, .. } => Self::error_is_not_found(source.as_ref()),
            _ => false,
        }
    }

    fn error_is_not_found(error: &(dyn std::error::Error + 'static)) -> bool {
        if let Some(io_error) = error.downcast_ref::<std::io::Error>()
            && io_error.kind() == std::io::ErrorKind::NotFound
        {
            return true;
        }

        let message = error.to_string();
        if message.contains("NoSuchKey")
            || message.contains("Not Found")
            || message.contains("not found")
            || message.contains("404")
        {
            return true;
        }

        error.source().is_some_and(Self::error_is_not_found)
    }
}

#[cfg(test)]
mod tests {
    use super::InfraError;

    #[test]
    fn test_s3_read_not_found_is_detected_from_source_chain() {
        let error = InfraError::s3_read(
            "bucket",
            "missing.html",
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing object"),
        );

        assert!(error.is_not_found());
    }

    #[test]
    fn test_s3_read_not_found_is_detected_from_message() {
        let error = InfraError::s3_read(
            "bucket",
            "missing.html",
            std::io::Error::other("NoSuchKey: missing"),
        );

        assert!(error.is_not_found());
    }
}
