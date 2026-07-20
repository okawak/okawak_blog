use anyhow::Result;
use publisher::run_main;
use std::path::Path;

const OBSIDIAN_DIR: &str = "crates/publish/publisher/obsidian/Publish";
const OUTPUT_DIR: &str = "crates/publish/publisher/dist";

#[tokio::main]
async fn main() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", "info")
        .write_style_or("LOG_STYLE", "auto");
    env_logger::init_from_env(env);

    run_main(Path::new(OBSIDIAN_DIR), Path::new(OUTPUT_DIR)).await?;

    Ok(())
}
