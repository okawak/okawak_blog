//! Resolve references using only the explicitly public source set.
use crate::vault::{Source, digest};
use anyhow::{Result, bail};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};

pub(crate) fn normalize(sources: &mut [Source], root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let index: Vec<_> = sources
        .iter()
        .map(|s| {
            let mut headings = BTreeMap::<String, usize>::new();
            for (event, range) in Parser::new_ext(&s.document.body, options()).into_offset_iter() {
                if matches!(event, Event::Start(Tag::Heading { .. })) {
                    *headings
                        .entry(heading_text(&s.document.body[range]))
                        .or_default() += 1;
                }
            }
            (
                s.key.clone(),
                s.document.meta.id.to_string(),
                s.document.meta.title.clone(),
                headings,
            )
        })
        .collect();
    let files = crate::markdown::files(root)?;
    let mut assets = BTreeMap::new();
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
                    let raw = &body[range.clone()];
                    let image = raw.starts_with('!');
                    let wiki = matches!(link_type, LinkType::WikiLink { .. });
                    let target = dest_url.trim().trim_end_matches('\\');
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
                            edits.push((
                                range,
                                format!(
                                    "{}[{}](<{}>{title})",
                                    if image { "!" } else { "" },
                                    label,
                                    target.replace('<', "%3C").replace('>', "%3E")
                                ),
                            ));
                        }
                        continue;
                    }
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
                    let target = target.as_str();
                    let anchor = anchor.as_deref();
                    if target.contains(['\\', '\0'])
                        || target.starts_with('/')
                        || target.contains("://")
                    {
                        bail!("private or unsupported reference");
                    }
                    let extensionless = if Path::new(target)
                        .extension()
                        .and_then(|s| s.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
                    {
                        &target[..target.len() - 3]
                    } else {
                        target
                    };
                    let relative = resolve_relative(&source.key, extensionless)?;
                    let exact: Vec<_> = index
                        .iter()
                        .filter(|(key, _, _, _)| {
                            if extensionless.is_empty() {
                                key == &source.key
                            } else {
                                key == &relative || (wiki && key == extensionless)
                            }
                        })
                        .collect();
                    // An explicit Markdown image names a file, even when a
                    // public note such as image.png.md has the same stem.
                    let prefer_asset = image
                        && !wiki
                        && root.join(resolve_relative(&source.key, target)?).is_file();
                    let matches = if prefer_asset {
                        Vec::new()
                    } else if wiki && exact.is_empty() {
                        index
                            .iter()
                            .filter(|(key, _, _, _)| key.rsplit('/').next() == Some(extensionless))
                            .collect()
                    } else {
                        exact
                    };
                    let mut default_label = String::new();
                    let href = match matches.as_slice() {
                        [(_, id, title, headings)] => {
                            default_label = title.clone();
                            if let Some(anchor) = anchor
                                && headings.get(anchor) != Some(&1)
                            {
                                bail!("missing or ambiguous public heading reference");
                            }
                            let anchor = anchor
                                .map(|a| format!("#section-{}", &digest(a)[..12]))
                                .unwrap_or_default();
                            format!("content:{id}{anchor}")
                        }
                        [] if image && anchor.is_none() => {
                            let relative = resolve_relative(&source.key, target)?;
                            let matches: Vec<_> = files
                                .iter()
                                .filter(|p| {
                                    let key = p.strip_prefix(root).unwrap().to_string_lossy();
                                    key == relative
                                        || (wiki
                                            && (key == target
                                                || p.file_name().and_then(|p| p.to_str())
                                                    == Some(target)))
                                })
                                .collect();
                            let [path] = matches.as_slice() else {
                                bail!("missing or ambiguous public image");
                            };
                            let extension = path
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or_default()
                                .to_ascii_lowercase();
                            if !["png", "jpg", "jpeg", "gif", "webp", "avif"]
                                .contains(&extension.as_str())
                            {
                                bail!("unsupported public image format");
                            }
                            let data = fs::read(path)?;
                            let name = format!("{}.{}", digest(&data), extension);
                            assets.insert(name.clone(), data);
                            format!("/content-assets/{name}")
                        }
                        _ => bail!("missing, non-public or ambiguous note reference"),
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
                    edits.push((range, format!("{marker}[{label}]({href})")));
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
                            &digest(heading)[..12]
                        ),
                    ));
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    let fragment = scraper::Html::parse_fragment(&html);
                    let selector = scraper::Selector::parse("*").expect("static selector");
                    for element in fragment.select(&selector) {
                        for (name, value) in element.value().attrs() {
                            if (["href", "src", "poster", "data"].contains(&name)
                                && !external(value))
                                || ["srcset", "style"].contains(&name)
                            {
                                bail!(
                                    "raw HTML local references must use Markdown links/images before export"
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        edits.sort_by_key(|(range, _)| (range.start, range.end));
        for pair in edits.windows(2) {
            if pair[0].0.end > pair[1].0.start {
                bail!("nested links/images require separate Markdown references");
            }
        }
        let mut normalized = body.clone();
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
        source.document.body = normalized;
    }
    Ok(assets)
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
    bail!("unsupported link syntax")
}

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH
        | Options::ENABLE_WIKILINKS
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
            _ => bail!("reference escapes public source root"),
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
