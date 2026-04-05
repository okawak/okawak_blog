//! API Handlers - HTTP API エンドポイント

pub mod articles;

pub use articles::*;

use axum::{Router, routing::get};
use leptos::prelude::LeptosOptions;
use std::sync::Arc;

use infra::LocalArtifactReader;

/// API ルーターを作成
pub fn create_api_router(artifact_reader: Arc<LocalArtifactReader>) -> Router<LeptosOptions> {
    Router::new().route(
        "/articles",
        get(move || articles::list_articles(artifact_reader.clone())),
    )
}
