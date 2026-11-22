//! Public Markdown file adapter and reconciliation of export-managed output.
use crate::{ExportError, Result, content::Document, filesystem};
use blog_domain::{LabelCatalog, LabelEntry, Locale, PublicContentMeta};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    io::Write,
    path::Path,
};

pub(crate) fn read_locale(root: &Path, locale: blog_domain::Locale) -> Result<Vec<Document>> {
    let directory = root.join(locale.as_str());
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut ids = HashSet::new();
    filesystem::files(&directory)?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| {
            let doc = Document::parse(&fs::read_to_string(&p)?)?;
            if doc.meta.locale != locale || p != root.join(doc.meta.markdown_path(locale)) {
                return Err(ExportError::invalid_input(
                    "public filename/locale must match its metadata",
                ));
            }
            if !ids.insert(doc.meta.id.clone()) {
                return Err(ExportError::invalid_input("duplicate public content ID"));
            }
            Ok(doc)
        })
        .collect()
}

pub(crate) fn read_document(
    root: &Path,
    locale: Locale,
    source: &PublicContentMeta,
) -> Result<Option<Document>> {
    let path = root.join(source.markdown_path(locale));
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return Err(ExportError::invalid_input(
                "public document must be a regular file",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let document = Document::parse(&fs::read_to_string(&path)?)?;
    if document.meta.locale != locale
        || document.meta.id != source.id
        || path != root.join(document.meta.markdown_path(locale))
    {
        return Err(ExportError::invalid_input(
            "public filename/locale must match its metadata",
        ));
    }
    Ok(Some(document))
}

pub(crate) fn reconcile(
    stage: &Path,
    previous: Vec<Document>,
    documents: &[Document],
) -> Result<()> {
    let sources: HashMap<_, _> = documents.iter().map(|doc| (&doc.meta.id, doc)).collect();
    let mut unchanged = HashSet::new();
    let mut retained = Vec::new();
    // Remove moved paths before writing any replacements, including when two IDs swap routes.
    for old in previous {
        if let Some(source) = reconcile_path(stage, &old, &sources)?
            && crate::content::text_hash(&old)? == crate::content::text_hash(source)?
        {
            unchanged.insert(old.meta.id);
        }
    }
    for locale in Locale::translations() {
        for mut old in read_locale(stage, locale)? {
            if let Some(source) = reconcile_path(stage, &old, &sources)? {
                if !unchanged.contains(&old.meta.id)
                    && let Some(history) = &mut old.meta.translation
                {
                    history.stale = true;
                }
                old.refresh_translation_metadata(source);
                retained.push(old);
            }
        }
    }
    for doc in documents.iter().chain(&retained) {
        write_document(stage, doc)?;
    }
    Ok(())
}

fn reconcile_path<'a>(
    stage: &Path,
    old: &Document,
    sources: &HashMap<&blog_domain::Slug, &'a Document>,
) -> Result<Option<&'a Document>> {
    let locale = old.meta.locale;
    let path = stage.join(old.meta.markdown_path(locale));
    let source = sources.get(&old.meta.id).copied();
    if let Some(doc) = source {
        if path != stage.join(doc.meta.markdown_path(locale)) {
            fs::remove_file(path)?;
        }
    } else {
        let archive = stage.join(".export-archive").join(format!(
            "{locale}-{}-{}.md",
            old.meta.id,
            crate::content::digest(fs::read(&path)?)
        ));
        fs::create_dir_all(archive.parent().unwrap())?;
        fs::rename(path, archive)?;
    }
    Ok(source)
}

pub(crate) fn write_document(root: &Path, doc: &Document) -> Result<()> {
    let path = root.join(doc.meta.markdown_path(doc.meta.locale));
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, doc.encode()?)?;
    Ok(())
}

pub(crate) fn prepare_tag_catalog(stage: &Path, documents: &[Document]) -> Result<LabelCatalog> {
    let path = stage.join("tags.json");
    let mut catalog = if path.exists() {
        read_catalog(&path)?
    } else {
        LabelCatalog::default()
    };
    let tags = documents
        .iter()
        .flat_map(|doc| &doc.meta.tags)
        .map(|tag| tag.as_str())
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
        catalog
            .entries
            .entry(tag.to_string())
            .or_insert_with_key(|tag| LabelEntry {
                source: tag.clone(),
                context: concat!(
                    "Public blog tag label; concise noun phrase. ",
                    "Do not change technical proper nouns."
                )
                .into(),
                translations: Default::default(),
            });
    }
    catalog.validate()?;
    Ok(catalog)
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
        for translation in entry.translations.values_mut() {
            if blog_domain::placeholders(&entry.source)
                != blog_domain::placeholders(&translation.value)
            {
                translation.stale = true;
            }
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
        fs::create_dir_all(root.path().join("ja/tech")).unwrap();
        let path = root.path().join("ja/tech/article.md");
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
    fn direct_document_read_distinguishes_missing_from_invalid_documents() {
        let root = tempfile::tempdir().unwrap();
        let source = document().meta;
        assert!(
            read_document(root.path(), Locale::Ja, &source)
                .unwrap()
                .is_none()
        );
        fs::create_dir_all(root.path().join("ja/tech")).unwrap();
        let path = root.path().join("ja/tech/article.md");
        let original = document();
        fs::write(&path, original.encode().unwrap()).unwrap();
        let actual = read_document(root.path(), Locale::Ja, &source)
            .unwrap()
            .unwrap();
        assert_eq!(actual.encode().unwrap(), original.encode().unwrap());
        for content in ["invalid Markdown".as_bytes(), &[255]] {
            fs::write(&path, content).unwrap();
            assert!(read_document(root.path(), Locale::Ja, &source).is_err());
        }
        for (wrong_id, wrong_locale) in [("other", Locale::Ja), ("article", Locale::En)] {
            let mut invalid = original.clone();
            invalid.meta.id = wrong_id.parse().unwrap();
            invalid.meta.locale = wrong_locale;
            fs::write(&path, invalid.encode().unwrap()).unwrap();
            assert!(read_document(root.path(), Locale::Ja, &source).is_err());
        }
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(read_document(root.path(), Locale::Ja, &source).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn direct_document_read_rejects_symlinks_including_broken_links() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("ja/tech")).unwrap();
        let target = root.path().join("target.md");
        fs::write(&target, document().encode().unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, root.path().join("ja/tech/article.md")).unwrap();
        let source = document().meta;
        assert!(read_document(root.path(), Locale::Ja, &source).is_err());
        fs::remove_file(target).unwrap();
        assert!(read_document(root.path(), Locale::Ja, &source).is_err());
    }

    #[test]
    fn syncing_marks_only_changed_source_text_stale_and_preserves_english_prose() {
        for changed in [false, true] {
            let root = tempfile::tempdir().unwrap();
            fs::create_dir_all(root.path().join("ja/tech")).unwrap();
            fs::create_dir_all(root.path().join("en/tech")).unwrap();
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
            fs::write(
                root.path().join("en/tech/article.md"),
                english.encode().unwrap(),
            )
            .unwrap();
            reconcile(root.path(), vec![previous], std::slice::from_ref(&current)).unwrap();
            let actual = read_locale(root.path(), Locale::En).unwrap().remove(0);
            assert_eq!(actual.meta.translation.unwrap().stale, changed);
            assert_eq!(actual.meta.summary, english.meta.summary);
            assert_eq!(actual.meta.title, english.meta.title);
            assert_eq!(actual.body, english.body);
        }
    }

    #[test]
    fn swapped_category_routes_keep_each_locales_reviewed_text_with_its_source_id() {
        let root = tempfile::tempdir().unwrap();
        let mut first = document();
        first.meta.kind = blog_domain::ContentKind::Category;
        first.meta.tags.clear();
        let mut second = first.clone();
        second.meta.id = "other".parse().unwrap();
        second.meta.category = Some(blog_domain::Category::Daily);
        let previous = vec![first, second];
        for source in &previous {
            write_document(root.path(), source).unwrap();
            for locale in Locale::translations() {
                let mut reviewed = source.clone();
                reviewed.meta.locale = locale;
                reviewed.body = format!("{locale} reviewed {}", source.meta.id);
                write_document(root.path(), &reviewed).unwrap();
            }
        }
        let mut current = previous.clone();
        current[0].meta.category = Some(blog_domain::Category::Daily);
        current[1].meta.category = Some(blog_domain::Category::Tech);
        reconcile(root.path(), previous, &current).unwrap();
        for source in &current {
            for locale in Locale::translations() {
                let reviewed = read_document(root.path(), locale, &source.meta)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    reviewed.body,
                    format!("{locale} reviewed {}", source.meta.id)
                );
            }
        }
    }

    #[test]
    fn tag_catalog_is_archived_only_when_a_tag_is_removed() {
        let root = tempfile::tempdir().unwrap();
        let mut doc = document();
        let catalog = prepare_tag_catalog(root.path(), std::slice::from_ref(&doc)).unwrap();
        write_json(&root.path().join("tags.json"), &catalog).unwrap();
        prepare_tag_catalog(root.path(), std::slice::from_ref(&doc)).unwrap();
        assert!(!root.path().join(".export-archive").exists());

        doc.meta.tags.clear();
        let catalog = prepare_tag_catalog(root.path(), &[doc]).unwrap();
        assert!(catalog.entries.is_empty());
        assert_eq!(
            fs::read_dir(root.path().join(".export-archive"))
                .unwrap()
                .count(),
            1
        );
    }
}
