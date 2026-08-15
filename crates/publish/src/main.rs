use anyhow::Result;
use publish::publish;
use std::path::Path;

const OBSIDIAN_DIR: &str = "crates/publish/obsidian/Publish";
const OUTPUT_DIR: &str = "crates/publish/dist";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .map_err(anyhow::Error::from_boxed)?;

    publish(Path::new(OBSIDIAN_DIR), Path::new(OUTPUT_DIR)).await?;

    Ok(())
}
