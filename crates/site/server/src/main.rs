//! Production entry point for the site server.

use anyhow::Result;
use infra::{ArtifactSourceConfig, build_artifact_reader};
use server::http_cache::artifact_validators_enabled;
use server::router::create_router;
use topcoat::asset::AssetBundle;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

const DEFAULT_ADDR: &str = "127.0.0.1:8008";
const ADDR_ENV: &str = "OKAWAK_BLOG_ADDR";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

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
