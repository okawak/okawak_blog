use anyhow::{Context, Result, bail};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut source = PathBuf::from("crates/publish/obsidian/Publish");
    let mut output = PathBuf::from("content");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source" => source = args.next().context("--source needs a path")?.into(),
            "--output" => output = args.next().context("--output needs a path")?.into(),
            "--help" => {
                println!("export [--source PUBLIC_VAULT_ROOT] [--output content]");
                return Ok(());
            }
            _ => bail!("unknown option: {arg}"),
        }
    }
    export::export_japanese(&source, &output)?;
    println!("Japanese public Markdown exported to {}", output.display());
    Ok(())
}
