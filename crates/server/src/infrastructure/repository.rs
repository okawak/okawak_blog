//! Repository Implementation - データアクセス実装

use async_trait::async_trait;
use domain::{
    Article, ArticleId, ArticleSummary, Category, Result, Slug, ports::ArticleRepository,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory repository implementation (将来的にデータベース実装に置き換え)
#[derive(Debug, Clone)]
pub struct InMemoryArticleRepository {
    articles: Arc<RwLock<HashMap<ArticleId, Article>>>,
}

impl InMemoryArticleRepository {
    pub fn new() -> Self {
        Self {
            articles: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryArticleRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ArticleRepository for InMemoryArticleRepository {
    async fn find_by_id(&self, _id: &ArticleId) -> Result<Option<Article>> {
        todo!("記事ID検索機能の実装")
    }

    async fn find_by_slug(&self, _category: &Category, _slug: &Slug) -> Result<Option<Article>> {
        todo!("スラッグ検索機能の実装")
    }

    async fn find_by_category(
        &self,
        _category: &Category,
        _limit: Option<usize>,
    ) -> Result<Vec<ArticleSummary>> {
        todo!("カテゴリ別検索機能の実装")
    }

    async fn find_latest(&self, _limit: usize) -> Result<Vec<ArticleSummary>> {
        todo!("最新記事検索機能の実装")
    }

    async fn save(&self, _article: &Article) -> Result<()> {
        todo!("記事保存機能の実装")
    }

    async fn delete(&self, _id: &ArticleId) -> Result<()> {
        todo!("記事削除機能の実装")
    }

    async fn search(&self, _query: &str, _limit: Option<usize>) -> Result<Vec<ArticleSummary>> {
        todo!("記事検索機能の実装")
    }
}
