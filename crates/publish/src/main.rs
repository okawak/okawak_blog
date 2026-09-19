use anyhow::{Context, Result, bail};
use publish::publish;
use std::path::PathBuf;

mod artifact_check;

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
    let mut validation_root = None;
    let mut generation_option = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                input = args.next().context("--input needs a path")?.into();
                generation_option = true;
            }
            "--output" => {
                output = args.next().context("--output needs a path")?.into();
                generation_option = true;
            }
            "--validate-artifacts" => {
                if validation_root.is_some() {
                    bail!("--validate-artifacts specified twice");
                }
                validation_root = Some(PathBuf::from(
                    args.next()
                        .context("--validate-artifacts needs a site path")?,
                ));
            }
            "--help" => {
                println!("publish [--input content] [--output crates/publish/dist]");
                println!("publish --validate-artifacts SITE_ROOT");
                return Ok(());
            }
            _ => bail!("unknown option: {arg}"),
        }
    }
    if let Some(root) = validation_root {
        if generation_option {
            bail!("artifact validation cannot be combined with --input or --output");
        }
        return artifact_check::validate(&root);
    }
    publish(&input, &output).await?;

    Ok(())
}
