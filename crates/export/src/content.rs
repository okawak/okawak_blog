//! Public document codec and pure Markdown/fingerprint primitives.
use crate::{ExportError, Result};
use domain::{PublicContentMeta, Sha256Digest};
use pulldown_cmark::Options;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub(crate) struct Document {
    pub(crate) meta: PublicContentMeta,
    pub(crate) body: String,
}

impl Document {
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let (yaml, body) = split(text)?
            .ok_or_else(|| ExportError::invalid_input("public Markdown requires frontmatter"))?;
        let meta: PublicContentMeta = serde_yaml::from_str(yaml)?;
        meta.validate()?;
        Ok(Self {
            meta,
            body: body.into(),
        })
    }

    /// Keep reviewed English prose and provenance while taking management fields from Japanese.
    pub(crate) fn refresh_translation_metadata(&mut self, source: &Self) {
        self.meta = PublicContentMeta {
            locale: domain::Locale::En,
            title: self.meta.title.clone(),
            summary: self.meta.summary.clone(),
            translation: self.meta.translation.clone(),
            ..source.meta.clone()
        };
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
        .ok_or_else(|| ExportError::invalid_input("unterminated frontmatter"))?;
    Ok(Some((yaml, body)))
}

pub(crate) fn digest(bytes: impl AsRef<[u8]>) -> Sha256Digest {
    Sha256Digest::from_bytes(Sha256::digest(bytes.as_ref()).into())
}

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH
        | Options::ENABLE_WIKILINKS
}

pub(crate) fn text_hash(document: &Document) -> Result<Sha256Digest> {
    Ok(digest(serde_json::to_vec(&(
        &document.meta.title,
        &document.meta.summary,
        &document.body,
    ))?))
}
