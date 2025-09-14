use anyhow::Result;
use obsidian_uploader::{Config, run_main};

#[tokio::main]
async fn main() -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", "info")
        .write_style_or("LOG_STYLE", "auto");
    env_logger::init_from_env(env);

    let config = Config::new()?;
    run_main(&config).await?;

    Ok(())
}
