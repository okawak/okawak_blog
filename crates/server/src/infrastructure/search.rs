//! Search Service Implementation - 検索サービス実装

use async_trait::async_trait;
use core::{Article, ArticleId, Result, ports::SearchService};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Simple in-memory search service (将来的に Elasticsearch などに置き換え)
#[derive(Debug, Clone)]
pub struct InMemorySearchService {
    index: Arc<RwLock<HashMap<ArticleId, SearchDocument>>>,
}

#[derive(Debug, Clone)]
struct SearchDocument {
    id: ArticleId,
    title: String,
    content: String,
    tags: Vec<String>,
}

impl InMemorySearchService {
    pub fn new() -> Self {
        Self {
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemorySearchService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchService for InMemorySearchService {
    async fn index_article(&self, _article: &Article) -> Result<()> {
        todo!("検索インデックス追加機能の実装")
    }

    async fn remove_article(&self, _id: &ArticleId) -> Result<()> {
        todo!("検索インデックス削除機能の実装")
    }

    async fn search(&self, _query: &str, _limit: Option<usize>) -> Result<Vec<ArticleId>> {
        todo!("検索機能の実装")
    }
}
