use anyhow::Result;
use publisher::publish;
use std::path::Path;

const OBSIDIAN_DIR: &str = "crates/publish/publisher/obsidian/Publish";
const OUTPUT_DIR: &str = "crates/publish/publisher/dist";

#[tokio::main]
async fn main() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", "info")
        .write_style_or("LOG_STYLE", "auto");
    env_logger::init_from_env(env);

    publish(Path::new(OBSIDIAN_DIR), Path::new(OUTPUT_DIR)).await?;

    Ok(())
}
