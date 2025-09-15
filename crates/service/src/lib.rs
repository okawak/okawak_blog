pub mod blog;
pub mod config;
pub mod error;
pub mod s3;

pub use domain::{Article, ArticleId, ArticleSummary, Category};
pub use error::ServiceError;

pub type Result<T> = std::result::Result<T, ServiceError>;
