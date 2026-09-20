mod cli;

use clap::Parser;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .try_init()
        .map_err(anyhow::Error::from_boxed)?;
    cli::Cli::parse().run()
}
