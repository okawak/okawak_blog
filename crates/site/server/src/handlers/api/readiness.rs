//! Runtime readiness checks backed by the configured artifact reader.

use axum::{Extension, http::StatusCode};
use infra::DynArtifactReader;

pub async fn artifact_readiness(
    Extension(artifact_reader): Extension<DynArtifactReader>,
) -> Result<&'static str, (StatusCode, &'static str)> {
    let result = async {
        let snapshot = artifact_reader.snapshot().await?;
        snapshot.read_site_metadata().await
    }
    .await;

    result.map(|_| "READY").map_err(|error| {
        eprintln!("Artifact readiness check failed: {error}");
        (StatusCode::SERVICE_UNAVAILABLE, "NOT READY")
    })
}
