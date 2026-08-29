use thiserror::Error;

/// Pure domain errors without I/O.
pub type Result<T> = std::result::Result<T, DomainError>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("invalid slug: {slug}")]
    InvalidSlug { slug: String },

    #[error("invalid title: {reason}")]
    InvalidTitle { reason: String },

    #[error("invalid category: {category}")]
    InvalidCategory { category: String },

    #[error("invalid path: {path}")]
    InvalidPath { path: String },

    #[error("invalid RFC 3339 timestamp: {value}")]
    InvalidTimestamp { value: String },

    #[error("validation error: {field}")]
    ValidationError { field: String },
}

impl DomainError {
    /// Helper for creating validation errors.
    pub fn validation<S: Into<String>>(field: S) -> Self {
        Self::ValidationError {
            field: field.into(),
        }
    }
}
