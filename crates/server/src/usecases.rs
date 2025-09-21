//! Use Cases - 旧アーキテクチャ互換性スタブ
//! 将来的に削除予定

use crate::ports::{ArticleRepository, FileStorage};
use domain::{Article, ArticleSummary};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UseCaseError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Repository error: {0}")]
    Repository(String),
}

pub struct BlogUseCases<R, S> {
    _repository: Arc<R>,
    _storage: Arc<S>,
}

impl<R, S> BlogUseCases<R, S>
where
    R: ArticleRepository,
    S: FileStorage,
{
    pub fn new(repository: Arc<R>, storage: Arc<S>) -> Self {
        Self {
            _repository: repository,
            _storage: storage,
        }
    }

    pub async fn get_article_by_id(&self, _id: String) -> Result<Article, UseCaseError> {
        // スタブ実装
        Err(UseCaseError::NotFound(
            "Article not implemented".to_string(),
        ))
    }

    pub async fn get_article_by_slug(&self, _slug: String) -> Result<Article, UseCaseError> {
        // スタブ実装
        Err(UseCaseError::NotFound(
            "Article not implemented".to_string(),
        ))
    }

    pub async fn list_articles(&self) -> Result<Vec<ArticleSummary>, UseCaseError> {
        // スタブ実装
        Ok(vec![])
    }
}
