//! Shared translation unit and manual-edit guard, independent of articles and UI.
use anyhow::{Result, bail};
use domain::TranslationProvenance;
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Reuse,
    Generate,
    Protect,
}

pub(crate) fn decide(
    input: &str,
    current: Option<&str>,
    provenance: Option<&TranslationProvenance>,
) -> Decision {
    match (current, provenance) {
        (None, _) => Decision::Generate,
        (Some(_), Some(p)) if p.input_hash == input => Decision::Reuse,
        (Some(hash), Some(p)) if p.generated_hash == hash => Decision::Generate,
        _ => Decision::Protect,
    }
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
        Ok(crate::vault::digest(serde_json::to_vec(&(
            "text-fragments-v1",
            self,
        ))?))
    }

    pub(crate) fn validate(&self, result: &Texts) -> Result<()> {
        if !self.texts.keys().eq(result.keys()) {
            bail!("translator changed message keys");
        }
        for (key, source) in &self.texts {
            let value = &result[key];
            if !source.trim().is_empty() && value.trim().is_empty() {
                bail!("translator returned empty text for {key}");
            }
            if value.contains(['\0', '\r'])
                || value.len() > source.len().saturating_mul(20).max(4096)
            {
                bail!("invalid translated text");
            }
            if placeholders(source) != placeholders(value) {
                bail!("translator changed interpolation variables for {key}");
            }
            for (term, translation) in &self.glossary {
                if source.contains(term)
                    && !value.to_lowercase().contains(&translation.to_lowercase())
                {
                    bail!("translation does not contain required glossary term: {translation}");
                }
            }
        }
        Ok(())
    }
}

fn placeholders(value: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut rest = value;
    while let Some((_, after)) = rest.split_once('{') {
        if let Some((key, after)) = after.split_once('}') {
            if !key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                placeholders.push(key.to_owned());
            }
            rest = after;
        } else {
            break;
        }
    }
    placeholders.sort();
    placeholders
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guard_preserves_manual_edits_even_when_input_is_unchanged() {
        let provenance = TranslationProvenance {
            input_hash: "old-input".into(),
            generated_hash: "machine".into(),
            stale: false,
        };
        assert_eq!(
            decide("old-input", Some("manual"), Some(&provenance)),
            Decision::Reuse
        );
        assert_eq!(
            decide("new-input", Some("manual"), Some(&provenance)),
            Decision::Protect
        );
        assert_eq!(
            decide("new-input", Some("machine"), Some(&provenance)),
            Decision::Generate
        );
        assert_eq!(decide("new-input", Some("manual"), None), Decision::Protect);
        assert_eq!(decide("new-input", None, None), Decision::Generate);
    }
}
