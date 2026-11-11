//! Production entry point for the site server.

use anyhow::{Context, Result};
use blog_infra::{ArtifactSourceConfig, build_artifact_reader};
use blog_server::app::create_router;
use blog_server::http_cache::artifact_validators_enabled;
use topcoat::{asset::AssetBundle, context::BaseUrl};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

const DEFAULT_SITE_ORIGIN: &str = "https://www.okawak.net";
const SITE_ORIGIN_ENV: &str = "OKAWAK_BLOG_SITE_ORIGIN";

fn configured_base_url() -> Result<BaseUrl> {
    let origin = std::env::var(SITE_ORIGIN_ENV)
        .ok()
        .filter(|origin| !origin.is_empty())
        .unwrap_or_else(|| DEFAULT_SITE_ORIGIN.to_string());
    BaseUrl::new(origin).context("invalid OKAWAK_BLOG_SITE_ORIGIN")
}

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
    let base_url = configured_base_url()?;
    let artifact_source = ArtifactSourceConfig::from_env()?;
    let artifact_reader = build_artifact_reader(artifact_source.clone()).await?;
    let validators_enabled = artifact_validators_enabled(&artifact_source);
    let assets = AssetBundle::load()?;
    let router = create_router(artifact_reader, validators_enabled, assets.into(), base_url);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(
        listen_addr = %addr,
        artifact_source = artifact_source.kind(),
        "site server starting"
    );

    topcoat::serve(listener, router).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_origin_uses_nonempty_override_or_production_default() {
        const EXPECTED: &str = "BLOG_TEST_EXPECTED_ORIGIN";
        if let Ok(expected) = std::env::var(EXPECTED) {
            assert_eq!(configured_base_url().unwrap().as_str(), expected);
            return;
        }
        // Isolate environment changes from concurrent router and metadata tests.
        for (origin, expected) in [
            (None, DEFAULT_SITE_ORIGIN),
            (Some(""), DEFAULT_SITE_ORIGIN),
            (Some("https://preview.example"), "https://preview.example"),
            (
                Some("https://preview.example///"),
                "https://preview.example",
            ),
        ] {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child
                .args([
                    "--exact",
                    "tests::site_origin_uses_nonempty_override_or_production_default",
                ])
                .env(EXPECTED, expected)
                .env_remove(SITE_ORIGIN_ENV);
            if let Some(origin) = origin {
                child.env(SITE_ORIGIN_ENV, origin);
            }
            let output = child.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }
}
