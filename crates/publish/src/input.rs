//! Read the public Markdown contract. No vault paths or publication flags.
use crate::{PublishError, Result};
use domain::{ContentKind, LabelCatalog, Locale, PublicContentMeta};
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

pub(crate) fn read_tag_catalog(root: &Path) -> Result<LabelCatalog> {
    let path = root.join("tags.json");
    let catalog: LabelCatalog = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => serde_json::from_slice(&fs::read(path)?)?,
        Ok(_) => return Err(invalid("public tag catalog must be a regular file")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Default::default(),
        Err(error) => return Err(error.into()),
    };
    catalog.validate()?;
    Ok(catalog)
}

pub(crate) fn read(root: &Path) -> Result<Vec<Document>> {
    if !public_directory_exists(root)? {
        return Err(PublishError::InvalidSourceDirectory(
            root.display().to_string(),
        ));
    }
    public_directory_exists(&root.join("assets"))?;
    let mut documents = Vec::new();
    let mut routes = HashSet::new();
    let mut identities = HashSet::new();
    for locale in Locale::ALL {
        let directory = root.join(locale.as_str());
        if !public_directory_exists(&directory)? {
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
                || ja.section_path != en.meta.section_path
                || ja.priority != en.meta.priority
                || ja.updated != en.meta.updated
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

fn public_directory_exists(path: &Path) -> Result<bool> {
    // A trailing slash makes the OS follow a directory symlink even for lstat.
    let path: std::path::PathBuf = path.components().collect();
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(_) => Err(PublishError::InvalidSourceDirectory(
            path.display().to_string(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn rejects_symlinked_public_input_directories() {
        let temp = tempfile::tempdir().unwrap();
        let external = temp.path().join("external");
        fs::create_dir(&external).unwrap();
        assert!(read(&external).is_ok());
        for target_exists in [true, false] {
            let target = if target_exists {
                external.clone()
            } else {
                temp.path().join("missing")
            };
            for location in ["root", "ja", "en", "assets"] {
                let root = temp
                    .path()
                    .join(format!("public-{location}-{target_exists}"));
                if location == "root" {
                    symlink(&target, &root).unwrap();
                } else {
                    fs::create_dir(&root).unwrap();
                    symlink(&target, root.join(location)).unwrap();
                }
                assert!(read(&root).is_err(), "accepted symlink: {root:?}");
                assert!(
                    read(&root.join("")).is_err(),
                    "accepted trailing slash: {root:?}"
                );
            }
        }
    }
}
