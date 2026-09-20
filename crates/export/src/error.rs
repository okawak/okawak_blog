use std::{io, path::StripPrefixError, str::Utf8Error};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ExportError>;

/// Failures callers may need to distinguish when exporting or reviewing content.
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("file system operation failed: {0}")]
    Io(#[from] io::Error),

    #[error("failed to read or write JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to read or write YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("domain validation failed: {0}")]
    Domain(#[from] domain::DomainError),

    #[error("path is outside the selected root: {0}")]
    StripPrefix(#[from] StripPrefixError),

    #[error("decoded path is not valid UTF-8: {0}")]
    Utf8(#[from] Utf8Error),

    #[error("failed to persist staged output: {0}")]
    Persist(#[from] tempfile::PersistError),

    #[error("invalid export input: {0}")]
    InvalidInput(String),

    #[error("translation response rejected: {0}")]
    InvalidTranslation(String),

    #[error("translation candidate conflict: {0}")]
    TranslationConflict(String),

    #[error("translation provider failed: {0}")]
    Translator(String),
}

impl ExportError {
    pub(crate) fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub(crate) fn invalid_translation(message: impl Into<String>) -> Self {
        Self::InvalidTranslation(message.into())
    }

    pub(crate) fn translation_conflict(message: impl Into<String>) -> Self {
        Self::TranslationConflict(message.into())
    }

    /// Creates an adapter failure without exposing provider-specific error types.
    pub fn translator(message: impl Into<String>) -> Self {
        Self::Translator(message.into())
    }
}
