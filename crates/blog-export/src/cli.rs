use anyhow::{Context, Result};
use blog_domain::{Locale, Slug};
use blog_export::{ProtectedContent, TranslationReport, TranslationSettings};
use clap::{Parser, Subcommand};
use std::{fs, path::Path};

/// Export public Markdown and translate changed article, tag, and UI text.
#[derive(Parser)]
#[command(version)]
pub(super) struct Cli {
    /// Translate one language, or choose the language of a review candidate.
    #[arg(long, global = true)]
    target: Option<Locale>,
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
        let ui_catalog = Path::new("crates/blog-server/locales/ui.json");
        let mut settings = load_settings(Path::new("translation.json"))?;
        settings.target = self.target.unwrap_or(Locale::En);
        match self.command {
            None => {
                let translator = blog_export::CodexTranslator::default();
                let settings: Vec<_> = Locale::translations()
                    .filter(|locale| self.target.is_none_or(|chosen| chosen == *locale))
                    .map(|target| TranslationSettings {
                        target,
                        ..settings.clone()
                    })
                    .collect();
                anyhow::ensure!(!settings.is_empty(), "Japanese is the translation source");
                tracing::info!("content export started");
                let reports =
                    blog_export::export_translations(source, output, &translator, &settings)?;
                for (report, setting) in reports.iter().zip(&settings) {
                    report_content(report, setting.target);
                }
                tracing::info!("UI translation started");
                let reports =
                    blog_export::translate_catalog_targets(ui_catalog, &translator, &settings)?;
                for (report, setting) in reports.iter().zip(&settings) {
                    report_ui(report, setting.target);
                }
                tracing::info!("export completed");
            }
            Some(Command::Article { article_id }) => {
                blog_export::accept_article_candidate(output, &article_id, &settings)?;
                tracing::info!(%article_id, "translation candidate accepted");
            }
            Some(Command::Tag { tag }) => {
                blog_export::accept_catalog_candidate(&output.join("tags.json"), &tag, &settings)?;
                tracing::info!(tag_id = %tag, "translation candidate accepted");
            }
            Some(Command::Ui { key }) => {
                blog_export::accept_catalog_candidate(ui_catalog, &key, &settings)?;
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

fn report_content(report: &TranslationReport<ProtectedContent>, target: Locale) {
    report_summary("Content", target, report);
    for item in &report.protected {
        let command = match item {
            ProtectedContent::Article(id) => {
                acceptance_command("accept-article", id.as_str(), target)
            }
            ProtectedContent::Tag(tag) => acceptance_command("accept-tag", tag, target),
        };
        tracing::warn!(%command, "translation candidate needs review");
    }
}

fn report_ui(report: &TranslationReport<String>, target: Locale) {
    report_summary("UI", target, report);
    for key in &report.protected {
        let command = acceptance_command("accept-ui", key, target);
        tracing::warn!(%command, "translation candidate needs review");
    }
}

fn report_summary<T>(scope: &'static str, target: Locale, report: &TranslationReport<T>) {
    tracing::info!(
        scope,
        language = %target,
        generated = report.generated,
        reused = report.reused,
        protected = report.protected.len(),
        "translation completed"
    );
}

fn acceptance_command(subcommand: &str, id: &str, target: Locale) -> String {
    let id = id.replace('\'', "'\"'\"'");
    let command = format!("cargo run -p blog-export -- {subcommand} '{id}'");
    if target == Locale::En {
        command
    } else {
        format!("{command} --target {target}")
    }
}

#[cfg(test)]
mod tests {
    use super::{Locale, acceptance_command};

    #[test]
    fn acceptance_commands_quote_shell_arguments() {
        assert_eq!(
            acceptance_command("accept-article", "article-1", Locale::En),
            "cargo run -p blog-export -- accept-article 'article-1'"
        );
        assert_eq!(
            acceptance_command("accept-tag", "author's note", Locale::En),
            "cargo run -p blog-export -- accept-tag 'author'\"'\"'s note'"
        );
        assert_eq!(
            acceptance_command("accept-ui", "greeting", Locale::En),
            "cargo run -p blog-export -- accept-ui 'greeting'"
        );
    }
}
