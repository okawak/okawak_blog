use crate::{
    fragments::{Fragments, text_hash},
    markdown::{self, Document},
    sync,
    translation::{Decision, TranslationRequest, TranslationSettings, Translator, decide},
};
use anyhow::{Context, Result, bail};
use domain::{Locale, Slug, TranslationProvenance};
use std::{fs, path::Path};

#[derive(Default, Debug)]
pub struct TranslationReport {
    pub generated: usize,
    pub reused: usize,
    pub protected: Vec<String>,
}

pub fn translate_public(
    output: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
    candidates: bool,
) -> Result<TranslationReport> {
    let mut report = TranslationReport::default();
    sync::transaction(output, |stage| {
        report = translate_stage(stage, output, translator, settings, candidates)?;
        Ok(())
    })?;
    Ok(report)
}

pub(crate) fn translate_stage(
    stage: &Path,
    cache_root: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
    candidates: bool,
) -> Result<TranslationReport> {
    let mut report = TranslationReport::default();
    let originals = markdown::read_locale(stage, Locale::Ja)?;
    let english = markdown::read_locale(stage, Locale::En)?;
    fs::create_dir_all(stage.join("en"))?;
    for original in originals {
        let fragments = Fragments::extract(&original);
        let request = TranslationRequest::new(fragments.texts.clone(), "Public blog article: ordered Markdown prose fragments. Return plain text only; preserve fragment boundaries.".into(), settings);
        let input = content_input(&request, &original)?;
        let current = english.iter().find(|d| d.meta.id == original.meta.id);
        let current_hash = current.map(text_hash).transpose()?;
        let decision = decide(
            &input,
            current_hash.as_deref(),
            current.and_then(|d| d.meta.translation.as_ref()),
        );
        let path = stage.join(format!("en/{}.md", original.meta.id));
        match decision {
            Decision::Reuse => {
                let mut preserved = current.unwrap().clone();
                // Refresh non-translatable metadata without changing edited prose.
                let provenance = preserved.meta.translation.take();
                let title = preserved.meta.title.clone();
                let summary = preserved.meta.summary.clone();
                preserved.meta = original.meta.clone();
                preserved.meta.locale = Locale::En;
                preserved.meta.title = title;
                preserved.meta.summary = summary;
                preserved.meta.translation = provenance.map(|mut p| {
                    p.stale = false;
                    p
                });
                fs::write(path, preserved.encode()?)?;
                report.reused += 1;
            }
            Decision::Protect | Decision::Generate => {
                if decision == Decision::Protect {
                    let mut preserved = current.unwrap().clone();
                    if let Some(p) = &mut preserved.meta.translation {
                        p.stale = true;
                    }
                    fs::write(&path, preserved.encode()?)?;
                    report.protected.push(original.meta.id.to_string());
                    if !candidates {
                        continue;
                    }
                }
                let checked = ArticleTranslator {
                    translator,
                    fragments: &fragments,
                    original: &original,
                };
                let translator: &dyn Translator = &checked;
                let cache = cache_root
                    .join(".export-candidates/cache")
                    .join(format!("{}.json", request.fingerprint()?));
                let result = if cache.exists() {
                    serde_json::from_slice(&fs::read(&cache)?)?
                } else {
                    let result = translator.translate(&request)?;
                    request.validate(&result)?;
                    fs::create_dir_all(cache.parent().unwrap())?;
                    let mut temporary = tempfile::NamedTempFile::new_in(cache.parent().unwrap())?;
                    use std::io::Write;
                    temporary.write_all(&serde_json::to_vec_pretty(&result)?)?;
                    temporary.persist(&cache)?;
                    result
                };
                request.validate(&result)?;
                let mut translated = fragments.apply(&original, &result)?;
                translated.meta.locale = Locale::En;
                translated.meta.translation = Some(TranslationProvenance {
                    input_hash: input,
                    generated_hash: text_hash(&translated)?,
                    stale: false,
                });
                let destination = if decision == Decision::Protect {
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
    // A failed run may leave successful unit responses in the ignored cache;
    // copy them into the completed transaction so a retry never loses them.
    let cache = cache_root.join(".export-candidates/cache");
    if cache.exists() {
        for file in markdown::all_files(&cache)? {
            let dest = stage
                .join(".export-candidates/cache")
                .join(file.file_name().unwrap());
            fs::create_dir_all(dest.parent().unwrap())?;
            fs::copy(file, dest)?;
        }
    }
    Ok(report)
}

// Validate reconstruction before a fresh response can enter the reusable cache.
struct ArticleTranslator<'a> {
    translator: &'a dyn Translator,
    fragments: &'a Fragments,
    original: &'a Document,
}

impl Translator for ArticleTranslator<'_> {
    fn translate(&self, request: &TranslationRequest) -> Result<crate::translation::Texts> {
        let result = self.translator.translate(request)?;
        request.validate(&result)?;
        self.fragments.apply(self.original, &result)?.encode()?;
        Ok(result)
    }
}

pub fn accept_translation(output: &Path, id: &Slug, settings: &TranslationSettings) -> Result<()> {
    sync::transaction(output, |stage| {
        let original = Document::parse(&fs::read_to_string(stage.join(format!("ja/{id}.md")))?)?;
        let candidate_path = stage.join(format!(".export-candidates/{id}.md"));
        let candidate = Document::parse(&fs::read_to_string(&candidate_path)?)?;
        let fragments = Fragments::extract(&original);
        let request = TranslationRequest::new(fragments.texts, "Public blog article: ordered Markdown prose fragments. Return plain text only; preserve fragment boundaries.".into(), settings);
        let provenance = candidate
            .meta
            .translation
            .as_ref()
            .context("candidate has no provenance")?;
        if candidate.meta.id != *id
            || candidate.meta.locale != Locale::En
            || provenance.input_hash != content_input(&request, &original)?
        {
            bail!("candidate no longer matches current source/settings");
        }
        fs::create_dir_all(stage.join("en"))?;
        fs::rename(candidate_path, stage.join(format!("en/{id}.md")))?;
        Ok(())
    })
}

fn content_input(request: &TranslationRequest, original: &Document) -> Result<String> {
    Ok(crate::vault::digest(serde_json::to_vec(&(
        request.fingerprint()?,
        &original.body,
    ))?))
}
