//! API Handlers - HTTP API エンドポイント

pub mod articles;

pub use articles::*;

use axum::{Router, routing::get};
use std::sync::Arc;

use crate::infrastructure::{MemoryArticleRepository, S3Storage};

pub struct AppState {
    pub _repository: Arc<MemoryArticleRepository>,
    pub _storage: Arc<S3Storage>,
}

/// API ルーターを作成
pub fn create_api_router() -> Router {
    Router::new().route("/articles", get(articles::list_articles))
}
