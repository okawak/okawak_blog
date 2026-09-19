use crate::{markdown, normalize, sync, vault};
use anyhow::{Result, bail};
use domain::Locale;
use std::{collections::HashSet, fs, path::Path};

pub fn export_japanese(source: &Path, output: &Path) -> Result<()> {
    export_with(source, output, |_| Ok(()))
}

pub fn export_translated(
    source: &Path,
    output: &Path,
    translator: &dyn crate::Translator,
    settings: &crate::TranslationSettings,
    candidates: bool,
) -> Result<crate::TranslationReport> {
    let mut report = crate::TranslationReport::default();
    export_with(source, output, |stage| {
        report = crate::translate_content::translate_stage(
            stage, output, translator, settings, candidates,
        )?;
        Ok(())
    })?;
    Ok(report)
}

fn export_with(
    source: &Path,
    output: &Path,
    after_prepare: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        bail!("symlink source is not allowed");
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
                .ok_or_else(|| anyhow::anyhow!("invalid output"))?,
        )
    };
    if output_absolute.starts_with(&source) || source.starts_with(&output_absolute) {
        bail!("public output and private input must be separate trees");
    }
    sync::transaction(output, |stage| {
        let previous = markdown::read_locale(stage, Locale::Ja)?;
        let mut sources = vault::extract(&source, &previous)?;
        let assets = normalize::normalize(&mut sources, &source)?;
        let ids: HashSet<_> = sources.iter().map(|s| s.document.meta.id.clone()).collect();
        fs::create_dir_all(stage.join("ja"))?;
        for locale in Locale::ALL {
            for old in markdown::read_locale(stage, locale)? {
                if !ids.contains(&old.meta.id) {
                    let path = stage.join(format!("{locale}/{}.md", old.meta.id));
                    let archive = stage.join(".export-archive").join(format!(
                        "{locale}-{}-{}.md",
                        old.meta.id,
                        vault::digest(fs::read(&path)?)
                    ));
                    fs::create_dir_all(archive.parent().unwrap())?;
                    fs::rename(path, archive)?;
                }
            }
        }
        for source in sources {
            let english_path = stage.join(format!("en/{}.md", source.document.meta.id));
            if english_path.exists() {
                let mut english = markdown::Document::parse(&fs::read_to_string(&english_path)?)?;
                let mut provenance = english.meta.translation.take();
                if previous
                    .iter()
                    .find(|d| d.meta.id == source.document.meta.id)
                    .map(crate::fragments::text_hash)
                    .transpose()?
                    != Some(crate::fragments::text_hash(&source.document)?)
                    && let Some(provenance) = &mut provenance
                {
                    provenance.stale = true;
                }
                let title = english.meta.title.clone();
                let summary = english.meta.summary.clone();
                english.meta = source.document.meta.clone();
                english.meta.locale = Locale::En;
                english.meta.title = title;
                english.meta.summary = summary;
                english.meta.translation = provenance;
                fs::write(english_path, english.encode()?)?;
            }
            fs::write(
                stage.join(format!("ja/{}.md", source.document.meta.id)),
                source.document.encode()?,
            )?;
        }
        fs::create_dir_all(stage.join("assets"))?;
        let managed_names: HashSet<_> = assets.keys().cloned().collect();
        let existing_english = markdown::read_locale(stage, Locale::En)?;
        for path in markdown::files(&stage.join("assets"))? {
            let name = path.file_name().unwrap().to_string_lossy();
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            if stem.len() == 64
                && stem.bytes().all(|b| b.is_ascii_hexdigit())
                && !managed_names.contains(name.as_ref())
                && !existing_english
                    .iter()
                    .any(|d| d.body.contains(name.as_ref()))
            {
                fs::remove_file(path)?;
            }
        }
        for (name, bytes) in assets {
            fs::write(stage.join("assets").join(name), bytes)?;
        }
        crate::tags::sync(stage)?;
        after_prepare(stage)
    })
}
