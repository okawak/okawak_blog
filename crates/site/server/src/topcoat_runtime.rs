//! Parallel Topcoat runtime shell used during the framework migration.

use infra::DynArtifactReader;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Router, StatusCode, route},
};

use crate::readiness::check_artifact_readiness;

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

pub fn create_topcoat_router(artifact_reader: DynArtifactReader) -> Router {
    Router::builder()
        .route(health)
        .route(readiness)
        .app_context(ArtifactReaderContext(artifact_reader))
        .build()
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use axum::http::Request;
    use infra::LocalArtifactReader;
    use tempfile::tempdir;
    use topcoat::router::{Body, StatusCode, to_bytes};

    use super::create_topcoat_router;

    fn fixture_reader() -> Arc<LocalArtifactReader> {
        Arc::new(LocalArtifactReader::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../e2e/fixtures/site"),
        ))
    }

    async fn response(path: &str, reader: Arc<LocalArtifactReader>) -> (StatusCode, String) {
        let router = create_topcoat_router(reader);
        let request = Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("request should be valid");
        let response = router.handle(request).await;
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");

        (
            status,
            String::from_utf8(body.to_vec()).expect("response body should be UTF-8"),
        )
    }

    #[tokio::test]
    async fn health_does_not_require_artifacts() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let (status, body) = response(
            "/api/health",
            Arc::new(LocalArtifactReader::new(temp_dir.path())),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "OK");
    }

    #[tokio::test]
    async fn readiness_succeeds_when_site_metadata_is_readable() {
        let (status, body) = response("/api/ready", fixture_reader()).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "READY");
    }

    #[tokio::test]
    async fn readiness_fails_when_site_metadata_is_missing() {
        let temp_dir = tempdir().expect("temp dir should be created");
        let (status, body) = response(
            "/api/ready",
            Arc::new(LocalArtifactReader::new(temp_dir.path())),
        )
        .await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body, "NOT READY");
    }
}
