//! Vault input adapter. Only explicitly completed notes cross this boundary.
mod normalize;
use crate::content::digest;
use crate::{
    ExportError, Result,
    content::{self, Document},
    filesystem,
};
use domain::{
    Category, ContentKind, GENERATED_CONTENT_ID_LENGTH, Locale, PUBLIC_CONTENT_SCHEMA_VERSION,
    PageKey, PublicContentMeta, SectionPath, Sha256Digest, Slug, TagId, Timestamp, Title,
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

#[derive(Deserialize)]
struct Frontmatter {
    title: Title,
    #[serde(default)]
    kind: ContentKind,
    summary: Option<String>,
    category: Option<Category>,
    page: Option<PageKey>,
    #[serde(default)]
    tags: Vec<TagId>,
    priority: Option<i32>,
    created: Timestamp,
    updated: Timestamp,
    publish_id: Option<Slug>,
}

struct VaultNote {
    key: String,
    document: Document,
}

pub(crate) fn prepare(root: &Path, previous: &[Document]) -> Result<Vec<Document>> {
    let mut notes = extract(root, previous)?;
    normalize::normalize(&mut notes)?;
    Ok(notes.into_iter().map(|note| note.document).collect())
}

fn extract(root: &Path, previous: &[Document]) -> Result<Vec<VaultNote>> {
    let mut notes = Vec::new();
    let mut claimed_ids = HashSet::new();
    let mut routes = HashSet::new();
    let mut previous_by_source_hash = HashMap::with_capacity(previous.len());
    for document in previous {
        // Keep the first document when source hashes repeat.
        previous_by_source_hash
            .entry(&document.meta.source_hash)
            .or_insert(document);
    }
    let paths = filesystem::files(root)?;
    // files(root) returns only paths beneath root, so stripping this prefix cannot fail.
    let current_source_hashes: HashSet<_> = paths
        .iter()
        .map(|p| digest(p.strip_prefix(root).unwrap().to_string_lossy().as_bytes()))
        .collect();
    for path in paths {
        if !path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("md"))
        {
            continue;
        }
        let text = fs::read_to_string(&path)?.replace("\r\n", "\n");
        let Some((yaml, body)) = content::split(&text)? else {
            continue;
        };
        // Inspect only the publication flag before requiring the public fields.
        // Drafts may omit title, category or timestamps entirely.
        let fields: serde_yaml::Value = serde_yaml::from_str(yaml)?;
        if fields
            .get("is_completed")
            .and_then(serde_yaml::Value::as_bool)
            != Some(true)
        {
            continue;
        }
        // Value conversion rejects unquoted numeric/bool strings accepted by from_str.
        let fm: Frontmatter =
            serde_yaml::from_value(fields).or_else(|_| serde_yaml::from_str(yaml))?;
        let relative = path.strip_prefix(root)?;
        let relative_str = relative
            .to_str()
            .ok_or_else(|| ExportError::invalid_input("source path must be UTF-8"))?;
        let source_hash = digest(relative_str);
        let same_path = previous_by_source_hash.get(&source_hash).copied();
        let id = resolve_id(
            &fm,
            relative_str,
            same_path,
            previous,
            &current_source_hashes,
        )?;
        if !claimed_ids.insert(id.clone()) {
            return Err(ExportError::invalid_input(
                "duplicate source identity; specify distinct publish_id values",
            ));
        }
        let section_path = section_path(relative, &fm)?;
        let meta = PublicContentMeta {
            schema_version: PUBLIC_CONTENT_SCHEMA_VERSION,
            id,
            locale: Locale::Ja,
            kind: fm.kind,
            title: fm.title,
            summary: fm.summary,
            category: fm.category,
            page: fm.page,
            section_path,
            tags: if fm.kind == ContentKind::Article {
                fm.tags
            } else {
                Vec::new()
            },
            priority: fm.priority,
            created: fm.created,
            updated: fm.updated,
            source_hash,
            translation: None,
        };
        meta.validate()?;
        if !routes.insert(meta.path()) {
            return Err(ExportError::invalid_input("duplicate public route"));
        }
        notes.push(VaultNote {
            key: relative.with_extension("").to_string_lossy().into_owned(),
            document: Document {
                meta,
                body: body.to_owned(),
            },
        });
    }
    Ok(notes)
}

fn resolve_id(
    fm: &Frontmatter,
    relative: &str,
    same_path: Option<&Document>,
    previous: &[Document],
    current_source_hashes: &HashSet<Sha256Digest>,
) -> Result<Slug> {
    if let Some(id) = &fm.publish_id {
        return Ok(id.clone());
    }
    if let Some(old) = same_path {
        return Ok(old.meta.id.clone());
    }
    let mut candidates = previous.iter().filter(|d| {
        d.meta.kind == fm.kind
            && d.meta.created == fm.created
            && !current_source_hashes.contains(&d.meta.source_hash)
    });
    match (candidates.next(), candidates.next()) {
        (Some(old), None) => Ok(old.meta.id.clone()),
        (None, _) => {
            let hash = digest(format!("{}/{relative}/{}", fm.title, fm.created));
            Ok(Slug::new(
                hash.as_str()[..GENERATED_CONTENT_ID_LENGTH].to_owned(),
            )?)
        }
        (Some(_), Some(_)) => Err(ExportError::invalid_input(
            "ambiguous source identity; specify publish_id",
        )),
    }
}

fn section_path(relative: &Path, fm: &Frontmatter) -> Result<SectionPath> {
    if fm.kind != ContentKind::Article {
        return Ok(SectionPath::default());
    }
    let category = fm
        .category
        .ok_or_else(|| ExportError::invalid_input("article requires category"))?;
    let article_path = relative
        .strip_prefix(category.as_str())
        .map_err(|_| ExportError::invalid_input("article directory must match category"))?;
    Ok(SectionPath::new(
        article_path
            .parent()
            .into_iter()
            .flat_map(|p| p.iter())
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::{formatdoc, indoc};

    fn frontmatter() -> Frontmatter {
        serde_yaml::from_str(indoc! {"
            title: Article
            category: tech
            created: '2025-01-01T00:00:00+09:00'
            updated: '2025-01-01T00:00:00+09:00'
        "})
        .unwrap()
    }

    fn previous_document(id: &str, path: &str) -> Document {
        Document::parse(&formatdoc! {"
            ---
            schema_version: 1
            id: {id}
            locale: ja
            kind: article
            title: Article
            category: tech
            created: '2025-01-01T00:00:00+09:00'
            updated: '2025-01-01T00:00:00+09:00'
            source_hash: '{}'
            ---
            Body
        ", digest(path)})
        .unwrap()
    }

    #[rstest::rstest]
    #[case(Some("explicit"), true, "explicit")]
    #[case(Some("explicit"), false, "explicit")]
    #[case(None, true, "existing")]
    fn explicit_id_and_exact_path_take_priority_over_ambiguous_renames(
        #[case] publish_id: Option<&str>,
        #[case] exact: bool,
        #[case] expected: &str,
    ) {
        let mut fm = frontmatter();
        fm.publish_id = publish_id.map(|id| id.parse().unwrap());
        let path = "tech/article.md";
        let hash = digest(path);
        let current_source_hashes = HashSet::from([hash.clone()]);
        let mut previous = vec![
            previous_document("first", "tech/removed-a.md"),
            previous_document("second", "tech/removed-b.md"),
        ];
        if exact {
            previous.push(previous_document("existing", path));
        }
        let same_path = previous.iter().find(|d| d.meta.source_hash == hash);
        assert_eq!(
            resolve_id(&fm, path, same_path, &previous, &current_source_hashes)
                .unwrap()
                .as_str(),
            expected,
        );
    }

    #[test]
    fn rename_candidates_must_be_missing_and_match_kind_and_creation_time() {
        let fm = frontmatter();
        let mut other_kind = previous_document("category", "tech/landing.md");
        other_kind.meta.kind = ContentKind::Category;
        let mut other_created = previous_document("older", "tech/older.md");
        other_created.meta.created = "2024-01-01T00:00:00+09:00".parse().unwrap();
        let previous = [
            previous_document("present", "tech/present.md"),
            other_kind,
            other_created,
            previous_document("renamed", "tech/removed.md"),
        ];
        let path = "tech/new.md";
        let hash = digest(path);
        let current_source_hashes = HashSet::from([hash, digest("tech/present.md")]);
        assert_eq!(
            resolve_id(&fm, path, None, &previous, &current_source_hashes)
                .unwrap()
                .as_str(),
            "renamed",
        );
    }

    #[test]
    fn sections_are_directories_below_the_article_category() {
        let sections =
            section_path(Path::new("tech/rust/async/article.md"), &frontmatter()).unwrap();
        assert_eq!(sections.segments(), &["rust", "async"]);
    }

    #[rstest::rstest]
    #[case(None, "tech/article.md", "article requires category")]
    #[case(
        Some(Category::Tech),
        "daily/article.md",
        "article directory must match category"
    )]
    fn section_path_rejects_missing_or_mismatched_category(
        #[case] category: Option<Category>,
        #[case] path: &str,
        #[case] message: &str,
    ) {
        let mut fm = frontmatter();
        fm.category = category;
        assert!(matches!(
            section_path(Path::new(path), &fm),
            Err(ExportError::InvalidInput(error)) if error == message,
        ));
    }
}
