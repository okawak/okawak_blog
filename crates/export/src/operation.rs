//! Export, catalog translation and candidate acceptance operations.
use crate::{
    ExportError, Result, filesystem, output,
    translation::{self, ProtectedContent, TranslationReport, TranslationSettings, Translator},
    vault,
};
use domain::{Locale, Slug};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn export_content(
    source: &Path,
    output: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
) -> Result<TranslationReport<ProtectedContent>> {
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
        let documents = vault::prepare(&source, &previous)?;
        output::reconcile(stage, &previous, &documents)?;
        output::sync_tags(stage)?;
        translation::translate_stage(stage, output, translator, settings)
    })
}

pub fn translate_catalog(
    path: &Path,
    translator: &dyn Translator,
    settings: &TranslationSettings,
) -> Result<TranslationReport<String>> {
    let path = canonical_catalog_path(path)?;
    let root = filesystem::parent(&path);
    filesystem::locked(root, || {
        translation::plan_catalog(&path, settings)?.apply(root, translator)
    })
}

pub fn accept_article_candidate(
    output: &Path,
    id: &Slug,
    settings: &TranslationSettings,
) -> Result<()> {
    filesystem::transaction(output, |stage| {
        translation::accept_article_candidate(stage, id, settings)
    })
}

pub fn accept_catalog_candidate(
    path: &Path,
    key: &str,
    settings: &TranslationSettings,
) -> Result<()> {
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
