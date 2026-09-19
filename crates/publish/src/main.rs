use anyhow::{Context, Result, bail};
use publish::publish;
use std::path::PathBuf;

const CONTENT_DIR: &str = "content";
const OUTPUT_DIR: &str = "crates/publish/dist";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .map_err(anyhow::Error::from_boxed)?;

    let mut input = PathBuf::from(CONTENT_DIR);
    let mut output = PathBuf::from(OUTPUT_DIR);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input = args.next().context("--input needs a path")?.into(),
            "--output" => output = args.next().context("--output needs a path")?.into(),
            "--help" => {
                println!("publish [--input content] [--output crates/publish/dist]");
                return Ok(());
            }
            _ => bail!("unknown option: {arg}"),
        }
    }
    publish(&input, &output).await?;

    Ok(())
}
