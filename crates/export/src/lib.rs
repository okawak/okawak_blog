#![warn(unreachable_pub)]

mod codex;
mod fragments;
mod markdown;
mod normalize;
mod pipeline;
mod report;
mod sync;
mod translate_content;
mod translation;
mod vault;

pub use codex::CodexTranslator;
pub use pipeline::{export_japanese, export_translated};
pub use report::{ProtectedContent, TranslationReport};
pub use translate_content::{accept_translation, translate_public};
pub use translation::{Texts, TranslationRequest, TranslationSettings, Translator};

mod catalog;
pub use catalog::{accept_catalog_translation, translate_catalog};

mod tags;
