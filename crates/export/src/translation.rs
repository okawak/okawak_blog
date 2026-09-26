//! Translation requests and validation shared by articles and catalogs.
mod articles;
mod cache;
mod catalog;
mod codex;
mod fragments;
mod plan;

pub(crate) use articles::{accept_article_candidate, translate_stage};
pub(crate) use catalog::{accept_catalog_candidate, plan_catalog};
pub use codex::CodexTranslator;

use crate::{ExportError, Result};
use domain::{Slug, placeholders};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Texts = BTreeMap<String, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationSettings {
    pub model: String,
    pub instruction: String,
    #[serde(default)]
    pub glossary: Texts,
}

#[derive(Clone, Debug, Serialize)]
pub struct TranslationRequest {
    pub texts: Texts,
    pub context: String,
    pub model: String,
    pub instruction: String,
    pub glossary: Texts,
}

pub trait Translator {
    fn translate(&self, request: &TranslationRequest) -> Result<Texts>;
}

impl TranslationRequest {
    pub(crate) fn new(texts: Texts, context: String, settings: &TranslationSettings) -> Self {
        let glossary = settings
            .glossary
            .iter()
            .filter(|(term, _)| {
                texts.values().any(|v| v.contains(term.as_str())) || context.contains(term.as_str())
            })
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        Self {
            texts,
            context,
            model: settings.model.clone(),
            instruction: settings.instruction.clone(),
            glossary,
        }
    }

    pub(crate) fn fingerprint(&self) -> Result<String> {
        Ok(crate::content::digest(serde_json::to_vec(&(
            "text-fragments-v3",
            self,
        ))?))
    }

    pub(crate) fn validate(&self, result: &Texts) -> Result<()> {
        if !self.texts.keys().eq(result.keys()) {
            return Err(ExportError::invalid_translation(
                "translator changed message keys",
            ));
        }
        for (key, source) in &self.texts {
            let value = &result[key];
            if !source.trim().is_empty() && value.trim().is_empty() {
                return Err(ExportError::invalid_translation(format!(
                    "translator returned empty text for {key}"
                )));
            }
            if value.contains(['\0', '\r'])
                || value.len() > source.len().saturating_mul(20).max(4096)
            {
                return Err(ExportError::invalid_translation("invalid translated text"));
            }
            if placeholders(source) != placeholders(value) {
                return Err(ExportError::invalid_translation(format!(
                    "translator changed interpolation variables for {key}"
                )));
            }
            for (term, translation) in &self.glossary {
                if source.contains(term)
                    && !value.to_lowercase().contains(&translation.to_lowercase())
                {
                    return Err(ExportError::invalid_translation(format!(
                        "translation does not contain required glossary term: {translation}"
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedContent {
    Article(Slug),
    Tag(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationReport<T> {
    pub generated: usize,
    pub reused: usize,
    pub protected: Vec<T>,
}

impl<T> Default for TranslationReport<T> {
    fn default() -> Self {
        Self {
            generated: 0,
            reused: 0,
            protected: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(glossary: Texts) -> TranslationSettings {
        TranslationSettings {
            model: "test".into(),
            instruction: "translate".into(),
            glossary,
        }
    }

    #[test]
    fn request_includes_only_glossary_terms_present_in_text_or_context() {
        let request = TranslationRequest::new(
            [("value".into(), "Rustの記事".into())].into(),
            "navigation".into(),
            &settings(
                [
                    ("Rust".into(), "Rust".into()),
                    ("統計".into(), "Statistics".into()),
                ]
                .into(),
            ),
        );

        assert_eq!(request.glossary, [("Rust".into(), "Rust".into())].into());
    }

    #[test]
    fn response_validation_rejects_each_untrusted_output_invariant() {
        let request = TranslationRequest::new(
            [("value".into(), "Rust {count}".into())].into(),
            String::new(),
            &settings([("Rust".into(), "Rust language".into())].into()),
        );
        let invalid = [
            Texts::from([("other".into(), "Rust language {count}".into())]),
            Texts::from([("value".into(), String::new())]),
            Texts::from([("value".into(), "Rust language".into())]),
            Texts::from([("value".into(), "Other {count}".into())]),
            Texts::from([("value".into(), "Rust language {count}\0".into())]),
        ];

        for response in invalid {
            assert!(matches!(
                request.validate(&response),
                Err(ExportError::InvalidTranslation(_))
            ));
        }
        assert!(
            request
                .validate(&Texts::from([(
                    "value".into(),
                    "Rust language {count}".into()
                )]))
                .is_ok()
        );
    }
}
