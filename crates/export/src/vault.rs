//! Vault input adapter. Only explicitly completed notes cross this boundary.
mod normalize;
use crate::content::digest;
use crate::{
    ExportError, Result,
    content::{self, Document},
    filesystem,
};
use domain::{ContentKind, Locale, PublicContentMeta, SectionPath, Slug};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

#[derive(Deserialize)]
struct Frontmatter {
    title: String,
    #[serde(default)]
    kind: ContentKind,
    summary: Option<String>,
    category: Option<domain::Category>,
    page: Option<domain::PageKey>,
    #[serde(default)]
    tags: Vec<String>,
    priority: Option<i32>,
    created: String,
    updated: String,
    publish_id: Option<Slug>,
}

struct Source {
    key: String,
    document: Document,
}

pub(crate) struct Prepared {
    pub(crate) documents: Vec<Document>,
    pub(crate) assets: BTreeMap<String, Vec<u8>>,
}

pub(crate) fn prepare(root: &Path, previous: &[Document]) -> Result<Prepared> {
    let mut sources = extract(root, previous)?;
    let assets = normalize::normalize(&mut sources, root)?;
    Ok(Prepared {
        documents: sources.into_iter().map(|s| s.document).collect(),
        assets,
    })
}

fn extract(root: &Path, previous: &[Document]) -> Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut claimed = HashSet::new();
    let mut routes = HashSet::new();
    let paths = filesystem::files(root)?;
    let current_hashes: HashSet<_> = paths
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
        let fm: Frontmatter = serde_yaml::from_str(yaml)?;
        let relative = path.strip_prefix(root)?;
        let relative_str = relative
            .to_str()
            .ok_or_else(|| ExportError::invalid_input("source path must be UTF-8"))?;
        let source_hash = digest(relative_str);
        let exact = previous.iter().find(|d| d.meta.source_hash == source_hash);
        let renamed: Vec<_> = previous
            .iter()
            .filter(|d| {
                d.meta.kind == fm.kind
                    && d.meta.created == fm.created
                    && !current_hashes.contains(&d.meta.source_hash)
            })
            .collect();
        let id = if let Some(id) = fm.publish_id {
            id
        } else if let Some(old) = exact {
            old.meta.id.clone()
        } else if renamed.len() == 1 {
            renamed[0].meta.id.clone()
        } else if renamed.len() > 1 {
            return Err(ExportError::invalid_input(
                "ambiguous source identity; specify publish_id",
            ));
        } else {
            Slug::new(
                digest(format!("{}/{relative_str}/{}", fm.title, fm.created))[..12].to_owned(),
            )?
        };
        if !claimed.insert(id.clone()) {
            return Err(ExportError::invalid_input(
                "duplicate source identity; specify distinct publish_id values",
            ));
        }
        let section_path = if fm.kind == ContentKind::Article {
            let category = fm
                .category
                .ok_or_else(|| ExportError::invalid_input("article requires category"))?;
            let article_path = relative
                .strip_prefix(category.as_str())
                .map_err(|_| ExportError::invalid_input("article directory must match category"))?;
            SectionPath::new(
                article_path
                    .parent()
                    .into_iter()
                    .flat_map(|p| p.iter())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
            )
        } else {
            SectionPath::default()
        };
        let meta = PublicContentMeta {
            schema_version: 1,
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
        sources.push(Source {
            key: relative.with_extension("").to_string_lossy().into_owned(),
            document: Document {
                meta,
                body: body.to_owned(),
            },
        });
    }
    Ok(sources)
}
