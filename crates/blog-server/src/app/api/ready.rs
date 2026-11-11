use topcoat::{
    Result,
    context::{Cx, context},
    openapi::Operation,
    router::{StatusCode, route},
};

use blog_infra::DynArtifactReader;

#[route(
    GET,
    with(Operation::new()
        .operation_id("readiness")
        .summary("Check artifact readiness")
        .description(
            "Reads the configured snapshot and site metadata. A usable cached stale snapshot is considered ready.",
        )
        .tag("health")
        .response_with::<&str>(StatusCode::OK, "READY")
        .response_with::<&str>(StatusCode::SERVICE_UNAVAILABLE, "NOT READY"))
)]
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
