use anyhow::{Context, Result, bail};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut source = PathBuf::from("crates/publish/obsidian/Publish");
    let mut output = PathBuf::from("content");
    let mut translate = false;
    let mut public_only = false;
    let mut candidates = false;
    let mut accept = None;
    let mut ui_only = false;
    let mut ui_path = PathBuf::from("crates/server/locales/ui.json");
    let mut accept_ui = None;
    let mut accept_tag = None;
    let mut settings_path = PathBuf::from("translation.json");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--translate" => translate = true,
            "--translate-only" => {
                translate = true;
                public_only = true;
            }
            "--ui-only" => {
                translate = true;
                ui_only = true;
            }
            "--ui-catalog" => ui_path = args.next().context("--ui-catalog needs a path")?.into(),
            "--accept-ui" => accept_ui = Some(args.next().context("--accept-ui needs a key")?),
            "--accept-tag" => {
                accept_tag = Some(args.next().context("--accept-tag needs a tag ID")?)
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
                    "export [--source PUBLIC_VAULT_ROOT] [--output content] [--translate | --translate-only | --accept ID] [--ui-only | --accept-ui KEY | --accept-tag ID] [--ui-catalog PATH] [--candidates] [--settings translation.json]"
                );
                return Ok(());
            }
            _ => bail!("unknown option: {arg}"),
        }
    }
    if translate || accept.is_some() || accept_ui.is_some() || accept_tag.is_some() {
        let settings = serde_json::from_slice(&std::fs::read(settings_path)?)?;
        if let Some(key) = accept_ui {
            export::accept_catalog_translation(&ui_path, &key, &settings)?;
        } else if let Some(key) = accept_tag {
            export::accept_catalog_translation(&output.join("tags.json"), &key, &settings)?;
        } else if let Some(id) = accept {
            export::accept_translation(&output, &id, &settings)?;
        } else {
            let translator = export::CodexTranslator::default();
            if ui_only {
                let report =
                    export::translate_catalog(&ui_path, &translator, &settings, candidates)?;
                println!("UI: {report:?}");
                return Ok(());
            }
            let report = if public_only {
                export::translate_public(&output, &translator, &settings, candidates)?
            } else {
                export::export_translated(&source, &output, &translator, &settings, candidates)?
            };
            println!(
                "Generated: {}, reused: {}, protected: {:?}",
                report.generated, report.reused, report.protected
            );
            let ui = export::translate_catalog(&ui_path, &translator, &settings, candidates)?;
            println!("UI: {ui:?}");
        }
    } else {
        export::export_japanese(&source, &output)?;
        println!("Japanese public Markdown exported to {}", output.display());
    }
    Ok(())
}
