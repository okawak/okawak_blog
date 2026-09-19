//! File adapter for versioned text catalogs. UI keys and their meaning belong to server.
use crate::{
    TranslationReport, sync,
    translation::{Decision, TranslationRequest, TranslationSettings, Translator, decide},
};
use anyhow::{Context, Result, bail};
use domain::{LabelCatalog, LabelProvenance, LabelTranslation};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    key: String,
    source: String,
    context: String,
    translation: LabelTranslation,
}

pub fn translate_catalog(
    path: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
    candidates: bool,
) -> Result<TranslationReport> {
    let path = std::path::absolute(path)?;
    let root = parent(&path);
    sync::locked(root, || {
        translate_catalog_stage(&path, root, translator, settings, candidates)
    })
}

pub(crate) fn translate_catalog_stage(
    path: &Path,
    cache_root: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
    candidates: bool,
) -> Result<TranslationReport> {
    let mut catalog = read(path)?;
    let mut report = TranslationReport::default();
    for (key, entry) in &mut catalog.entries {
        let request = request(entry, settings);
        let input = request.fingerprint()?;
        let current = entry
            .translation
            .as_ref()
            .map(|t| crate::vault::digest(&t.value));
        let decision = decide(
            &input,
            current.as_deref(),
            entry
                .translation
                .as_ref()
                .and_then(|t| t.provenance.as_ref())
                .map(|p| (p.input_hash.as_str(), p.generated_hash.as_str())),
        );
        match decision {
            Decision::Reuse => {
                entry.translation.as_mut().unwrap().stale = false;
                report.reused += 1;
            }
            Decision::Protect | Decision::Generate => {
                if decision == Decision::Protect {
                    if entry.translation.as_ref().unwrap().provenance.is_some() {
                        entry.translation.as_mut().unwrap().stale = true;
                    }
                    report.protected.push(key.clone());
                    if !candidates {
                        continue;
                    }
                }
                let response =
                    crate::translation::cached_response(&request, translator, cache_root)?;
                let value = response["value"].clone();
                let translation = LabelTranslation {
                    provenance: Some(LabelProvenance {
                        input_hash: input,
                        generated_hash: crate::vault::digest(&value),
                    }),
                    value,
                    stale: false,
                };
                if decision == Decision::Protect {
                    let candidate = candidate_path(path, key);
                    fs::create_dir_all(candidate.parent().unwrap())?;
                    write(
                        &candidate,
                        &Candidate {
                            key: key.clone(),
                            source: entry.source.clone(),
                            context: entry.context.clone(),
                            translation,
                        },
                    )?;
                } else {
                    entry.translation = Some(translation);
                }
                report.generated += 1;
            }
        }
    }
    catalog.validate()?;
    write(path, &catalog)?;
    Ok(report)
}

pub fn accept_catalog_translation(
    path: &Path,
    key: &str,
    settings: &TranslationSettings,
) -> Result<()> {
    let path = std::path::absolute(path)?;
    sync::locked(parent(&path), || {
        let mut catalog = read(&path)?;
        let entry = catalog
            .entries
            .get_mut(key)
            .context("unknown catalog key")?;
        let candidate_path = candidate_path(&path, key);
        let candidate: Candidate = serde_json::from_slice(&fs::read(&candidate_path)?)?;
        if candidate.key != key
            || candidate.source != entry.source
            || candidate.context != entry.context
        {
            bail!("catalog candidate identity changed");
        }
        let candidate = candidate.translation;
        let request = request(entry, settings);
        if candidate
            .provenance
            .as_ref()
            .context("candidate provenance missing")?
            .input_hash
            != request.fingerprint()?
            || candidate.stale
        {
            bail!("catalog candidate no longer matches current source/settings");
        }
        request.validate(&[("value".into(), candidate.value.clone())].into())?;
        entry.translation = Some(candidate);
        catalog.validate()?;
        write(&path, &catalog)?;
        fs::remove_file(candidate_path)?;
        Ok(())
    })
}

pub(crate) fn read(path: &Path) -> Result<LabelCatalog> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        bail!("catalog must be a regular file");
    }
    let catalog: LabelCatalog = serde_json::from_slice(&fs::read(path)?)?;
    catalog.validate()?;
    Ok(catalog)
}

pub(crate) fn write(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent(path))?;
    temporary.write_all(&bytes)?;
    temporary.persist(path)?;
    Ok(())
}

fn request(entry: &domain::LabelEntry, settings: &TranslationSettings) -> TranslationRequest {
    TranslationRequest::new(
        [("value".into(), entry.source.clone())].into(),
        entry.context.clone(),
        settings,
    )
}
fn candidate_path(path: &Path, key: &str) -> PathBuf {
    let identity = format!("{}:{key}", path.file_name().unwrap().to_string_lossy());
    parent(path)
        .join(".export-candidates/catalog")
        .join(format!("{}.json", crate::vault::digest(identity)))
}
fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}
