//! File adapter for versioned text catalogs. UI keys and their meaning belong to server.
use super::{
    TranslationReport, TranslationRequest, TranslationSettings, Translator, cache,
    plan::{Decision, UpdatePlan, decide, plan_update},
};
use crate::{ExportError, Result, content::digest, filesystem, output};
use blog_domain::{LabelCatalog, LabelProvenance, LabelTranslation, Sha256Digest};
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
    target: blog_domain::Locale,
    path: PathBuf,
    entries: Vec<EntryPlan>,
}

struct EntryPlan {
    key: String,
    request: TranslationRequest,
    input: Sha256Digest,
    update: UpdatePlan,
}

pub(crate) fn plan_catalog(
    path: &Path,
    catalog: &LabelCatalog,
    settings: &TranslationSettings,
) -> Result<CatalogPlan> {
    let mut entries = Vec::new();
    for (key, entry) in &catalog.entries {
        let request = request(entry, settings);
        let input = request.fingerprint()?;
        let decision = label_decision(&input, entry.translations.get(&settings.target));
        let existing_candidate_path = candidate_path(path, key, settings.target);
        let candidate_decision = if decision != Decision::Reuse && existing_candidate_path.exists()
        {
            let candidate: Candidate =
                serde_json::from_slice(&fs::read(&existing_candidate_path)?)?;
            if candidate.key != *key {
                return Err(ExportError::translation_conflict(
                    "catalog candidate identity changed",
                ));
            }
            Some(label_decision(&input, Some(&candidate.translation)))
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
        target: settings.target,
        path: path.to_owned(),
        entries,
    })
}

impl CatalogPlan {
    pub(crate) fn apply(
        self,
        catalog: &mut LabelCatalog,
        cache_root: &Path,
        translator: &dyn Translator,
    ) -> Result<TranslationReport<String>> {
        let Self {
            target,
            path,
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
                    entry.translations.get_mut(&target).unwrap().stale = false;
                    report.reused += 1;
                }
                UpdatePlan::Generate
                | UpdatePlan::GenerateCandidate
                | UpdatePlan::ReuseCandidate => {
                    if update.protects_current() {
                        let translation = entry.translations.get_mut(&target).unwrap();
                        if translation.provenance.is_some() {
                            translation.stale = true;
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
                        let candidate = candidate_path(&path, &key, target);
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
                        entry.translations.insert(target, translation);
                    }
                    report.generated += 1;
                }
            }
        }
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
    let candidate_path = candidate_path(path, key, settings.target);
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
    entry.translations.insert(settings.target, candidate);
    catalog.validate()?;
    output::write_json(path, &catalog)?;
    fs::remove_file(candidate_path)?;
    Ok(())
}

fn label_decision(input: &Sha256Digest, translation: Option<&LabelTranslation>) -> Decision {
    let hash = translation.map(|t| digest(&t.value));
    decide(
        input.as_str(),
        hash.as_ref().map(Sha256Digest::as_str),
        translation
            .and_then(|t| t.provenance.as_ref())
            .map(|p| (p.input_hash.as_str(), p.generated_hash.as_str())),
    )
}

fn request(entry: &blog_domain::LabelEntry, settings: &TranslationSettings) -> TranslationRequest {
    TranslationRequest::new(
        [("value".into(), entry.source.clone())].into(),
        entry.context.clone(),
        settings,
    )
}
fn candidate_path(path: &Path, key: &str, target: blog_domain::Locale) -> PathBuf {
    let identity = format!("{}:{key}", path.file_name().unwrap().to_string_lossy());
    super::candidate_dir(filesystem::parent(path), target)
        .join("catalog")
        .join(format!("{}.json", digest(identity)))
}
