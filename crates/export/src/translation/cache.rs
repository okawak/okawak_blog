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
        for file in crate::filesystem::all_files(&cache)? {
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
    use super::super::TranslationSettings;
    use super::*;
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
