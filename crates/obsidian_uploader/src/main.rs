use anyhow::Result;
use obsidian_uploader::{Config, run_main};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let config = Config::new()?;
    run_main(&config).await?;

    Ok(())
}
