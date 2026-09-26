#![warn(unreachable_pub)]

mod content;
mod error;
mod filesystem;
mod output;
mod pipeline;
mod source;
mod translation;

pub use error::{ExportError, Result};
pub use pipeline::{
    accept_article_candidate, accept_catalog_candidate, export_content, translate_catalog,
};
pub use translation::{
    CodexTranslator, ProtectedContent, Texts, TranslationReport, TranslationRequest,
    TranslationSettings, Translator,
};
