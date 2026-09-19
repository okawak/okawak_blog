//! Versioned text-unit data shared by content producers and consumers.
use crate::{DomainError, Locale, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabelProvenance {
    pub input_hash: String,
    pub generated_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabelTranslation {
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<LabelProvenance>,
    #[serde(default)]
    pub stale: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabelEntry {
    pub source: String,
    pub context: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translation: Option<LabelTranslation>,
}

impl LabelEntry {
    pub fn value(&self, locale: Locale) -> &str {
        match (&self.translation, locale) {
            (Some(t), Locale::En) if !t.stale => &t.value,
            _ => &self.source,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabelCatalog {
    pub schema_version: u32,
    pub entries: BTreeMap<String, LabelEntry>,
}

impl Default for LabelCatalog {
    fn default() -> Self {
        Self {
            schema_version: 1,
            entries: BTreeMap::new(),
        }
    }
}

impl LabelCatalog {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(DomainError::validation("label schema"));
        }
        for (key, entry) in &self.entries {
            if key.trim().is_empty()
                || entry.source.trim().is_empty()
                || entry.context.trim().is_empty()
                || key.contains(['\0', '\r', '\n'])
                || entry.source.contains(['\0', '\r'])
            {
                return Err(DomainError::validation("label entry"));
            }
            if let Some(t) = &entry.translation
                && (t.value.trim().is_empty()
                    || t.value.contains(['\0', '\r'])
                    || placeholders(&entry.source) != placeholders(&t.value)
                    || t.provenance.as_ref().is_some_and(|p| {
                        [&p.input_hash, &p.generated_hash]
                            .iter()
                            .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
                    }))
            {
                return Err(DomainError::validation("label translation"));
            }
        }
        Ok(())
    }
}

/// Interpolation names, including multiplicity. Values are escaped by the UI renderer.
pub fn placeholders(value: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut rest = value;
    while let Some((_, after)) = rest.split_once('{') {
        if let Some((key, after)) = after.split_once('}') {
            if !key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                result.push(key.to_owned());
            }
            rest = after;
        } else {
            break;
        }
    }
    result.sort();
    result
}

/// Content labels shipped alongside the article index, keyed by stable original tag ID.
pub type TagLabels = BTreeMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;
    fn entry() -> LabelEntry {
        LabelEntry {
            source: "{count}件".into(),
            context: "article count".into(),
            translation: Some(LabelTranslation {
                value: "{count} articles".into(),
                provenance: None,
                stale: false,
            }),
        }
    }
    #[test]
    fn explicit_fallback_and_interpolation_validation() {
        let mut entry = entry();
        assert_eq!(entry.value(Locale::En), "{count} articles");
        entry.translation.as_mut().unwrap().stale = true;
        assert_eq!(entry.value(Locale::En), "{count}件");
        entry.translation.as_mut().unwrap().value = "lost variable".into();
        let catalog = LabelCatalog {
            entries: [("count".into(), entry)].into(),
            ..Default::default()
        };
        assert!(catalog.validate().is_err());
        assert!(
            LabelCatalog {
                schema_version: 2,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
}
