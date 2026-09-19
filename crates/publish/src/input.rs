//! Read the public Markdown contract. No vault paths or publication flags.
use crate::{PublishError, Result};
use domain::{ContentKind, Locale, PublicContentMeta};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

#[derive(Clone)]
pub(crate) struct Document {
    pub(crate) meta: PublicContentMeta,
    pub(crate) body: String,
}

pub(crate) fn read(root: &Path) -> Result<Vec<Document>> {
    if !root.is_dir() {
        return Err(PublishError::InvalidSourceDirectory(
            root.display().to_string(),
        ));
    }
    let mut documents = Vec::new();
    let mut routes = HashSet::new();
    let mut identities = HashSet::new();
    for locale in Locale::ALL {
        let directory = root.join(locale.as_str());
        if !directory.exists() {
            continue;
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                return Err(invalid(
                    "public locale input must contain regular files only",
                ));
            }
            if entry.path().extension().is_none_or(|e| e != "md") {
                continue;
            }
            let text = fs::read_to_string(entry.path())?;
            let Some((yaml, body)) = text
                .strip_prefix("---\n")
                .and_then(|s| s.split_once("\n---\n"))
            else {
                return Err(invalid("public Markdown requires frontmatter"));
            };
            let meta: PublicContentMeta =
                serde_yaml::from_str(yaml).map_err(|_| invalid("invalid public frontmatter"))?;
            meta.validate()
                .map_err(|_| invalid("public frontmatter validation failed"))?;
            if meta.locale != locale
                || entry.path().file_stem().and_then(|s| s.to_str()) != Some(meta.id.as_str())
            {
                return Err(invalid("public filename/locale must match metadata"));
            }
            if body.trim().is_empty() {
                return Err(invalid("public body must not be empty"));
            }
            if !identities.insert((locale, meta.id.clone())) || !routes.insert(meta.path()) {
                return Err(invalid("duplicate public identity or route"));
            }
            documents.push(Document {
                meta,
                body: body.into(),
            });
        }
    }
    documents.sort_by_key(|d| (d.meta.locale.as_str(), d.meta.id.to_string()));
    let japanese: HashMap<_, _> = documents
        .iter()
        .filter(|d| d.meta.locale == Locale::Ja)
        .map(|d| (&d.meta.id, &d.meta))
        .collect();
    for en in documents.iter().filter(|d| d.meta.locale == Locale::En) {
        let Some(ja) = japanese.get(&en.meta.id) else {
            return Err(invalid("English content has no Japanese source"));
        };
        if en.meta.translation.as_ref().is_some_and(|p| !p.stale)
            && (ja.kind != en.meta.kind
                || ja.category != en.meta.category
                || ja.page != en.meta.page
                || ja.tags != en.meta.tags
                || ja.source_hash != en.meta.source_hash
                || ja.created != en.meta.created)
        {
            return Err(invalid(
                "English identity metadata differs from Japanese source",
            ));
        }
    }
    Ok(documents)
}

pub(crate) fn eligible(documents: &[Document], locale: Locale) -> Vec<Document> {
    let candidates: Vec<_> = documents
        .iter()
        .filter(|d| {
            d.meta.locale == locale
                && (locale == Locale::Ja || d.meta.translation.as_ref().is_some_and(|p| !p.stale))
        })
        .cloned()
        .collect();
    if locale == Locale::Ja {
        return candidates;
    }
    let categories: HashSet<_> = candidates
        .iter()
        .filter(|d| d.meta.kind == ContentKind::Category)
        .filter_map(|d| d.meta.category)
        .collect();
    candidates
        .into_iter()
        .filter(|d| {
            d.meta.kind != ContentKind::Article
                || d.meta.category.is_some_and(|c| categories.contains(&c))
        })
        .collect()
}

fn invalid(message: &str) -> PublishError {
    tracing::error!(message, "invalid public input");
    PublishError::ContentErrors { count: 1 }
}
