//! Production entry point for the site server.

mod logging;

use infra::{ArtifactSourceConfig, build_artifact_reader};
use server::http_cache::artifact_validators_enabled;
use server::router::create_router;
use topcoat::asset::AssetBundle;

const DEFAULT_ADDR: &str = "127.0.0.1:8008";
const ADDR_ENV: &str = "OKAWAK_BLOG_ADDR";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    logging::init()?;

    let addr = std::env::var(ADDR_ENV).unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let artifact_source = ArtifactSourceConfig::from_env()?;
    let artifact_reader = build_artifact_reader(artifact_source.clone()).await?;
    let validators_enabled = artifact_validators_enabled(&artifact_source);
    let assets = AssetBundle::load()?;
    let router = create_router(artifact_reader, validators_enabled, assets.into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(
        listen_addr = %addr,
        artifact_source = artifact_source.kind(),
        "site server starting"
    );

    topcoat::serve(listener, router).await?;
    Ok(())
}
