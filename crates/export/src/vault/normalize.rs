//! Resolve references using only the explicitly public source set.
use super::VaultNote;
use crate::{
    ExportError, Result,
    content::{digest, options},
};
use pulldown_cmark::{Event, LinkType, Parser, Tag};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};

pub(super) fn normalize(
    sources: &mut [VaultNote],
    root: &Path,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let index = NoteIndex::new(sources);
    let mut assets = Assets::new(root)?;
    for source in sources {
        let body = &source.document.body;
        let mut edits = Vec::new();
        let mut heading_counts = BTreeMap::<String, usize>::new();
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
                        &source.key,
                        &index,
                        &mut assets,
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
        source.document.body = apply_edits(body, edits)?;
    }
    Ok(assets.contents)
}

fn image_extension(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    ["png", "jpg", "jpeg", "gif", "webp", "avif"]
        .contains(&extension.as_str())
        .then_some(extension)
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

fn markdown_label(raw: &str, image: bool) -> Result<&str> {
    let start = usize::from(image) + 1;
    let opaque: Vec<_> = Parser::new_ext(raw, options())
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(
                event,
                Event::Code(_) | Event::InlineHtml(_) | Event::Html(_)
            )
            .then_some(range)
        })
        .collect();
    let mut depth = 1;
    let mut escaped = false;
    for (index, ch) in raw.char_indices().filter(|(index, _)| *index >= start) {
        if opaque.iter().any(|range| range.contains(&index)) {
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

fn external(target: &str) -> bool {
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|prefix| target.starts_with(prefix))
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

fn heading_text(markdown: &str) -> String {
    Parser::new_ext(markdown, options())
        .filter_map(|event| match event {
            Event::Text(text) | Event::Code(text) => Some(text.to_string()),
            _ => None,
        })
        .collect::<String>()
}

struct Note {
    key: String,
    id: String,
    title: String,
    headings: BTreeMap<String, usize>,
}

struct NoteIndex {
    notes: Vec<Note>,
}

impl NoteIndex {
    fn new(sources: &[VaultNote]) -> Self {
        let notes = sources
            .iter()
            .map(|s| {
                let mut headings = BTreeMap::<String, usize>::new();
                for (event, range) in
                    Parser::new_ext(&s.document.body, options()).into_offset_iter()
                {
                    if matches!(event, Event::Start(Tag::Heading { .. })) {
                        *headings
                            .entry(heading_text(&s.document.body[range]))
                            .or_default() += 1;
                    }
                }
                Note {
                    key: s.key.clone(),
                    id: s.document.meta.id.to_string(),
                    title: s.document.meta.title.to_string(),
                    headings,
                }
            })
            .collect();
        Self { notes }
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
        let relative = resolve_relative(source_key, extensionless)?;
        let exact: Vec<_> = self
            .notes
            .iter()
            .filter(|note| {
                let key = &note.key;
                if extensionless.is_empty() {
                    key == source_key
                } else {
                    key == &relative || (wiki && key == extensionless)
                }
            })
            .collect();
        let matches = if wiki && exact.is_empty() {
            self.notes
                .iter()
                .filter(|note| note.key.rsplit('/').next() == Some(extensionless))
                .collect()
        } else {
            exact
        };
        match matches.as_slice() {
            [] => Ok(None),
            [note] => Ok(Some(*note)),
            _ => Err(ExportError::invalid_input(
                "missing, non-public or ambiguous note reference",
            )),
        }
    }
}

struct Assets<'a> {
    root: &'a Path,
    files: Vec<std::path::PathBuf>,
    contents: BTreeMap<String, Vec<u8>>,
}

impl<'a> Assets<'a> {
    fn new(root: &'a Path) -> Result<Self> {
        Ok(Self {
            root,
            files: crate::filesystem::files(root)?,
            contents: BTreeMap::new(),
        })
    }

    // An explicit Markdown image names a file, even when image.png.md also exists.
    fn has_image(&self, source_key: &str, target: &str) -> Result<bool> {
        Ok(image_extension(Path::new(target)).is_some()
            && self
                .root
                .join(resolve_relative(source_key, target)?)
                .is_file())
    }

    fn resolve(&mut self, source_key: &str, target: &str, wiki: bool) -> Result<String> {
        let relative = resolve_relative(source_key, target)?;
        let matches: Vec<_> = self
            .files
            .iter()
            .filter(|p| {
                let key = p.strip_prefix(self.root).unwrap().to_string_lossy();
                key == relative
                    || (wiki
                        && (key == target
                            || p.file_name().and_then(|p| p.to_str()) == Some(target)))
            })
            .collect();
        let [path] = matches.as_slice() else {
            return Err(ExportError::invalid_input(
                "missing or ambiguous public image",
            ));
        };
        let Some(extension) = image_extension(path) else {
            return Err(ExportError::invalid_input(
                "unsupported public image format",
            ));
        };
        let data = fs::read(path)?;
        let name = format!("{}.{}", digest(&data), extension);
        self.contents.insert(name.clone(), data);
        Ok(format!("/content-assets/{name}"))
    }
}

fn rewrite_link(
    raw: &str,
    link_type: LinkType,
    destination: &str,
    title: &str,
    source_key: &str,
    index: &NoteIndex,
    assets: &mut Assets<'_>,
) -> Result<Option<String>> {
    let image = raw.starts_with('!');
    let wiki = matches!(link_type, LinkType::WikiLink { .. });
    let target = destination.trim().trim_end_matches('\\');
    if !wiki && (external(target) || matches!(link_type, LinkType::Email)) {
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
    let note = if image && !wiki && assets.has_image(source_key, target)? {
        None
    } else {
        index.resolve(source_key, target, wiki)?
    };
    let mut default_label = String::new();
    let href = match note {
        Some(note) => {
            default_label = note.title.clone();
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
            format!("content:{}{anchor}", note.id)
        }
        None if image && anchor.is_none() => assets.resolve(source_key, target, wiki)?,
        None => {
            return Err(ExportError::invalid_input(
                "missing, non-public or ambiguous note reference",
            ));
        }
    };
    let label = if wiki {
        let inner = raw
            .trim_start_matches('!')
            .trim_start_matches("[[")
            .strip_suffix("]]")
            .unwrap_or_default();
        let alias = inner
            .split_once('|')
            .map(|(_, l)| l)
            .unwrap_or(&default_label);
        escape_unescaped_brackets(alias)
    } else {
        markdown_label(raw, image)?.to_owned()
    };
    let marker = if image && href.starts_with("/content-assets/") {
        "!"
    } else {
        ""
    };
    Ok(Some(format!("{marker}[{label}]({href})")))
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

fn validate_html(html: &str) -> Result<()> {
    let fragment = scraper::Html::parse_fragment(html);
    let selector = scraper::Selector::parse("*").expect("static selector");
    for element in fragment.select(&selector) {
        for (name, value) in element.value().attrs() {
            if (["href", "src", "poster", "data"].contains(&name) && !external(value))
                || ["srcset", "style"].contains(&name)
            {
                return Err(ExportError::invalid_input(
                    "raw HTML local references must use Markdown links/images before export",
                ));
            }
        }
    }
    Ok(())
}

fn apply_edits(body: &str, mut edits: Vec<(std::ops::Range<usize>, String)>) -> Result<String> {
    edits.sort_by_key(|(range, _)| (range.start, range.end));
    for pair in edits.windows(2) {
        if pair[0].0.end > pair[1].0.start {
            return Err(ExportError::invalid_input(
                "nested links/images require separate Markdown references",
            ));
        }
    }
    let mut normalized = body.to_owned();
    for (range, value) in edits.into_iter().rev() {
        normalized.replace_range(range, &value);
    }
    // All resolved references are inline now. Repeat to remove shadowed
    // duplicate definitions too; parser ranges leave fenced examples intact.
    loop {
        let parser = Parser::new_ext(&normalized, options());
        let mut definitions: Vec<_> = parser
            .reference_definitions()
            .iter()
            .map(|(_, definition)| definition.span.clone())
            .collect();
        if definitions.is_empty() {
            break;
        }
        definitions.sort_by_key(|range| range.start);
        for range in definitions.into_iter().rev() {
            normalized.replace_range(range, "");
        }
    }
    Ok(normalized)
}
