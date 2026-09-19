use anyhow::{Context, Result, bail};
use domain::PublicContentMeta;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(crate) struct Document {
    pub(crate) meta: PublicContentMeta,
    pub(crate) body: String,
}

impl Document {
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let (yaml, body) = split(text)?.context("public Markdown requires frontmatter")?;
        let meta: PublicContentMeta = serde_yaml::from_str(yaml)?;
        meta.validate()?;
        Ok(Self {
            meta,
            body: body.into(),
        })
    }

    pub(crate) fn encode(&self) -> Result<String> {
        self.meta.validate()?;
        Ok(format!(
            "---\n{}---\n{}",
            serde_yaml::to_string(&self.meta)?,
            self.body
        ))
    }
}

pub(crate) fn split(text: &str) -> Result<Option<(&str, &str)>> {
    let Some(rest) = text.trim_start().strip_prefix("---\n") else {
        return Ok(None);
    };
    let (yaml, body) = rest
        .split_once("\n---\n")
        .context("unterminated frontmatter")?;
    Ok(Some((yaml, body)))
}

/// Reject symlinks instead of silently including content outside the selected root.
pub(crate) fn files(root: &Path) -> Result<Vec<PathBuf>> {
    walk(root, false)
}

pub(crate) fn all_files(root: &Path) -> Result<Vec<PathBuf>> {
    walk(root, true)
}

fn walk(root: &Path, hidden: bool) -> Result<Vec<PathBuf>> {
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        bail!("symlink root is not allowed");
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !hidden && entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            bail!("symlink input is not allowed: {}", entry.path().display());
        }
        if kind.is_dir() {
            paths.extend(walk(&entry.path(), hidden)?);
        } else if kind.is_file() {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

pub(crate) fn read_locale(root: &Path, locale: domain::Locale) -> Result<Vec<Document>> {
    let root = root.join(locale.as_str());
    if !root.exists() {
        return Ok(Vec::new());
    }
    files(&root)?
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .map(|p| {
            let doc = Document::parse(&fs::read_to_string(&p)?)?;
            if doc.meta.locale != locale
                || p.file_stem().and_then(|s| s.to_str()) != Some(doc.meta.id.as_str())
            {
                bail!("public filename/locale must match its metadata");
            }
            Ok(doc)
        })
        .collect()
}
