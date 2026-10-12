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
        let meta: PublicContentMeta = yaml_serde::from_str(yaml)?;
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
            yaml_serde::to_string(&self.meta)?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn reviewed_frontmatter_round_trips_without_changing_yaml_or_markdown() {
        let text = indoc! {"
            ---
            schema_version: 1
            id: yaml-compatibility
            locale: en
            kind: article
            title: '123'
            summary: |-
              日本語: 概要
              true
            category: Tech
            tags:
            - 'true'
            - 日本語
            priority: -1
            created: 2025-01-01T00:00:00+09:00
            updated: 2025-01-02T00:00:00+09:00
            source_hash: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
            translation:
              input_hash: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
              generated_hash: cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
              stale: false
            ---
            # Reviewed prose

            本文 **bold** `code`
        "};
        let document = Document::parse(text).unwrap();
        assert_eq!(document.meta.title.as_str(), "123");
        assert_eq!(document.meta.summary.as_deref(), Some("日本語: 概要\ntrue"));
        assert_eq!(document.meta.tags[0].as_str(), "true");
        assert_eq!(document.encode().unwrap(), text);
    }
}
