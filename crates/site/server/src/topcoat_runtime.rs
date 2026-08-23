//! Parallel Topcoat runtime shell used during the framework migration.

use infra::DynArtifactReader;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Router, StatusCode, content::Json, error::internal_server_error, route},
};

use crate::{article_index::read_article_index, readiness::check_artifact_readiness};

#[derive(Clone)]
struct ArtifactReaderContext(DynArtifactReader);

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("OK")
}

#[route(GET "/api/ready")]
async fn readiness(cx: &Cx) -> Result<(StatusCode, &'static str)> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;

    match check_artifact_readiness(artifact_reader).await {
        Ok(()) => Ok((StatusCode::OK, "READY")),
        Err(error) => {
            eprintln!("Artifact readiness check failed: {error}");
            Ok((StatusCode::SERVICE_UNAVAILABLE, "NOT READY"))
        }
    }
}

#[route(GET "/api/articles")]
async fn articles(cx: &Cx) -> Result<Json<domain::ArticleIndexDocument>> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;
    let document = read_article_index(artifact_reader)
        .await
        .map_err(internal_server_error)?;
    Ok(Json(document))
}

pub fn create_topcoat_router(artifact_reader: DynArtifactReader) -> Router {
    Router::builder()
        .route(health)
        .route(readiness)
        .route(articles)
        .app_context(ArtifactReaderContext(artifact_reader))
        .build()
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use axum::http::{Request, StatusCode};
    use infra::LocalArtifactReader;
    use tempfile::tempdir;
    use topcoat::router::{Body, to_bytes};

    use super::create_topcoat_router;

    struct TestResponse {
        status: StatusCode,
        content_type: Option<String>,
        body: String,
    }

    fn fixture_reader() -> Arc<LocalArtifactReader> {
        Arc::new(LocalArtifactReader::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../e2e/fixtures/site"),
        ))
    }

    async fn response(path: &str, reader: Arc<LocalArtifactReader>) -> TestResponse {
        let router = create_topcoat_router(reader);
        let request = Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("request should be valid");
        let response = router.handle(request).await;
        let status = response.status();
        let content_type = response.headers().get("content-type").map(|value| {
            value
                .to_str()
                .expect("content type should be valid")
                .to_owned()
        });
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");

        TestResponse {
            status,
            content_type,
            body: String::from_utf8(body.to_vec()).expect("response body should be UTF-8"),
        }
    }

    #[tokio::test]
    async fn health_does_not_require_artifacts() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let response = response(
            "/api/health",
            Arc::new(LocalArtifactReader::new(temp_dir.path())),
        )
        .await;

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, "OK");
    }

    #[tokio::test]
    async fn readiness_succeeds_when_site_metadata_is_readable() {
        let response = response("/api/ready", fixture_reader()).await;

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, "READY");
    }

    #[tokio::test]
    async fn readiness_fails_when_site_metadata_is_missing() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let response = response(
            "/api/ready",
            Arc::new(LocalArtifactReader::new(temp_dir.path())),
        )
        .await;

        assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.body, "NOT READY");
    }

    #[tokio::test]
    async fn articles_returns_the_published_index_as_json() {
        let response = response("/api/articles", fixture_reader()).await;

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.content_type.as_deref(), Some("application/json"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response.body)
                .expect("article index should be JSON"),
            serde_json::json!({
                "articles": [{
                    "slug": "e2e-article",
                    "title": "E2E Article",
                    "category": "tech",
                    "section_path": ["rust", "async"],
                    "description": "Article fixture description",
                    "tags": ["rust", "e2e"],
                    "priority": 10,
                    "created_at": "2026-01-01T00:00:00+09:00",
                    "updated_at": "2026-01-02T00:00:00+09:00"
                }]
            })
        );
    }

    #[tokio::test]
    async fn articles_returns_internal_server_error_when_index_is_missing() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let response = response(
            "/api/articles",
            Arc::new(LocalArtifactReader::new(temp_dir.path())),
        )
        .await;

        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}
