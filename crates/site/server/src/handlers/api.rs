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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_fixture_site;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use domain::{ArticlePageDocument, CategoryPageDocument, HomePageDocument};
    use infra::LocalArtifactReader;
    use std::sync::Arc;
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    const MAX_TEST_RESPONSE_BYTES: usize = 1024 * 1024;

    fn create_test_router(site_root: &std::path::Path) -> Router {
        create_api_router(Arc::new(LocalArtifactReader::new(site_root)))
            .with_state(LeptosOptions::builder().output_name("web").build())
    }

    async fn request_json<T: serde::de::DeserializeOwned>(
        router: Router,
        uri: &str,
    ) -> (StatusCode, T) {
        let response = router
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), MAX_TEST_RESPONSE_BYTES)
            .await
            .unwrap();
        let document = serde_json::from_slice(&body).unwrap();

        (status, document)
    }

    #[tokio::test]
    async fn test_api_router_serves_home_page_fixture() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let (status, document): (StatusCode, HomePageDocument) =
            request_json(create_test_router(temp_dir.path()), "/page/home").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(document.total_articles, 1);
        assert_eq!(document.articles.len(), 1);
    }

    #[tokio::test]
    async fn test_api_router_serves_article_page_fixture() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let (status, document): (StatusCode, ArticlePageDocument) = request_json(
            create_test_router(temp_dir.path()),
            "/page/articles/sample0000001",
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(document.article.title.as_str(), "Sample");
        assert!(document.html.contains("<h1>Sample</h1>"));
    }

    #[tokio::test]
    async fn test_api_router_returns_not_found_for_missing_article() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let response = create_test_router(temp_dir.path())
            .oneshot(
                Request::builder()
                    .uri("/page/articles/missing000001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_api_router_serves_category_page_fixture() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let (status, document): (StatusCode, CategoryPageDocument) =
            request_json(create_test_router(temp_dir.path()), "/page/categories/tech").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(document.category_display_name, "技術");
        assert_eq!(document.articles.len(), 1);
    }

    #[tokio::test]
    async fn test_api_router_returns_not_found_for_invalid_category() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let response = create_test_router(temp_dir.path())
            .oneshot(
                Request::builder()
                    .uri("/page/categories/unknown")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
