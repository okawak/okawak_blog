//! Framework-neutral artifact readiness check shared by server runtimes.

use infra::{DynArtifactReader, Result};

pub async fn check_artifact_readiness(artifact_reader: &DynArtifactReader) -> Result<()> {
    let snapshot = artifact_reader.snapshot().await?;
    snapshot.read_site_metadata().await?;
    Ok(())
}
