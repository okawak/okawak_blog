//! Persist validated responses outside the public-tree transaction for retries.
use super::{Texts, TranslationRequest, Translator, plan::UpdatePlan};
use crate::Result;
use std::path::{Path, PathBuf};

pub(super) fn will_call_translator(
    plan: UpdatePlan,
    request: &TranslationRequest,
    root: &Path,
) -> Result<bool> {
    Ok(plan.requires_response() && !response_cache_path(request, root)?.exists())
}

pub(super) fn cached_response(
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

pub(super) fn copy_cache(root: &Path, stage: &Path) -> Result<()> {
    use std::fs;
    let cache = root.join(".export-candidates/cache");
    if cache.exists() {
        let destination = stage.join(".export-candidates/cache");
        fs::create_dir_all(&destination)?;
        for entry in crate::filesystem::walk(&cache, true) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let dest = destination.join(entry.file_name());
                // Fingerprinted responses are immutable; staging already copied older ones.
                if !dest.exists() {
                    fs::copy(entry.path(), dest)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::TranslationSettings;
    use super::*;

    #[test]
    fn copies_only_new_responses_into_stage() {
        use std::{
            fs,
            time::{Duration, UNIX_EPOCH},
        };

        let root = tempfile::tempdir().unwrap();
        let stage = tempfile::tempdir().unwrap();
        let cache = root.path().join(".export-candidates/cache");
        let staged_cache = stage.path().join(".export-candidates/cache");
        fs::create_dir_all(&cache).unwrap();
        fs::create_dir_all(&staged_cache).unwrap();
        fs::write(cache.join("old.json"), "{}").unwrap();
        fs::write(cache.join("new.json"), r#"{"text":"new"}"#).unwrap();
        let old = staged_cache.join("old.json");
        fs::write(&old, "{}").unwrap();
        let modified = UNIX_EPOCH + Duration::from_secs(1_000_000);
        fs::File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_modified(modified)
            .unwrap();

        copy_cache(root.path(), stage.path()).unwrap();

        assert_eq!(fs::metadata(old).unwrap().modified().unwrap(), modified);
        assert_eq!(
            fs::read(staged_cache.join("new.json")).unwrap(),
            br#"{"text":"new"}"#
        );
    }

    #[test]
    fn progress_ai_count_excludes_reuse_and_cached_responses() {
        let root = tempfile::TempDir::new().unwrap();
        let request = TranslationRequest::new(
            Texts::from([("text".into(), "文章".into())]),
            "test".into(),
            &TranslationSettings {
                model: "test".into(),
                instruction: "translate".into(),
                glossary: Texts::new(),
            },
        );
        assert!(!will_call_translator(UpdatePlan::Reuse, &request, root.path()).unwrap());
        assert!(will_call_translator(UpdatePlan::Generate, &request, root.path()).unwrap());

        let cache = response_cache_path(&request, root.path()).unwrap();
        std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
        std::fs::write(cache, "{}").unwrap();
        assert!(!will_call_translator(UpdatePlan::Generate, &request, root.path()).unwrap());
    }
}
