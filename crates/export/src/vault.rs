//! Obsidian input adapter. Only explicitly completed notes cross this boundary.
use crate::markdown::{self, Document};
use anyhow::{Context, Result, bail};
use domain::{ContentKind, Locale, PublicContentMeta, SectionPath, Slug};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, path::Path};

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

pub(crate) struct Source {
    pub(crate) key: String,
    pub(crate) document: Document,
}

pub(crate) fn digest(bytes: impl AsRef<[u8]>) -> String {
    Sha256::digest(bytes.as_ref())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn extract(root: &Path, previous: &[Document]) -> Result<Vec<Source>> {
    let mut sources = Vec::new();
    let mut claimed = HashSet::new();
    let mut routes = HashSet::new();
    let paths = markdown::files(root)?;
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
        let Some((yaml, body)) = markdown::split(&text)? else {
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
        let fm: Frontmatter = serde_yaml::from_str(yaml).context("invalid completed note")?;
        let relative = path.strip_prefix(root)?;
        let relative_str = relative.to_str().context("source path must be UTF-8")?;
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
            bail!("ambiguous source identity; specify publish_id");
        } else {
            Slug::new(
                digest(format!("{}/{relative_str}/{}", fm.title, fm.created))[..12].to_owned(),
            )?
        };
        if !claimed.insert(id.clone()) {
            bail!("duplicate source identity; specify distinct publish_id values");
        }
        let section_path = if fm.kind == ContentKind::Article {
            let category = fm.category.context("article requires category")?;
            let article_path = relative
                .strip_prefix(category.as_str())
                .context("article directory must match category")?;
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
            bail!("duplicate public route");
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
