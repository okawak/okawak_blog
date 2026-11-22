//! Export, catalog translation and candidate acceptance operations.
use crate::{
    ExportError, Result, filesystem, output,
    translation::{self, ProtectedContent, TranslationReport, TranslationSettings, Translator},
    vault,
};
use blog_domain::{Locale, Slug};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

pub fn export_content(
    source: &Path,
    output: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
) -> Result<TranslationReport<ProtectedContent>> {
    let mut reports =
        export_translations(source, output, translator, std::slice::from_ref(settings))?;
    Ok(reports.pop().expect("single translation target"))
}

/// Reconcile the source once and commit all requested article and tag translations together.
pub fn export_translations(
    source: &Path,
    output: &Path,
    translator: &dyn Translator,
    settings: &[TranslationSettings],
) -> Result<Vec<TranslationReport<ProtectedContent>>> {
    validate_targets(settings)?;
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err(ExportError::invalid_input("symlink source is not allowed"));
    }
    let source = source.canonicalize()?;
    let output_absolute = if output.exists() {
        output.canonicalize()?
    } else {
        let parent = filesystem::parent(output);
        fs::create_dir_all(parent)?;
        parent.canonicalize()?.join(
            output
                .file_name()
                .ok_or_else(|| ExportError::invalid_input("invalid output"))?,
        )
    };
    if output_absolute.starts_with(&source) || source.starts_with(&output_absolute) {
        return Err(ExportError::invalid_input(
            "public output and private input must be separate trees",
        ));
    }
    filesystem::transaction(output, |stage| {
        let previous = output::read_locale(stage, Locale::Ja)?;
        let mut documents = vault::prepare(&source, &previous)?;
        documents.sort_by(|a, b| a.meta.id.as_str().cmp(b.meta.id.as_str()));
        output::reconcile(stage, previous, &documents)?;
        let mut tags = output::prepare_tag_catalog(stage, &documents)?;
        let plans = settings
            .iter()
            .map(|setting| translation::plan_content(stage, &documents, &tags, setting))
            .collect::<Result<Vec<_>>>()?;
        let reports = plans
            .into_iter()
            .map(|plan| plan.apply(stage, &mut tags, output, translator))
            .collect::<Result<_>>()?;
        tags.validate()?;
        output::write_json(&stage.join("tags.json"), &tags)?;
        translation::copy_cache(output, stage)?;
        Ok(reports)
    })
}

pub fn translate_catalog(
    path: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
) -> Result<TranslationReport<String>> {
    let mut reports = translate_catalog_targets(path, translator, std::slice::from_ref(settings))?;
    Ok(reports.pop().expect("single translation target"))
}

pub fn translate_catalog_targets(
    path: &Path,
    translator: &dyn Translator,
    settings: &[TranslationSettings],
) -> Result<Vec<TranslationReport<String>>> {
    validate_targets(settings)?;
    let path = canonical_catalog_path(path)?;
    let root = filesystem::parent(&path);
    filesystem::locked(root, || {
        let mut catalog = output::read_catalog(&path)?;
        let plans = settings
            .iter()
            .map(|s| translation::plan_catalog(&path, &catalog, s))
            .collect::<Result<Vec<_>>>()?;
        let reports = plans
            .into_iter()
            .map(|plan| plan.apply(&mut catalog, root, translator))
            .collect::<Result<_>>()?;
        catalog.validate()?;
        output::write_json(&path, &catalog)?;
        Ok(reports)
    })
}

pub fn accept_article_candidate(
    output: &Path,
    id: &Slug,
    settings: &TranslationSettings,
) -> Result<()> {
    settings.validate_target()?;
    filesystem::transaction(output, |stage| {
        translation::accept_article_candidate(stage, id, settings)
    })
}

pub fn accept_catalog_candidate(
    path: &Path,
    key: &str,
    settings: &TranslationSettings,
) -> Result<()> {
    settings.validate_target()?;
    let path = canonical_catalog_path(path)?;
    filesystem::locked(filesystem::parent(&path), || {
        translation::accept_catalog_candidate(&path, key, settings)
    })
}

fn canonical_catalog_path(path: &Path) -> Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or_else(|| ExportError::invalid_input("catalog needs a file name"))?;
    // Resolve directory aliases for the shared tree lock, but retain the final
    // component so read_catalog() can continue rejecting symlink catalog files.
    Ok(filesystem::parent(path).canonicalize()?.join(name))
}

fn validate_targets(settings: &[TranslationSettings]) -> Result<()> {
    if settings.is_empty() {
        return Err(ExportError::invalid_input(
            "at least one translation target is required",
        ));
    }
    let mut seen = HashSet::new();
    for setting in settings {
        setting.validate_target()?;
        if !seen.insert(setting.target) {
            return Err(ExportError::invalid_input("duplicate translation target"));
        }
    }
    Ok(())
}
