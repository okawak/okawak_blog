mod report;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use domain::Slug;
use export::TranslationSettings;
use std::{fs, path::Path};

/// Export public Markdown and translate changed article, tag, and UI text.
#[derive(Parser)]
#[command(version)]
pub(super) struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Accept an article candidate.
    #[command(name = "accept-article")]
    Article {
        /// Filename in content/.export-candidates/, without .md (e.g. 012345abcdef).
        article_id: Slug,
    },
    /// Accept a tag candidate.
    #[command(name = "accept-tag")]
    Tag {
        /// Original tag name in tags.json (e.g. Rust or 統計).
        tag: String,
    },
    /// Accept a UI text candidate.
    #[command(name = "accept-ui")]
    Ui {
        /// Key in ui.json entries (e.g. filter.label).
        key: String,
    },
}

impl Cli {
    pub(super) fn run(self) -> Result<()> {
        let source = Path::new("obsidian/Publish");
        let output = Path::new("content");
        let ui_catalog = Path::new("crates/server/locales/ui.json");
        let settings = load_settings(Path::new("translation.json"))?;
        match self.command {
            None => {
                let translator = export::CodexTranslator::default();
                tracing::info!("content export started");
                let content = export::export_translated(source, output, &translator, &settings)?;
                report::content(&content);
                tracing::info!("UI translation started");
                let ui = export::translate_catalog(ui_catalog, &translator, &settings)?;
                report::ui(&ui);
                tracing::info!("export completed");
            }
            Some(Command::Article { article_id }) => {
                export::accept_translation(output, &article_id, &settings)?;
                tracing::info!(%article_id, "translation candidate accepted");
            }
            Some(Command::Tag { tag }) => {
                export::accept_catalog_translation(&output.join("tags.json"), &tag, &settings)?;
                tracing::info!(tag_id = %tag, "translation candidate accepted");
            }
            Some(Command::Ui { key }) => {
                export::accept_catalog_translation(ui_catalog, &key, &settings)?;
                tracing::info!(message_key = %key, "translation candidate accepted");
            }
        }
        Ok(())
    }
}

fn load_settings(path: &Path) -> Result<TranslationSettings> {
    let bytes =
        fs::read(path).with_context(|| format!("cannot read settings {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("invalid settings {}", path.display()))
}
