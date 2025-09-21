use thiserror::Error;

/// Pure domain errors - 純粋ドメインエラー（I/Oなし）
pub type Result<T> = std::result::Result<T, DomainError>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    #[error("無効なIDです: {id}")]
    InvalidId { id: String },

    #[error("無効なスラッグです: {slug}")]
    InvalidSlug { slug: String },

    #[error("無効なタイトルです: {reason}")]
    InvalidTitle { reason: String },

    #[error("無効なカテゴリです: {category}")]
    InvalidCategory { category: String },

    #[error("ビジネスルール違反: {rule}")]
    BusinessRuleViolation { rule: String },

    #[error("バリデーションエラー: {field}")]
    ValidationError { field: String },
}

impl DomainError {
    /// ビジネスルール違反エラーのヘルパー
    pub fn business_rule<S: Into<String>>(rule: S) -> Self {
        Self::BusinessRuleViolation { rule: rule.into() }
    }

    /// バリデーションエラーのヘルパー
    pub fn validation<S: Into<String>>(field: S) -> Self {
        Self::ValidationError {
            field: field.into(),
        }
    }
}
