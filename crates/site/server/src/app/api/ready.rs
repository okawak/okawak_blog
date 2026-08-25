use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{StatusCode, route},
};

use super::ArtifactReaderContext;

#[route(GET)]
async fn readiness(cx: &Cx) -> Result<(StatusCode, &'static str)> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;
    let ready = match artifact_reader.snapshot().await {
        Ok(snapshot) => snapshot.read_site_metadata().await,
        Err(error) => Err(error),
    };

    match ready {
        Ok(_) => Ok((StatusCode::OK, "READY")),
        Err(error) => {
            tracing::warn!(%error, "artifact readiness check failed");
            Ok((StatusCode::SERVICE_UNAVAILABLE, "NOT READY"))
        }
    }
}
