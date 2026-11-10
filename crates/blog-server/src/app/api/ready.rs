use topcoat::{
    Result,
    context::{Cx, context},
    router::{StatusCode, route},
};

use blog_infra::DynArtifactReader;

#[route(GET)]
async fn readiness(cx: &Cx) -> Result<(StatusCode, &'static str)> {
    let artifact_reader = context::<DynArtifactReader>(cx);
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
