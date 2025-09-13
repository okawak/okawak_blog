//! Axum Server Setup - サーバー設定とルーティング

use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::{
    handlers::{
        articles::{
            AppState, create_article_handler, get_article_handler,
            list_articles_by_category_handler, list_latest_articles_handler,
            publish_article_handler, search_articles_handler,
        },
        health::health_check,
    },
    infrastructure::{InMemoryArticleRepository, InMemorySearchService, S3FileStorage},
};

/// Axum アプリケーションを作成
pub fn create_app(
    repository: Arc<InMemoryArticleRepository>,
    storage: Arc<S3FileStorage>,
    search: Arc<InMemorySearchService>,
) -> Router {
    let app_state = AppState {
        repository,
        storage,
        search,
    };

    Router::new()
        // Health check
        .route("/health", get(health_check))
        // Article API routes
        .route("/api/articles", post(create_article_handler))
        .route("/api/articles/:id/publish", post(publish_article_handler))
        .route("/api/articles/:category/:slug", get(get_article_handler))
        .route(
            "/api/articles/category/:category",
            get(list_articles_by_category_handler),
        )
        .route("/api/articles/latest", get(list_latest_articles_handler))
        .route("/api/articles/search", get(search_articles_handler))
        // Middleware
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        // State
        .with_state(app_state)
}

/// アプリケーション状態の初期化
pub async fn initialize_services(
    s3_client: aws_sdk_s3::Client,
    bucket_name: String,
) -> (
    Arc<InMemoryArticleRepository>,
    Arc<S3FileStorage>,
    Arc<InMemorySearchService>,
) {
    let repository = Arc::new(InMemoryArticleRepository::new());
    let storage = Arc::new(S3FileStorage::new(s3_client, bucket_name));
    let search = Arc::new(InMemorySearchService::new());

    (repository, storage, search)
}
