//! Shared translation unit and manual-edit guard, independent of articles and UI.
use crate::{ExportError, Result};
use domain::placeholders;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

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

impl Decision {
    pub(crate) fn action(&self, candidate: Option<&Self>) -> &'static str {
        match self {
            Self::Reuse => "reuse",
            Self::Generate => "generate",
            Self::Protect if candidate == Some(&Self::Reuse) => "reuse candidate",
            Self::Protect => "generate candidate",
        }
    }

    fn requires_response(&self, candidate: Option<&Self>) -> bool {
        !matches!(self, Self::Reuse) && candidate != Some(&Self::Reuse)
    }
}

pub(crate) fn will_call_translator(
    decision: &Decision,
    candidate: Option<&Decision>,
    request: &TranslationRequest,
    root: &Path,
) -> Result<bool> {
    Ok(decision.requires_response(candidate) && !response_cache_path(request, root)?.exists())
}

pub(crate) fn decide(
    input: &str,
    current: Option<&str>,
    provenance: Option<(&str, &str)>,
) -> Decision {
    match (current, provenance) {
        (None, _) => Decision::Generate,
        (Some(_), Some((previous_input, _))) if previous_input == input => Decision::Reuse,
        (Some(hash), Some((_, generated_hash))) if generated_hash == hash => Decision::Generate,
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

pub(crate) fn cached_response(
    request: &TranslationRequest,
    translator: &dyn Translator,
    root: &Path,
) -> Result<Texts> {
    use std::{fs, io::Write};
    let cache = response_cache_path(request, root)?;
    let result = if cache.exists() {
        tracing::info!(
            text_count = request.texts.len(),
            "cached AI translation reused"
        );
        serde_json::from_slice(&fs::read(&cache)?)?
    } else {
        let result = translator.translate(request)?;
        request.validate(&result)?;
        fs::create_dir_all(cache.parent().unwrap())?;
        let mut temporary = tempfile::NamedTempFile::new_in(cache.parent().unwrap())?;
        temporary.write_all(&serde_json::to_vec_pretty(&result)?)?;
        temporary.persist(cache)?;
        result
    };
    request.validate(&result)?;
    Ok(result)
}

fn response_cache_path(request: &TranslationRequest, root: &Path) -> Result<PathBuf> {
    Ok(root
        .join(".export-candidates/cache")
        .join(format!("{}.json", request.fingerprint()?)))
}

pub(crate) fn copy_cache(root: &std::path::Path, stage: &std::path::Path) -> Result<()> {
    use std::fs;
    let cache = root.join(".export-candidates/cache");
    if cache.exists() {
        for file in crate::markdown::all_files(&cache)? {
            let dest = stage
                .join(".export-candidates/cache")
                .join(file.file_name().unwrap());
            fs::create_dir_all(dest.parent().unwrap())?;
            fs::copy(file, dest)?;
        }
    }
    Ok(())
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
    fn guard_preserves_manual_edits_even_when_input_is_unchanged() {
        let provenance = ("old-input", "machine");
        assert_eq!(
            decide("old-input", Some("manual"), Some(provenance)),
            Decision::Reuse
        );
        assert_eq!(
            decide("new-input", Some("manual"), Some(provenance)),
            Decision::Protect
        );
        assert_eq!(
            decide("new-input", Some("machine"), Some(provenance)),
            Decision::Generate
        );
        assert_eq!(decide("new-input", Some("manual"), None), Decision::Protect);
        assert_eq!(decide("new-input", None, None), Decision::Generate);
    }

    #[test]
    fn progress_action_describes_each_decision() {
        assert_eq!(Decision::Reuse.action(None), "reuse");
        assert_eq!(Decision::Generate.action(None), "generate");
        assert_eq!(Decision::Protect.action(None), "generate candidate");
        assert_eq!(
            Decision::Protect.action(Some(&Decision::Reuse)),
            "reuse candidate"
        );
    }

    #[test]
    fn progress_ai_count_excludes_reuse_and_cached_responses() {
        let root = tempfile::TempDir::new().unwrap();
        let request = TranslationRequest::new(
            Texts::from([("text".into(), "文章".into())]),
            "test".into(),
            &settings(Texts::new()),
        );
        assert!(!will_call_translator(&Decision::Reuse, None, &request, root.path()).unwrap());
        assert!(will_call_translator(&Decision::Generate, None, &request, root.path()).unwrap());

        let cache = response_cache_path(&request, root.path()).unwrap();
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(cache, "{}").unwrap();
        assert!(!will_call_translator(&Decision::Generate, None, &request, root.path()).unwrap());
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
