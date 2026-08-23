//! Runtime readiness checks backed by the configured artifact reader.

use axum::{Extension, http::StatusCode};
use infra::DynArtifactReader;

use crate::readiness::check_artifact_readiness;

pub async fn artifact_readiness(
    Extension(artifact_reader): Extension<DynArtifactReader>,
) -> Result<&'static str, (StatusCode, &'static str)> {
    check_artifact_readiness(&artifact_reader)
        .await
        .map(|()| "READY")
        .map_err(|error| {
            eprintln!("Artifact readiness check failed: {error}");
            (StatusCode::SERVICE_UNAVAILABLE, "NOT READY")
        })
}
