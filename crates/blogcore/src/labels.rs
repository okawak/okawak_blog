//! Versioned text-unit data shared by content producers and consumers.
use crate::{DomainError, Locale, Result, Sha256Digest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabelProvenance {
    pub input_hash: Sha256Digest,
    pub generated_hash: Sha256Digest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LabelTranslation {
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<LabelProvenance>,
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
        self.validate_structure()?;
        for entry in self.entries.values() {
            if let Some(translation) = &entry.translation
                && !translation.stale
                && placeholders(&entry.source) != placeholders(&translation.value)
            {
                return Err(DomainError::validation("label translation placeholders"));
            }
        }
        Ok(())
    }

    /// Validate editable data before reconciling translations with a changed source.
    /// Publishing must use `validate`, which also checks each active translation.
    pub fn validate_structure(&self) -> Result<()> {
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
                && (t.value.trim().is_empty() || t.value.contains(['\0', '\r']))
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
    fn label_translation_requires_explicit_freshness() {
        let mut value = serde_json::to_value(entry()).unwrap();
        value["translation"]
            .as_object_mut()
            .unwrap()
            .remove("stale");
        assert!(serde_json::from_value::<LabelEntry>(value.clone()).is_err());
        for stale in [true, false] {
            value["translation"]["stale"] = stale.into();
            let entry = serde_json::from_value::<LabelEntry>(value.clone()).unwrap();
            assert_eq!(entry.translation.unwrap().stale, stale);
        }
    }

    #[test]
    fn label_provenance_rejects_invalid_hashes_on_read() {
        for field in ["input_hash", "generated_hash"] {
            let mut value = serde_json::json!({
                "input_hash": "a".repeat(64),
                "generated_hash": "b".repeat(64),
            });
            value[field] = "invalid".into();
            assert!(serde_json::from_value::<LabelProvenance>(value).is_err());
        }
    }
    #[test]
    fn explicit_fallback_and_interpolation_validation() {
        let mut entry = entry();
        assert_eq!(entry.value(Locale::En), "{count} articles");
        entry.translation.as_mut().unwrap().stale = true;
        assert_eq!(entry.value(Locale::En), "{count}件");
        entry.translation.as_mut().unwrap().value = "lost variable".into();
        let mut catalog = LabelCatalog {
            entries: [("count".into(), entry)].into(),
            ..Default::default()
        };
        // Stale text is preserved for editing while rendering falls back to source.
        assert!(catalog.validate().is_ok());
        catalog
            .entries
            .get_mut("count")
            .unwrap()
            .translation
            .as_mut()
            .unwrap()
            .stale = false;
        assert!(catalog.validate_structure().is_ok());
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
