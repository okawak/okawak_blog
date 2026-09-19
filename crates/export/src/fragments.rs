//! Reassemble translated prose into the original Markdown; non-text bytes never go to AI.
use crate::{markdown::Document, normalize::options, translation::Texts};
use anyhow::{Result, bail};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::ops::Range;

pub(crate) struct Fragments {
    pub(crate) texts: Texts,
    ranges: Vec<(String, Range<usize>)>,
}

impl Fragments {
    pub(crate) fn extract(document: &Document) -> Self {
        let mut texts = Texts::from([("title".into(), document.meta.title.clone())]);
        if let Some(summary) = &document.meta.summary {
            texts.insert("summary".into(), summary.clone());
        }
        let mut ranges = Vec::new();
        let mut code = false;
        for (event, range) in Parser::new_ext(&document.body, options()).into_offset_iter() {
            match event {
                Event::Start(Tag::CodeBlock(_)) => code = true,
                Event::End(TagEnd::CodeBlock) => code = false,
                Event::Text(text) if !code && !text.trim().is_empty() => {
                    let key = format!("text_{:05}", ranges.len());
                    texts.insert(key.clone(), text.to_string());
                    ranges.push((key, range));
                }
                _ => {}
            }
        }
        Self { texts, ranges }
    }

    pub(crate) fn apply(&self, source: &Document, result: &Texts) -> Result<Document> {
        if !self.texts.keys().eq(result.keys()) {
            bail!("fragment keys changed");
        }
        let mut translated = source.clone();
        translated.meta.title = result["title"].clone();
        translated.meta.summary = result.get("summary").cloned();
        for (key, range) in self.ranges.iter().rev() {
            let value = &result[key];
            if value.contains('\n') {
                bail!("translated inline fragment contains a line break");
            }
            translated.body.replace_range(range.clone(), &escape(value));
        }
        Ok(translated)
    }
}

pub(crate) fn text_hash(document: &Document) -> Result<String> {
    Ok(crate::vault::digest(serde_json::to_vec(&(
        &document.meta.title,
        &document.meta.summary,
        &document.body,
    ))?))
}

fn escape(text: &str) -> String {
    let mut result = String::new();
    for ch in text.chars() {
        if "\\`*_{}[]<>|$!#&".contains(ch) {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}
