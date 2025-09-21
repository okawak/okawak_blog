//! Ports - 旧アーキテクチャ互換性スタブ
//! 将来的に削除予定

use async_trait::async_trait;
use domain::{Article, ArticleSummary};
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_trait]
pub trait ArticleRepository: Send + Sync {
    async fn find_by_id(&self, _id: &str) -> Result<Option<Article>>;
    async fn find_by_slug(&self, _slug: &str) -> Result<Option<Article>>;
    async fn list_all(&self) -> Result<Vec<ArticleSummary>>;
}

#[async_trait]
pub trait FileStorage: Send + Sync {
    async fn get_content(&self, _path: &str) -> Result<String>;
    async fn put_content(&self, _path: &str, _content: &str) -> Result<()>;
}
