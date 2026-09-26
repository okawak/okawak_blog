//! File adapter for versioned text catalogs. UI keys and their meaning belong to server.
use super::{
    TranslationReport, TranslationRequest, TranslationSettings, Translator, cache,
    plan::{Decision, UpdatePlan, decide, plan_update},
};
use crate::{ExportError, Result, content::digest, filesystem, output};
use domain::{LabelCatalog, LabelProvenance, LabelTranslation, Sha256Digest};
use std::{
    fs,
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

pub(crate) struct CatalogPlan {
    path: PathBuf,
    catalog: LabelCatalog,
    entries: Vec<EntryPlan>,
}

struct EntryPlan {
    key: String,
    request: TranslationRequest,
    input: Sha256Digest,
    update: UpdatePlan,
}

pub(crate) fn plan_catalog(path: &Path, settings: &TranslationSettings) -> Result<CatalogPlan> {
    let catalog = output::read_catalog(path)?;
    let mut entries = Vec::new();
    for (key, entry) in &catalog.entries {
        let request = request(entry, settings);
        let input = request.fingerprint()?;
        let current = entry.translation.as_ref().map(|t| digest(&t.value));
        let decision = decide(
            input.as_str(),
            current.as_ref().map(Sha256Digest::as_str),
            entry
                .translation
                .as_ref()
                .and_then(|t| t.provenance.as_ref())
                .map(|p| (p.input_hash.as_str(), p.generated_hash.as_str())),
        );
        let existing_candidate_path = candidate_path(path, key);
        let candidate_decision = if decision != Decision::Reuse && existing_candidate_path.exists()
        {
            let candidate: Candidate =
                serde_json::from_slice(&fs::read(&existing_candidate_path)?)?;
            if candidate.key != *key {
                return Err(ExportError::translation_conflict(
                    "catalog candidate identity changed",
                ));
            }
            Some(decide(
                input.as_str(),
                Some(digest(&candidate.translation.value).as_str()),
                candidate
                    .translation
                    .provenance
                    .as_ref()
                    .map(|p| (p.input_hash.as_str(), p.generated_hash.as_str())),
            ))
        } else {
            None
        };
        let update = plan_update(decision, candidate_decision, key)?;
        entries.push(EntryPlan {
            key: key.clone(),
            request,
            input,
            update,
        });
    }
    Ok(CatalogPlan {
        path: path.to_owned(),
        catalog,
        entries,
    })
}

impl CatalogPlan {
    pub(crate) fn apply(
        self,
        cache_root: &Path,
        translator: &dyn Translator,
    ) -> Result<TranslationReport<String>> {
        let Self {
            path,
            mut catalog,
            entries,
        } = self;
        let mut report = TranslationReport::default();
        let scope = if path.file_name().and_then(|name| name.to_str()) == Some("tags.json") {
            "Tags"
        } else {
            "UI"
        };
        let total = entries.len();
        let ai_requests = entries.iter().try_fold(0, |count, entry| {
            Ok::<_, ExportError>(
                count
                    + usize::from(cache::will_call_translator(
                        entry.update,
                        &entry.request,
                        cache_root,
                    )?),
            )
        })?;
        tracing::info!(scope, total, ai_requests, "translation phase started");
        for (
            index,
            EntryPlan {
                key,
                request,
                input,
                update,
            },
        ) in entries.into_iter().enumerate()
        {
            tracing::info!(
                scope,
                current = index + 1,
                total,
                item = %key,
                action = update.action(),
                "translation item started"
            );
            let entry = catalog
                .entries
                .get_mut(&key)
                .expect("planned catalog entry");
            match update {
                UpdatePlan::Reuse => {
                    entry.translation.as_mut().unwrap().stale = false;
                    report.reused += 1;
                }
                UpdatePlan::Generate
                | UpdatePlan::GenerateCandidate
                | UpdatePlan::ReuseCandidate => {
                    if update.protects_current() {
                        if entry.translation.as_ref().unwrap().provenance.is_some() {
                            entry.translation.as_mut().unwrap().stale = true;
                        }
                        report.protected.push(key.clone());
                        if update == UpdatePlan::ReuseCandidate {
                            report.reused += 1;
                            continue;
                        }
                    }
                    let response = cache::cached_response(&request, translator, cache_root)?;
                    let value = response["value"].clone();
                    let translation = LabelTranslation {
                        provenance: Some(LabelProvenance {
                            input_hash: input,
                            generated_hash: digest(&value),
                        }),
                        value,
                        stale: false,
                    };
                    if update.protects_current() {
                        let candidate = candidate_path(&path, &key);
                        fs::create_dir_all(candidate.parent().unwrap())?;
                        output::write_json(
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
        output::write_json(&path, &catalog)?;
        tracing::info!(scope, total, "translation phase completed");
        Ok(report)
    }
}

pub(crate) fn accept_catalog_candidate(
    path: &Path,
    key: &str,
    settings: &TranslationSettings,
) -> Result<()> {
    let mut catalog = output::read_catalog(path)?;
    let entry = catalog
        .entries
        .get_mut(key)
        .ok_or_else(|| ExportError::invalid_input("unknown catalog key"))?;
    let candidate_path = candidate_path(path, key);
    let candidate: Candidate = serde_json::from_slice(&fs::read(&candidate_path)?)?;
    if candidate.key != key
        || candidate.source != entry.source
        || candidate.context != entry.context
    {
        return Err(ExportError::translation_conflict(
            "catalog candidate identity changed",
        ));
    }
    let candidate = candidate.translation;
    let request = request(entry, settings);
    if candidate
        .provenance
        .as_ref()
        .ok_or_else(|| ExportError::translation_conflict("candidate provenance missing"))?
        .input_hash
        != request.fingerprint()?
        || candidate.stale
    {
        return Err(ExportError::translation_conflict(
            "catalog candidate no longer matches current source/settings",
        ));
    }
    request.validate(&[("value".into(), candidate.value.clone())].into())?;
    entry.translation = Some(candidate);
    catalog.validate()?;
    output::write_json(path, &catalog)?;
    fs::remove_file(candidate_path)?;
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
    filesystem::parent(path)
        .join(".export-candidates/catalog")
        .join(format!("{}.json", digest(identity)))
}
