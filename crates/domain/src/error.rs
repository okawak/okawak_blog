use thiserror::Error;

/// Core domain errors - Rustの慣用的エラーハンドリング
pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum CoreError {
    #[error("記事が見つかりません: {id}")]
    ArticleNotFound { id: String },

    #[error("無効なスラッグです: {slug}")]
    InvalidSlug { slug: String },

    #[error("無効なタイトルです: {reason}")]
    InvalidTitle { reason: String },

    #[error("無効なカテゴリです: {category}")]
    InvalidCategory { category: String },

    #[error("ビジネスルール違反: {rule}")]
    BusinessRuleViolation { rule: String },

    #[error("外部サービスエラー: {service}")]
    ExternalService { service: String },

    #[error("設定エラー: {message}")]
    Configuration { message: String },
}

impl CoreError {
    /// ビジネスルール違反エラーのヘルパー
    pub fn business_rule<S: Into<String>>(rule: S) -> Self {
        Self::BusinessRuleViolation { rule: rule.into() }
    }

    /// 外部サービスエラーのヘルパー
    pub fn external_service<S: Into<String>>(service: S) -> Self {
        Self::ExternalService {
            service: service.into(),
        }
    }
}
