//! Public Markdown file adapter and reconciliation of export-managed output.
use crate::{ExportError, Result, content::Document, filesystem};
use domain::{LabelCatalog, LabelEntry, Locale, Slug};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    io::Write,
    path::Path,
};

pub(crate) fn read_locale(root: &Path, locale: domain::Locale) -> Result<Vec<Document>> {
    let root = root.join(locale.as_str());
    if !root.exists() {
        return Ok(Vec::new());
    }
    filesystem::files(&root)?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| {
            let doc = Document::parse(&fs::read_to_string(&p)?)?;
            if doc.meta.locale != locale
                || p.file_stem().and_then(|s| s.to_str()) != Some(doc.meta.id.as_str())
            {
                return Err(ExportError::invalid_input(
                    "public filename/locale must match its metadata",
                ));
            }
            Ok(doc)
        })
        .collect()
}

pub(crate) fn reconcile(stage: &Path, previous: &[Document], documents: &[Document]) -> Result<()> {
    let ids = documents.iter().map(|d| d.meta.id.clone()).collect();
    fs::create_dir_all(stage.join("ja"))?;
    archive_removed(stage, &ids)?;
    sync_documents(stage, previous, documents)
}

fn archive_removed(stage: &Path, ids: &HashSet<Slug>) -> Result<()> {
    for locale in Locale::ALL {
        for old in read_locale(stage, locale)? {
            if !ids.contains(&old.meta.id) {
                let path = stage.join(format!("{locale}/{}.md", old.meta.id));
                let archive = stage.join(".export-archive").join(format!(
                    "{locale}-{}-{}.md",
                    old.meta.id,
                    crate::content::digest(fs::read(&path)?)
                ));
                fs::create_dir_all(archive.parent().unwrap())?;
                fs::rename(path, archive)?;
            }
        }
    }
    Ok(())
}

fn sync_documents(stage: &Path, previous: &[Document], documents: &[Document]) -> Result<()> {
    for document in documents {
        let english_path = stage.join(format!("en/{}.md", document.meta.id));
        if english_path.exists() {
            let mut english = Document::parse(&fs::read_to_string(&english_path)?)?;
            let mut provenance = english.meta.translation.take();
            if previous
                .iter()
                .find(|d| d.meta.id == document.meta.id)
                .map(crate::content::text_hash)
                .transpose()?
                != Some(crate::content::text_hash(document)?)
                && let Some(provenance) = &mut provenance
            {
                provenance.stale = true;
            }
            english.meta.translation = provenance;
            english.refresh_translation_metadata(document);
            fs::write(english_path, english.encode()?)?;
        }
        fs::write(
            stage.join(format!("ja/{}.md", document.meta.id)),
            document.encode()?,
        )?;
    }
    Ok(())
}

pub(crate) fn sync_tags(stage: &Path) -> Result<()> {
    let path = stage.join("tags.json");
    let mut catalog = if path.exists() {
        read_catalog(&path)?
    } else {
        LabelCatalog::default()
    };
    let tags = read_locale(stage, Locale::Ja)?
        .into_iter()
        .flat_map(|d| d.meta.tags)
        .collect::<BTreeSet<_>>();
    if catalog
        .entries
        .keys()
        .any(|key| !tags.contains(key.as_str()))
    {
        let bytes = fs::read(&path)?;
        let archive = stage
            .join(".export-archive")
            .join(format!("tags-{}.json", crate::content::digest(&bytes)));
        fs::create_dir_all(archive.parent().unwrap())?;
        fs::write(archive, bytes)?;
    }
    catalog.entries.retain(|key, _| tags.contains(key.as_str()));
    for tag in tags {
        let tag = tag.to_string();
        catalog.entries.entry(tag.clone()).or_insert(LabelEntry {
            source: tag,
            context:
                "Public blog tag label; concise noun phrase. Do not change technical proper nouns."
                    .into(),
            translation: None,
        });
    }
    catalog.validate()?;
    write_json(&path, &catalog)
}

pub(crate) fn read_catalog(path: &Path) -> Result<LabelCatalog> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(ExportError::invalid_input("catalog must be a regular file"));
    }
    let mut catalog: LabelCatalog = serde_json::from_slice(&fs::read(path)?)?;
    catalog.validate_structure()?;
    // A source edit may change interpolation variables. Preserve the old text for
    // the normal update decision, but never expose an incompatible active value.
    for entry in catalog.entries.values_mut() {
        if let Some(translation) = &mut entry.translation
            && domain::placeholders(&entry.source) != domain::placeholders(&translation.value)
        {
            translation.stale = true;
        }
    }
    Ok(catalog)
}

pub(crate) fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    if fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    let mut temporary = tempfile::NamedTempFile::new_in(filesystem::parent(path))?;
    temporary.write_all(&bytes)?;
    temporary.persist(path)?;
    Ok(())
}
