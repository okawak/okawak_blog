//! Reassemble translated prose into the original Markdown; non-text bytes never go to AI.
use crate::{markdown::Document, normalize::options, translation::Texts};
use anyhow::{Result, bail};
use pulldown_cmark::{Event, LinkType, Parser, Tag, TagEnd};
use std::ops::Range;

pub(crate) struct Fragments {
    pub(crate) texts: Texts,
    ranges: Vec<Fragment>,
}

struct Fragment {
    key: String,
    range: Range<usize>,
    leading: String,
    trailing: String,
}

impl Fragments {
    pub(crate) fn extract(document: &Document) -> Self {
        let mut texts = Texts::from([("title".into(), document.meta.title.clone())]);
        if let Some(summary) = &document.meta.summary {
            texts.insert("summary".into(), summary.clone());
        }
        let mut ranges = Vec::new();
        let mut code = false;
        let mut autolink = false;
        let mut html_elements = Vec::new();
        for (event, range) in Parser::new_ext(&document.body, options()).into_offset_iter() {
            match event {
                Event::Start(Tag::CodeBlock(_)) => code = true,
                Event::End(TagEnd::CodeBlock) => code = false,
                Event::Start(Tag::Link {
                    link_type: LinkType::Autolink | LinkType::Email,
                    ..
                }) => autolink = true,
                Event::End(TagEnd::Link) => autolink = false,
                Event::InlineHtml(html) => track_inline_html(&html, &mut html_elements),
                Event::Text(text)
                    if !code
                        && !autolink
                        && html_elements.is_empty()
                        && !text.trim().is_empty() =>
                {
                    let key = format!("text_{:05}", ranges.len());
                    texts.insert(key.clone(), text.trim().to_string());
                    ranges.push(Fragment {
                        key,
                        range,
                        leading: text[..text.len() - text.trim_start().len()].into(),
                        trailing: text[text.trim_end().len()..].into(),
                    });
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
        for fragment in self.ranges.iter().rev() {
            let value = &result[&fragment.key];
            if value.contains('\n') {
                bail!("translated inline fragment contains a line break");
            }
            translated.body.replace_range(
                fragment.range.clone(),
                &format!(
                    "{}{}{}",
                    fragment.leading,
                    escape(value.trim()),
                    fragment.trailing
                ),
            );
        }
        Ok(translated)
    }
}

// InlineHtml events are already individual HTML tokens recognized by the
// Markdown parser. Track their element names without interpreting attributes.
fn track_inline_html(token: &str, elements: &mut Vec<String>) {
    let Some(tag) = token.strip_prefix('<') else {
        return;
    };
    let closing = tag.starts_with('/');
    let tag = tag.strip_prefix('/').unwrap_or(tag);
    if !tag.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return; // Comments, declarations and processing instructions.
    }
    let name: String = tag
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ':')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if closing {
        if let Some(position) = elements.iter().rposition(|element| element == &name) {
            elements.truncate(position);
        }
    } else if !token.trim_end().ends_with("/>")
        && !matches!(
            name.as_str(),
            "area"
                | "base"
                | "br"
                | "col"
                | "embed"
                | "hr"
                | "img"
                | "input"
                | "link"
                | "meta"
                | "param"
                | "source"
                | "track"
                | "wbr"
        )
    {
        elements.push(name);
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
        if ch.is_ascii_punctuation() {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}
