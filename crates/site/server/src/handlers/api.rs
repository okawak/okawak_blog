//! API Handlers - HTTP API エンドポイント

pub mod articles;
pub mod pages;

pub use articles::*;
pub use pages::*;

use axum::Extension;
use axum::{Router, routing::get};
use infra::DynArtifactReader;
use leptos::prelude::LeptosOptions;

/// API ルーターを作成
pub fn create_api_router(artifact_reader: DynArtifactReader) -> Router<LeptosOptions> {
    Router::new()
        .route("/articles", get(articles::list_articles))
        .route("/page/home", get(pages::get_home_page))
        .route("/page/articles/{slug}", get(pages::get_article_page))
        .route("/page/categories/{category}", get(pages::get_category_page))
        .layer(Extension(artifact_reader))
}
