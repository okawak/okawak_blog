mod report;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use domain::Slug;
use export::TranslationSettings;
use std::{fs, path::PathBuf};

/// Export public Markdown and translate changed article, tag, and UI text.
#[derive(Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
pub(super) struct Cli {
    #[command(flatten)]
    export: ExportArgs,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args)]
struct ExportArgs {
    /// Obsidian directory containing explicitly published notes.
    #[arg(long, default_value = "obsidian/Publish")]
    source: PathBuf,
    /// Public Markdown directory.
    #[arg(long, default_value = "content")]
    output: PathBuf,
    /// UI text catalog.
    #[arg(long, default_value = "crates/server/locales/ui.json")]
    ui_catalog: PathBuf,
    #[command(flatten)]
    settings: SettingsArgs,
}

#[derive(Args)]
struct SettingsArgs {
    /// Translation model, instructions, and glossary.
    #[arg(long, default_value = "translation.json")]
    settings: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Accept one reviewed candidate without reading Obsidian or calling AI.
    Accept {
        #[command(subcommand)]
        target: AcceptTarget,
    },
}

#[derive(Subcommand)]
enum AcceptTarget {
    /// Accept a translated article, home, page, or category candidate.
    Article {
        /// Public content ID.
        id: Slug,
        /// Public Markdown directory.
        #[arg(long, default_value = "content")]
        output: PathBuf,
        #[command(flatten)]
        settings: SettingsArgs,
    },
    /// Accept a translated tag candidate.
    Tag {
        /// Original tag ID used in article metadata.
        id: String,
        /// Public Markdown directory.
        #[arg(long, default_value = "content")]
        output: PathBuf,
        #[command(flatten)]
        settings: SettingsArgs,
    },
    /// Accept a translated UI message candidate.
    Ui {
        /// Message key in the UI catalog.
        key: String,
        /// UI text catalog.
        #[arg(long, default_value = "crates/server/locales/ui.json")]
        ui_catalog: PathBuf,
        #[command(flatten)]
        settings: SettingsArgs,
    },
}

impl SettingsArgs {
    fn load(&self) -> Result<TranslationSettings> {
        let bytes = fs::read(&self.settings)
            .with_context(|| format!("cannot read settings {}", self.settings.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid settings {}", self.settings.display()))
    }
}

impl Cli {
    pub(super) fn run(self) -> Result<()> {
        match self.command {
            None => {
                let args = self.export;
                let settings = args.settings.load()?;
                let translator = export::CodexTranslator::default();
                tracing::info!("content export started");
                let content =
                    export::export_translated(&args.source, &args.output, &translator, &settings)?;
                report::content(&content, &args.output, &args.settings.settings);
                tracing::info!("UI translation started");
                let ui = export::translate_catalog(&args.ui_catalog, &translator, &settings)?;
                report::ui(&ui, &args.ui_catalog, &args.settings.settings);
                tracing::info!("export completed");
            }
            Some(Command::Accept { target }) => match target {
                AcceptTarget::Article {
                    id,
                    output,
                    settings,
                } => {
                    export::accept_translation(&output, &id, &settings.load()?)?;
                    tracing::info!(article_id = %id, "translation candidate accepted");
                }
                AcceptTarget::Tag {
                    id,
                    output,
                    settings,
                } => {
                    export::accept_catalog_translation(
                        &output.join("tags.json"),
                        &id,
                        &settings.load()?,
                    )?;
                    tracing::info!(tag_id = %id, "translation candidate accepted");
                }
                AcceptTarget::Ui {
                    key,
                    ui_catalog,
                    settings,
                } => {
                    export::accept_catalog_translation(&ui_catalog, &key, &settings.load()?)?;
                    tracing::info!(message_key = %key, "translation candidate accepted");
                }
            },
        }
        Ok(())
    }
}
