use super::{
    ProtectedContent, Texts, TranslationReport, TranslationRequest, TranslationSettings,
    Translator, cache,
    fragments::Fragments,
    plan::{Decision, UpdatePlan, decide, plan_update},
};
use crate::{
    ExportError, Result,
    content::{Document, digest, text_hash},
    filesystem, output,
};
use domain::{Locale, Slug, TranslationProvenance};
use std::{fs, path::Path};

struct ArticlePlan<'a> {
    original: Document,
    current: Option<&'a Document>,
    fragments: Fragments,
    request: TranslationRequest,
    input: String,
    update: UpdatePlan,
}

pub fn translate_public(
    output: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
) -> Result<TranslationReport<ProtectedContent>> {
    filesystem::transaction(output, |stage| {
        output::sync_tags(stage)?;
        translate_stage(stage, output, translator, settings)
    })
}

pub(crate) fn translate_stage(
    stage: &Path,
    cache_root: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
) -> Result<TranslationReport<ProtectedContent>> {
    let mut report = TranslationReport::default();
    let originals = output::read_locale(stage, Locale::Ja)?;
    let english = output::read_locale(stage, Locale::En)?;
    let tags = super::catalog::plan_catalog(&stage.join("tags.json"), settings)?;
    let mut plans = Vec::new();
    for original in originals {
        let fragments = Fragments::extract(&original);
        let request = article_request(fragments.texts.clone(), settings);
        let input = content_input(&request, &original)?;
        let current = english.iter().find(|d| d.meta.id == original.meta.id);
        let current_hash = current.map(text_hash).transpose()?;
        let decision = decide(
            &input,
            current_hash.as_deref(),
            current
                .and_then(|d| d.meta.translation.as_ref())
                .map(|p| (p.input_hash.as_str(), p.generated_hash.as_str())),
        );
        let candidate_path = stage.join(format!(".export-candidates/{}.md", original.meta.id));
        let candidate_decision = if decision != Decision::Reuse && candidate_path.exists() {
            let candidate = Document::parse(&fs::read_to_string(&candidate_path)?)?;
            if candidate.meta.id != original.meta.id || candidate.meta.locale != Locale::En {
                return Err(ExportError::translation_conflict(
                    "candidate identity changed",
                ));
            }
            Some(decide(
                &input,
                Some(&text_hash(&candidate)?),
                candidate
                    .meta
                    .translation
                    .as_ref()
                    .map(|p| (p.input_hash.as_str(), p.generated_hash.as_str())),
            ))
        } else {
            None
        };
        let update = plan_update(decision, candidate_decision, original.meta.id.as_str())?;
        plans.push(ArticlePlan {
            original,
            current,
            fragments,
            request,
            input,
            update,
        });
    }
    // Inspect every candidate before any AI call, cache write or output update.
    fs::create_dir_all(stage.join("en"))?;
    let total = plans.len();
    let ai_requests = plans.iter().try_fold(0, |count, plan| {
        Ok::<_, ExportError>(
            count
                + usize::from(cache::will_call_translator(
                    plan.update,
                    &plan.request,
                    cache_root,
                )?),
        )
    })?;
    tracing::info!(
        scope = "Articles",
        total,
        ai_requests,
        "translation phase started"
    );
    for (
        index,
        ArticlePlan {
            original,
            current,
            fragments,
            request,
            input,
            update,
        },
    ) in plans.into_iter().enumerate()
    {
        tracing::info!(
            scope = "Article",
            current = index + 1,
            total,
            article_id = %original.meta.id,
            action = update.action(),
            "translation item started"
        );
        let path = stage.join(format!("en/{}.md", original.meta.id));
        match update {
            UpdatePlan::Reuse => {
                let mut preserved = current.unwrap().clone();
                preserved.refresh_translation_metadata(&original);
                if let Some(provenance) = &mut preserved.meta.translation {
                    provenance.stale = false;
                }
                fs::write(path, preserved.encode()?)?;
                report.reused += 1;
            }
            UpdatePlan::Generate | UpdatePlan::GenerateCandidate | UpdatePlan::ReuseCandidate => {
                if update.protects_current() {
                    let mut preserved = current.unwrap().clone();
                    if let Some(p) = &mut preserved.meta.translation {
                        p.stale = true;
                    }
                    fs::write(&path, preserved.encode()?)?;
                    report
                        .protected
                        .push(ProtectedContent::Article(original.meta.id.clone()));
                    if update == UpdatePlan::ReuseCandidate {
                        report.reused += 1;
                        continue;
                    }
                }
                let checked = ArticleTranslator {
                    translator,
                    fragments: &fragments,
                    original: &original,
                };
                let result = cache::cached_response(&request, &checked, cache_root)?;
                let mut translated = fragments.apply(&original, &result)?;
                translated.meta.locale = Locale::En;
                translated.meta.translation = Some(TranslationProvenance {
                    input_hash: input,
                    generated_hash: text_hash(&translated)?,
                    stale: false,
                });
                let destination = if update.protects_current() {
                    stage.join(format!(".export-candidates/{}.md", original.meta.id))
                } else {
                    path
                };
                fs::create_dir_all(destination.parent().unwrap())?;
                fs::write(destination, translated.encode()?)?;
                report.generated += 1;
            }
        }
    }
    tracing::info!(scope = "Articles", total, "translation phase completed");
    let tags = tags.apply(cache_root, translator)?;
    report.generated += tags.generated;
    report.reused += tags.reused;
    report
        .protected
        .extend(tags.protected.into_iter().map(ProtectedContent::Tag));
    cache::copy_cache(cache_root, stage)?;
    Ok(report)
}

// Validate reconstruction before a fresh response can enter the reusable cache.
struct ArticleTranslator<'a> {
    translator: &'a dyn Translator,
    fragments: &'a Fragments,
    original: &'a Document,
}

impl Translator for ArticleTranslator<'_> {
    fn translate(&self, request: &TranslationRequest) -> Result<Texts> {
        let result = self.translator.translate(request)?;
        request.validate(&result)?;
        self.fragments.apply(self.original, &result)?.encode()?;
        Ok(result)
    }
}

pub fn accept_translation(output: &Path, id: &Slug, settings: &TranslationSettings) -> Result<()> {
    filesystem::transaction(output, |stage| {
        let original = Document::parse(&fs::read_to_string(stage.join(format!("ja/{id}.md")))?)?;
        let candidate_path = stage.join(format!(".export-candidates/{id}.md"));
        let mut candidate = Document::parse(&fs::read_to_string(&candidate_path)?)?;
        let fragments = Fragments::extract(&original);
        let request = article_request(fragments.texts, settings);
        let provenance = candidate
            .meta
            .translation
            .as_ref()
            .ok_or_else(|| ExportError::translation_conflict("candidate has no provenance"))?;
        if original.meta.id != *id
            || original.meta.locale != Locale::Ja
            || candidate.meta.id != *id
            || candidate.meta.locale != Locale::En
            || provenance.stale
            || provenance.input_hash != content_input(&request, &original)?
        {
            return Err(ExportError::translation_conflict(
                "candidate no longer matches current source/settings",
            ));
        }
        // Candidate prose may have been reviewed manually while management
        // fields changed in Japanese. Those fields always come from the source.
        candidate.refresh_translation_metadata(&original);
        fs::create_dir_all(stage.join("en"))?;
        fs::write(stage.join(format!("en/{id}.md")), candidate.encode()?)?;
        fs::remove_file(candidate_path)?;
        Ok(())
    })
}

fn content_input(request: &TranslationRequest, original: &Document) -> Result<String> {
    Ok(digest(serde_json::to_vec(&(
        // Rebuild generated Markdown when escaping changes, retaining the
        // separate plain-text response cache and protecting manual edits.
        "markdown-reassembly-v1",
        request.fingerprint()?,
        &original.body,
    ))?))
}

fn article_request(texts: Texts, settings: &TranslationSettings) -> TranslationRequest {
    TranslationRequest::new(texts, "Public blog article: ordered Markdown prose fragments. Return plain text only; preserve fragment boundaries.".into(), settings)
}
