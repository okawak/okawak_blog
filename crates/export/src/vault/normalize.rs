//! Resolve references using only the explicitly public source set.
use super::VaultNote;
use crate::{
    ExportError, Result,
    content::{digest, options},
};
use pulldown_cmark::{Event, LinkType, Parser, Tag};
use std::{
    collections::HashMap,
    ops::Range,
    path::{Component, Path},
};

pub(super) fn normalize(notes: &mut [VaultNote]) -> Result<()> {
    let index = NoteIndex::new(notes);
    for note in notes {
        let body = &note.document.body;
        let mut edits = Vec::new();
        let mut heading_counts = HashMap::<String, usize>::new();
        for (event, range) in Parser::new_ext(body, options()).into_offset_iter() {
            match event {
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    ..
                })
                | Event::Start(Tag::Image {
                    link_type,
                    dest_url,
                    title,
                    ..
                }) => {
                    if let Some(link) = rewrite_link(
                        &body[range.clone()],
                        link_type,
                        &dest_url,
                        &title,
                        &note.key,
                        &index,
                    )? {
                        edits.push((range, link));
                    }
                }
                Event::Start(Tag::Heading { .. }) => {
                    let heading = heading_text(&body[range.clone()]);
                    let count = heading_counts.entry(heading.clone()).or_default();
                    *count += 1;
                    let suffix = if *count == 1 {
                        String::new()
                    } else {
                        format!("-{count}")
                    };
                    edits.push((
                        range.start..range.start,
                        format!(
                            "<a id=\"section-{}{suffix}\"></a>\n\n",
                            &digest(heading).as_str()[..12]
                        ),
                    ));
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    validate_html(&html)?;
                }
                _ => {}
            }
        }
        note.document.body = remove_reference_definitions(apply_edits(body, edits)?)?;
    }
    Ok(())
}

struct Note {
    id: String,
    title: String,
    headings: HashMap<String, usize>,
}

struct NoteIndex {
    notes: Vec<Note>,
    by_path: HashMap<String, Vec<usize>>,
    by_name: HashMap<String, Vec<usize>>,
}

impl NoteIndex {
    fn new(notes: &[VaultNote]) -> Self {
        let mut index = Self {
            notes: Vec::with_capacity(notes.len()),
            by_path: HashMap::with_capacity(notes.len()),
            by_name: HashMap::with_capacity(notes.len()),
        };
        for (position, note) in notes.iter().enumerate() {
            // Different extension spellings can produce the same key; retain all candidates.
            index
                .by_path
                .entry(note.key.clone())
                .or_default()
                .push(position);
            let name = note.key.rsplit('/').next().unwrap_or(&note.key);
            index
                .by_name
                .entry(name.to_owned())
                .or_default()
                .push(position);
            let mut headings = HashMap::<String, usize>::new();
            for (event, range) in Parser::new_ext(&note.document.body, options()).into_offset_iter()
            {
                if matches!(event, Event::Start(Tag::Heading { .. })) {
                    *headings
                        .entry(heading_text(&note.document.body[range]))
                        .or_default() += 1;
                }
            }
            index.notes.push(Note {
                id: note.document.meta.id.to_string(),
                title: note.document.meta.title.to_string(),
                headings,
            });
        }
        index
    }

    fn resolve(&self, source_key: &str, target: &str, wiki: bool) -> Result<Option<&Note>> {
        let extensionless = if Path::new(target)
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            &target[..target.len() - 3]
        } else {
            target
        };
        let relative_path = if extensionless.is_empty() {
            source_key.to_owned()
        } else {
            resolve_relative(source_key, extensionless)?
        };
        let relative = self.by_path.get(&relative_path);
        // Do not count the same path twice, or treat a self-reference as a root lookup.
        let root = if wiki && !extensionless.is_empty() && extensionless != relative_path {
            self.by_path.get(extensionless)
        } else {
            None
        };
        let basename = if wiki && relative.is_none() && root.is_none() {
            self.by_name.get(extensionless)
        } else {
            None
        };
        let mut matches = relative.into_iter().chain(root).chain(basename).flatten();
        match (matches.next(), matches.next()) {
            (None, _) => Ok(None),
            (Some(&position), None) => Ok(Some(&self.notes[position])),
            _ => Err(ExportError::invalid_input(
                "missing, non-public or ambiguous note reference",
            )),
        }
    }
}

fn resolve_relative(source_key: &str, target: &str) -> Result<String> {
    let parent = Path::new(source_key).parent().unwrap_or(Path::new(""));
    let combined = parent.join(target);
    let mut parts = Vec::new();
    for part in combined.components() {
        match part {
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir if !parts.is_empty() => {
                parts.pop();
            }
            _ => {
                return Err(ExportError::invalid_input(
                    "reference escapes public source root",
                ));
            }
        }
    }
    Ok(parts.join("/"))
}

fn rewrite_link(
    raw: &str,
    link_type: LinkType,
    destination: &str,
    title: &str,
    source_key: &str,
    index: &NoteIndex,
) -> Result<Option<String>> {
    let image = raw.starts_with('!');
    let wiki = matches!(link_type, LinkType::WikiLink { .. });
    let target = destination.trim().trim_end_matches('\\');
    let is_external = if image {
        target.starts_with("https://") || target.starts_with("http://")
    } else {
        external(target) || matches!(link_type, LinkType::Email)
    };
    if !wiki && is_external {
        if matches!(
            link_type,
            LinkType::Reference | LinkType::Collapsed | LinkType::Shortcut
        ) {
            let label = markdown_label(raw, image)?;
            let title = if title.is_empty() {
                String::new()
            } else {
                format!(" \"{}\"", title.replace('\\', "\\\\").replace('"', "\\\""))
            };
            return Ok(Some(format!(
                "{}[{}](<{}>{title})",
                if image { "!" } else { "" },
                label,
                target.replace('<', "%3C").replace('>', "%3E")
            )));
        }
        return Ok(None);
    }
    let (target, anchor) = parse_target(target, wiki)?;
    let target = target.as_str();
    let anchor = anchor.as_deref();
    // Markdown note embeds must name a .md file; image.png must not resolve to image.png.md.
    if image
        && !wiki
        && !Path::new(target)
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("md"))
    {
        return Err(ExportError::invalid_input(
            "local images are not supported; use an HTTP(S) URL uploaded with S3 Image Uploader",
        ));
    }
    let note = index.resolve(source_key, target, wiki)?.ok_or_else(|| {
        ExportError::invalid_input(if image {
            "unresolved public note embed; for images, use an HTTP(S) URL uploaded with S3 Image Uploader"
        } else {
            "missing, non-public or ambiguous note reference"
        })
    })?;
    if let Some(anchor) = anchor
        && note.headings.get(anchor) != Some(&1)
    {
        return Err(ExportError::invalid_input(
            "missing or ambiguous public heading reference",
        ));
    }
    let anchor = anchor
        .map(|a| format!("#section-{}", &digest(a).as_str()[..12]))
        .unwrap_or_default();
    let href = format!("content:{}{anchor}", note.id);
    let label = if wiki {
        let inner = raw
            .trim_start_matches('!')
            .trim_start_matches("[[")
            .strip_suffix("]]")
            .unwrap_or_default();
        let alias = inner.split_once('|').map(|(_, l)| l).unwrap_or(&note.title);
        escape_unescaped_brackets(alias)
    } else {
        markdown_label(raw, image)?.to_owned()
    };
    Ok(Some(format!("[{label}]({href})")))
}

fn external(target: &str) -> bool {
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|prefix| target.starts_with(prefix))
}

fn parse_target(target: &str, wiki: bool) -> Result<(String, Option<String>)> {
    let (target, anchor) = target
        .split_once('#')
        .map(|(t, a)| (t, Some(a)))
        .unwrap_or((target, None));
    let decode = |value: &str| -> Result<String> {
        if wiki {
            Ok(value.to_owned())
        } else {
            Ok(percent_encoding::percent_decode_str(value)
                .decode_utf8()?
                .into_owned())
        }
    };
    let target = decode(target)?;
    let anchor = anchor.map(decode).transpose()?;
    if target.contains(['\\', '\0']) || target.starts_with('/') || target.contains("://") {
        return Err(ExportError::invalid_input(
            "private or unsupported reference",
        ));
    }
    Ok((target, anchor))
}

fn markdown_label(raw: &str, image: bool) -> Result<&str> {
    let start = usize::from(image) + 1;
    let mut opaque = Parser::new_ext(raw, options())
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(
                event,
                Event::Code(_) | Event::InlineHtml(_) | Event::Html(_)
            )
            .then_some(range)
        })
        .peekable();
    let mut depth = 1;
    let mut escaped = false;
    for (index, ch) in raw.char_indices().filter(|(index, _)| *index >= start) {
        while opaque.peek().is_some_and(|range| range.end <= index) {
            opaque.next();
        }
        if opaque.peek().is_some_and(|range| range.contains(&index)) {
            continue;
        }
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&raw[start..index]);
                }
            }
            _ => {}
        }
    }
    Err(ExportError::invalid_input("unsupported link syntax"))
}

fn escape_unescaped_brackets(label: &str) -> String {
    let mut escaped = false;
    let mut result = String::new();
    for ch in label.chars() {
        if !escaped && matches!(ch, '[' | ']') {
            result.push('\\');
        }
        result.push(ch);
        escaped = !escaped && ch == '\\';
    }
    if escaped {
        result.push('\\'); // Do not let a trailing slash escape the new label terminator.
    }
    result
}

fn heading_text(markdown: &str) -> String {
    Parser::new_ext(markdown, options())
        .filter_map(|event| match event {
            Event::Text(text) | Event::Code(text) => Some(text.to_string()),
            _ => None,
        })
        .collect::<String>()
}

fn validate_html(html: &str) -> Result<()> {
    let fragment = scraper::Html::parse_fragment(html);
    for element in fragment.tree.nodes().filter_map(scraper::ElementRef::wrap) {
        for (name, value) in element.value().attrs() {
            if (["href", "src", "poster", "data"].contains(&name) && !external(value))
                || ["srcset", "style"].contains(&name)
            {
                return Err(ExportError::invalid_input(
                    "raw HTML local references are not supported; use public Markdown note links or HTTP(S) image URLs",
                ));
            }
        }
    }
    Ok(())
}

fn apply_edits(body: &str, mut edits: Vec<(Range<usize>, String)>) -> Result<String> {
    edits.sort_by_key(|(range, _)| (range.start, range.end));
    let mut result = String::with_capacity(body.len());
    let mut cursor = 0;
    for (range, value) in edits {
        if range.start < cursor {
            return Err(ExportError::invalid_input(
                "nested links/images require separate Markdown references",
            ));
        }
        result.push_str(&body[cursor..range.start]);
        result.push_str(&value);
        cursor = range.end;
    }
    result.push_str(&body[cursor..]);
    Ok(result)
}

fn remove_reference_definitions(mut body: String) -> Result<String> {
    // All resolved references are inline now. Repeat to remove shadowed
    // duplicate definitions too; parser ranges leave fenced examples intact.
    loop {
        let parser = Parser::new_ext(&body, options());
        let definitions: Vec<_> = parser
            .reference_definitions()
            .iter()
            .map(|(_, definition)| (definition.span.clone(), String::new()))
            .collect();
        if definitions.is_empty() {
            return Ok(body);
        }
        body = apply_edits(&body, definitions)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Document;
    use indoc::{formatdoc, indoc};

    fn note(key: &str, body: &str) -> VaultNote {
        VaultNote {
            key: key.into(),
            document: Document::parse(&formatdoc! {"
                ---
                schema_version: 1
                id: test
                locale: ja
                kind: article
                title: {key}
                category: tech
                created: '2025-01-01T00:00:00+09:00'
                updated: '2025-01-01T00:00:00+09:00'
                source_hash: '{}'
                ---
                {body}", digest(key)})
            .unwrap(),
        }
    }

    #[rstest::rstest]
    #[case("tech/source", "target", true, &["tech/target", "other/target"], Some("tech/target"))]
    #[case("tech/source", "other/target", true, &["other/target"], Some("other/target"))]
    #[case("tech/source", "target", true, &["other/target"], Some("other/target"))]
    #[case("source", "target", true, &["target", "a/target", "b/target"], Some("target"))]
    #[case("tech/source", "", true, &["tech/source", "source"], Some("tech/source"))]
    #[case("tech/source", "target", false, &["other/target"], None)]
    fn note_resolution_uses_exact_paths_before_wiki_basename_fallback(
        #[case] source: &str,
        #[case] target: &str,
        #[case] wiki: bool,
        #[case] keys: &[&str],
        #[case] expected: Option<&str>,
    ) {
        let notes: Vec<_> = keys.iter().map(|key| note(key, "")).collect();
        let index = NoteIndex::new(&notes);
        let resolved = index.resolve(source, target, wiki).unwrap();
        assert_eq!(resolved.map(|note| note.title.as_str()), expected);
    }

    #[rstest::rstest]
    #[case(&["tech/target", "target"])]
    #[case(&["a/target", "b/target"])]
    #[case(&["tech/target", "tech/target"])]
    fn wiki_resolution_rejects_multiple_candidates(#[case] keys: &[&str]) {
        let notes: Vec<_> = keys.iter().map(|key| note(key, "")).collect();
        let index = NoteIndex::new(&notes);
        assert!(matches!(
            index.resolve("tech/source", "target", true),
            Err(ExportError::InvalidInput(message)) if message.contains("ambiguous note"),
        ));
    }

    #[test]
    fn formatted_duplicate_headings_get_distinct_anchors_but_cannot_be_linked() {
        let body = indoc! {"
            # **同じ** `見出し`

            ## 同じ 見出し
        "};
        let mut notes = [note("source", body)];
        normalize(&mut notes).unwrap();
        let hash = digest("同じ 見出し");
        let hash = &hash.as_str()[..12];
        assert_eq!(
            notes[0].document.body,
            formatdoc! {"
                <a id=\"section-{hash}\"></a>

                # **同じ** `見出し`

                <a id=\"section-{hash}-2\"></a>

                ## 同じ 見出し
            "},
        );

        let mut notes = [note(
            "source",
            &formatdoc! {"
            {body}
            [[#同じ 見出し]]
        "},
        )];
        assert!(matches!(
            normalize(&mut notes),
            Err(ExportError::InvalidInput(message)) if message.contains("ambiguous public heading"),
        ));
    }

    #[test]
    fn edits_preserve_unicode_and_allow_an_insertion_at_a_replacement_start() {
        assert_eq!(
            apply_edits("前旧後", vec![(3..6, "新".into()), (3..3, "挿入".into())],).unwrap(),
            "前挿入新後",
        );
    }

    #[test]
    fn nested_link_and_image_rewrites_are_rejected() {
        let mut notes = [
            note("source", "[![note](target.md)](target.md)"),
            note("target", ""),
        ];
        assert!(matches!(
            normalize(&mut notes),
            Err(ExportError::InvalidInput(message)) if message.contains("nested links/images"),
        ));
    }
}
