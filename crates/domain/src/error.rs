use thiserror::Error;

/// Pure domain errors without I/O.
pub type Result<T> = std::result::Result<T, DomainError>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("無効なスラッグです: {slug}")]
    InvalidSlug { slug: String },

    #[error("無効なタイトルです: {reason}")]
    InvalidTitle { reason: String },

    #[error("無効なカテゴリです: {category}")]
    InvalidCategory { category: String },

    #[error("無効なパスです: {path}")]
    InvalidPath { path: String },

    #[error("バリデーションエラー: {field}")]
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
