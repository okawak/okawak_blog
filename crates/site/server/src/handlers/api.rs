//! HTTP API handlers.

pub mod articles;

pub use articles::*;

use axum::Extension;
use axum::{Router, routing::get};
use infra::DynArtifactReader;
use leptos::prelude::LeptosOptions;

/// Builds the compatibility API router.
pub fn create_api_router(artifact_reader: DynArtifactReader) -> Router<LeptosOptions> {
    Router::new()
        .route("/articles", get(articles::list_articles))
        .layer(Extension(artifact_reader))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use infra::LocalArtifactReader;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    fn create_test_router(site_root: &std::path::Path) -> Router {
        create_api_router(Arc::new(LocalArtifactReader::new(site_root)))
            .with_state(LeptosOptions::builder().output_name("web").build())
    }

    #[tokio::test]
    async fn test_api_router_does_not_expose_page_documents() {
        let temp_dir = TempDir::new().unwrap();

        let response = create_test_router(temp_dir.path())
            .oneshot(
                Request::builder()
                    .uri("/page/home")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
