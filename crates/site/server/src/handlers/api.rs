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
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use domain::{
        ArticleIndexDocument, ArticlePageDocument, ArticleSummaryDocument, CategoryIndexDocument,
        CategoryMetadataDocument, CategoryPageDocument, HomePageDocument, SiteMetadataDocument,
    };
    use infra::LocalArtifactReader;
    use std::{fs, sync::Arc};
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    fn write_fixture_site(root: &std::path::Path) {
        fs::create_dir_all(root.join("articles")).unwrap();
        fs::create_dir_all(root.join("categories")).unwrap();
        fs::create_dir_all(root.join("metadata")).unwrap();

        fs::write(
            root.join("articles/index.json"),
            serde_json::to_string_pretty(&ArticleIndexDocument {
                articles: vec![ArticleSummaryDocument {
                    slug: "sample0000001".to_string(),
                    title: "Sample".to_string(),
                    category: "tech".to_string(),
                    description: Some("summary".to_string()),
                    tags: vec!["rust".to_string()],
                    priority: Some(1),
                    created_at: "2025-01-01T00:00:00+09:00".to_string(),
                    updated_at: "2025-01-01T00:00:00+09:00".to_string(),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("categories/tech.json"),
            serde_json::to_string_pretty(&CategoryIndexDocument {
                category: "tech".to_string(),
                articles: vec![ArticleSummaryDocument {
                    slug: "sample0000001".to_string(),
                    title: "Sample".to_string(),
                    category: "tech".to_string(),
                    description: Some("summary".to_string()),
                    tags: vec!["rust".to_string()],
                    priority: Some(1),
                    created_at: "2025-01-01T00:00:00+09:00".to_string(),
                    updated_at: "2025-01-01T00:00:00+09:00".to_string(),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("metadata/site.json"),
            serde_json::to_string_pretty(&SiteMetadataDocument {
                total_articles: 1,
                categories: vec![CategoryMetadataDocument {
                    category: "tech".to_string(),
                    article_count: 1,
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("articles/sample0000001.html"),
            "<article><h1>Sample</h1></article>",
        )
        .unwrap();
    }

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
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
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
