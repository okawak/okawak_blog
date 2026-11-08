//! Public Markdown file adapter and reconciliation of export-managed output.
use crate::{ExportError, Result, content::Document, filesystem};
use blog_domain::{LabelCatalog, LabelEntry, Locale, Slug};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    io::Write,
    path::Path,
};

pub(crate) fn read_locale(root: &Path, locale: blog_domain::Locale) -> Result<Vec<Document>> {
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
            && blog_domain::placeholders(&entry.source)
                != blog_domain::placeholders(&translation.value)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> Document {
        Document {
            meta: serde_json::from_value(serde_json::json!({
                "schema_version":1, "id":"article", "locale":"ja", "kind":"article",
                "title":"原文", "category":"tech", "tags":["rust"],
                "created":"2026-01-01T00:00:00Z", "updated":"2026-01-01T00:00:00Z",
                "source_hash":"a".repeat(64)
            }))
            .unwrap(),
            body: "本文".into(),
        }
    }

    #[test]
    fn public_locale_files_must_match_both_filename_and_locale() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ja")).unwrap();
        let path = root.path().join("ja/article.md");
        let mut doc = document();
        fs::write(&path, doc.encode().unwrap()).unwrap();
        assert_eq!(read_locale(root.path(), Locale::Ja).unwrap().len(), 1);
        doc.meta.id = "different".parse().unwrap();
        fs::write(&path, doc.encode().unwrap()).unwrap();
        assert!(read_locale(root.path(), Locale::Ja).is_err());
        doc.meta.id = "article".parse().unwrap();
        doc.meta.locale = Locale::En;
        fs::write(path, doc.encode().unwrap()).unwrap();
        assert!(read_locale(root.path(), Locale::Ja).is_err());
    }

    #[test]
    fn syncing_marks_only_changed_source_text_stale_and_preserves_english_prose() {
        for changed in [false, true] {
            let root = tempfile::tempdir().unwrap();
            fs::create_dir(root.path().join("ja")).unwrap();
            fs::create_dir(root.path().join("en")).unwrap();
            let previous = document();
            let mut current = previous.clone();
            if changed {
                current.body = "更新した本文".into();
            }
            let mut english = previous.clone();
            english.meta.locale = Locale::En;
            english.meta.title = "Reviewed title".parse().unwrap();
            english.meta.summary = Some("Reviewed summary".into());
            english.body = "Reviewed prose".into();
            english.meta.translation = Some(blog_domain::TranslationProvenance {
                input_hash: crate::content::digest("input"),
                generated_hash: crate::content::digest("generated"),
                stale: false,
            });
            fs::write(root.path().join("en/article.md"), english.encode().unwrap()).unwrap();
            sync_documents(root.path(), &[previous], &[current]).unwrap();
            let actual = read_locale(root.path(), Locale::En).unwrap().remove(0);
            assert_eq!(actual.meta.translation.unwrap().stale, changed);
            assert_eq!(actual.meta.summary, english.meta.summary);
            assert_eq!(actual.meta.title, english.meta.title);
            assert_eq!(actual.body, english.body);
        }
    }

    #[test]
    fn tag_catalog_is_archived_only_when_a_tag_is_removed() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("ja")).unwrap();
        let path = root.path().join("ja/article.md");
        let mut doc = document();
        fs::write(&path, doc.encode().unwrap()).unwrap();
        sync_tags(root.path()).unwrap();
        sync_tags(root.path()).unwrap();
        assert!(!root.path().join(".export-archive").exists());
        doc.meta.tags.clear();
        fs::write(path, doc.encode().unwrap()).unwrap();
        sync_tags(root.path()).unwrap();
        assert!(
            read_catalog(&root.path().join("tags.json"))
                .unwrap()
                .entries
                .is_empty()
        );
        assert_eq!(
            fs::read_dir(root.path().join(".export-archive"))
                .unwrap()
                .count(),
            1
        );
    }
}
