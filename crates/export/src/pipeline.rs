use crate::{
    ExportError, Result, filesystem, output, source,
    translation::{ProtectedContent, TranslationReport},
};
use domain::Locale;
use std::{fs, path::Path};

pub fn export_translated(
    source: &Path,
    output: &Path,
    translator: &dyn crate::Translator,
    settings: &crate::TranslationSettings,
) -> Result<TranslationReport<ProtectedContent>> {
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err(ExportError::invalid_input("symlink source is not allowed"));
    }
    let source = source.canonicalize()?;
    let output_absolute = if output.exists() {
        output.canonicalize()?
    } else {
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
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
        let prepared = source::prepare(&source, &previous)?;
        output::reconcile(stage, &previous, &prepared.documents, &prepared.assets)?;
        output::sync_tags(stage)?;
        crate::translation::translate_stage(stage, output, translator, settings)
    })
}
