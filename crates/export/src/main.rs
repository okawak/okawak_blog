use anyhow::{Context, Result, bail};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut source = PathBuf::from("crates/publish/obsidian/Publish");
    let mut output = PathBuf::from("content");
    let mut translate = false;
    let mut public_only = false;
    let mut candidates = false;
    let mut accept = None;
    let mut settings_path = PathBuf::from("translation/settings.json");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--translate" => translate = true,
            "--translate-only" => {
                translate = true;
                public_only = true;
            }
            "--candidates" => candidates = true,
            "--accept" => {
                accept = Some(
                    args.next()
                        .context("--accept needs an ID")?
                        .parse::<domain::Slug>()?,
                )
            }
            "--settings" => settings_path = args.next().context("--settings needs a path")?.into(),
            "--source" => source = args.next().context("--source needs a path")?.into(),
            "--output" => output = args.next().context("--output needs a path")?.into(),
            "--help" => {
                println!(
                    "export [--source PUBLIC_VAULT_ROOT] [--output content] [--translate | --translate-only | --accept ID] [--candidates] [--settings translation/settings.json]"
                );
                return Ok(());
            }
            _ => bail!("unknown option: {arg}"),
        }
    }
    if translate || accept.is_some() {
        let settings = serde_json::from_slice(&std::fs::read(settings_path)?)?;
        if let Some(id) = accept {
            export::accept_translation(&output, &id, &settings)?;
        } else {
            let translator = export::CodexTranslator::default();
            let report = if public_only {
                export::translate_public(&output, &translator, &settings, candidates)?
            } else {
                export::export_translated(&source, &output, &translator, &settings, candidates)?
            };
            println!(
                "Generated: {}, reused: {}, protected: {:?}",
                report.generated, report.reused, report.protected
            );
        }
    } else {
        export::export_japanese(&source, &output)?;
        println!("Japanese public Markdown exported to {}", output.display());
    }
    Ok(())
}
