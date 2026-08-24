//! Runtime API endpoints backed by the configured artifact reader.

use infra::{DynArtifactReader, DynArtifactSnapshot};
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{StatusCode, content::Json, error::internal_server_error, route},
};

#[derive(Clone)]
pub(crate) struct ArtifactReaderContext(pub(crate) DynArtifactReader);

#[route(GET "/api/health")]
pub(crate) async fn health() -> Result<&'static str> {
    Ok("OK")
}

#[route(GET "/api/ready")]
pub(crate) async fn readiness(cx: &Cx) -> Result<(StatusCode, &'static str)> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;
    let ready = match artifact_reader.snapshot().await {
        Ok(snapshot) => snapshot.read_site_metadata().await,
        Err(error) => Err(error),
    };

    match ready {
        Ok(_) => Ok((StatusCode::OK, "READY")),
        Err(error) => {
            eprintln!("Artifact readiness check failed: {error}");
            Ok((StatusCode::SERVICE_UNAVAILABLE, "NOT READY"))
        }
    }
}

#[route(GET "/api/articles")]
pub(crate) async fn articles(cx: &Cx) -> Result<Json<domain::ArticleIndexDocument>> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;
    let document = match try_request_context::<DynArtifactSnapshot>(cx) {
        Some(snapshot) => snapshot.read_article_index().await,
        None => {
            let snapshot = artifact_reader
                .snapshot()
                .await
                .map_err(internal_server_error)?;
            snapshot.read_article_index().await
        }
    }
    .map_err(internal_server_error)?;
    Ok(Json(document))
}
