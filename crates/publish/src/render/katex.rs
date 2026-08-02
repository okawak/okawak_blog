use regex::Regex;
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::LazyLock,
};

pub(super) struct ProtectedMarkdown {
    markdown: String,
    placeholders: Vec<KatexPlaceholder>,
}

impl ProtectedMarkdown {
    pub(super) fn new(markdown: &str) -> Self {
        let (markdown, placeholders) = extract_placeholders(markdown);
        Self {
            markdown,
            placeholders,
        }
    }

    pub(super) fn as_str(&self) -> &str {
        &self.markdown
    }

    pub(super) fn restore(self, html: &str) -> String {
        let html = replace_placeholders(html, &self.placeholders);
        repair_unparsed_strong_markers(&html)
    }
}

#[derive(Clone, Copy)]
enum KatexMode {
    Inline,
    Display,
}

struct KatexPlaceholder {
    token: String,
    content: String,
    mode: KatexMode,
}

#[derive(Clone, Copy)]
enum CodeState {
    Outside,
    Inline(usize),
    Fenced(usize),
}

#[derive(Clone, Copy)]
enum LinkState {
    Outside,
    AfterClosingBracket,
    Destination(usize),
}

fn extract_placeholders(markdown: &str) -> (String, Vec<KatexPlaceholder>) {
    let mut placeholders = Vec::new();
    let mut output = String::with_capacity(markdown.len());
    let mut chars = markdown.chars().peekable();
    let mut code_state = CodeState::Outside;
    let mut link_state = LinkState::Outside;
    let mut line_prefix_is_whitespace = true;
    let mut in_html_tag = false;

    while let Some(ch) = chars.next() {
        if in_html_tag {
            output.push(ch);
            if ch == '>' {
                in_html_tag = false;
            }
            line_prefix_is_whitespace =
                ch == '\n' || (line_prefix_is_whitespace && ch.is_whitespace());
            continue;
        }

        if ch == '`' {
            let mut tick_count = 1;
            while chars.peek() == Some(&'`') {
                chars.next();
                tick_count += 1;
            }

            match code_state {
                CodeState::Outside => {
                    if tick_count >= 3 && line_prefix_is_whitespace {
                        code_state = CodeState::Fenced(tick_count);
                    } else {
                        code_state = CodeState::Inline(tick_count);
                    }
                }
                CodeState::Inline(delimiter_len) => {
                    if tick_count == delimiter_len {
                        code_state = CodeState::Outside;
                    }
                }
                CodeState::Fenced(delimiter_len) => {
                    if line_prefix_is_whitespace && tick_count >= delimiter_len {
                        code_state = CodeState::Outside;
                    }
                }
            }

            for _ in 0..tick_count {
                output.push('`');
            }
            line_prefix_is_whitespace = false;
            continue;
        }

        if !matches!(code_state, CodeState::Outside) {
            output.push(ch);
            line_prefix_is_whitespace =
                ch == '\n' || (line_prefix_is_whitespace && ch.is_whitespace());
            continue;
        }

        if ch == '<' {
            let starts_html_tag = matches!(
                chars.peek(),
                Some(next) if next.is_ascii_alphabetic() || matches!(next, '/' | '!' | '?')
            );
            if starts_html_tag {
                in_html_tag = true;
                output.push(ch);
                line_prefix_is_whitespace = false;
                continue;
            }
        }

        match link_state {
            LinkState::Outside => {
                if ch == ']' {
                    output.push(ch);
                    link_state = LinkState::AfterClosingBracket;
                    line_prefix_is_whitespace = false;
                    continue;
                }
            }
            LinkState::AfterClosingBracket => {
                output.push(ch);
                if ch == '(' {
                    link_state = LinkState::Destination(1);
                } else {
                    link_state = LinkState::Outside;
                }
                line_prefix_is_whitespace = false;
                continue;
            }
            LinkState::Destination(depth) => {
                output.push(ch);
                link_state = match ch {
                    '(' => LinkState::Destination(depth + 1),
                    ')' if depth == 1 => LinkState::Outside,
                    ')' => LinkState::Destination(depth - 1),
                    _ => LinkState::Destination(depth),
                };
                line_prefix_is_whitespace = false;
                continue;
            }
        }

        if ch != '$' {
            output.push(ch);
            line_prefix_is_whitespace =
                ch == '\n' || (line_prefix_is_whitespace && ch.is_whitespace());
            continue;
        }

        let preceding_backslash_count = output.chars().rev().take_while(|c| *c == '\\').count();
        if preceding_backslash_count % 2 == 1 {
            output.push(ch);
            line_prefix_is_whitespace = false;
            continue;
        }

        if chars.peek() == Some(&'$') {
            chars.next();
            if let Some(content) = take_until_delimiter(&mut chars, "$$") {
                let token = build_token(placeholders.len(), KatexMode::Display, &content);
                placeholders.push(KatexPlaceholder {
                    token: token.clone(),
                    content,
                    mode: KatexMode::Display,
                });
                output.push_str(&token);
                line_prefix_is_whitespace = false;
            } else {
                output.push_str("$$");
                line_prefix_is_whitespace = false;
            }
            continue;
        }

        if let Some(content) = take_until_delimiter(&mut chars, "$") {
            let token = build_token(placeholders.len(), KatexMode::Inline, &content);
            placeholders.push(KatexPlaceholder {
                token: token.clone(),
                content,
                mode: KatexMode::Inline,
            });
            output.push_str(&token);
            line_prefix_is_whitespace = false;
        } else {
            output.push('$');
            line_prefix_is_whitespace = false;
        }
    }

    (output, placeholders)
}

fn build_token(index: usize, mode: KatexMode, content: &str) -> String {
    let mut hasher = DefaultHasher::new();
    index.hash(&mut hasher);
    match mode {
        KatexMode::Inline => "inline".hash(&mut hasher),
        KatexMode::Display => "display".hash(&mut hasher),
    }
    content.hash(&mut hasher);

    format!("\u{E000}OKAWAKKATEX{:016x}\u{E001}", hasher.finish())
}

fn take_until_delimiter(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    delimiter: &str,
) -> Option<String> {
    let mut content = String::new();

    while let Some(ch) = chars.next() {
        if delimiter == "$$" && ch == '$' && chars.peek() == Some(&'$') {
            chars.next();
            return Some(content);
        }

        if delimiter == "$" && ch == '$' {
            return Some(content);
        }

        content.push(ch);
    }

    None
}

fn replace_placeholders(html: &str, placeholders: &[KatexPlaceholder]) -> String {
    if placeholders.is_empty() {
        return html.to_string();
    }

    let replacements = placeholders
        .iter()
        .map(|placeholder| {
            let content = html_escape(&normalize_content(&placeholder.content));
            let replacement = match placeholder.mode {
                KatexMode::Inline => {
                    format!(r#"<span class="okawak-katex-inline">{content}</span>"#)
                }
                KatexMode::Display => {
                    format!(r#"<span class="okawak-katex-display">{content}</span>"#)
                }
            };

            (placeholder.token.as_str(), replacement)
        })
        .collect::<HashMap<_, _>>();

    let token_pattern = placeholders
        .iter()
        .map(|placeholder| regex::escape(&placeholder.token))
        .collect::<Vec<_>>()
        .join("|");
    let token_re = Regex::new(&token_pattern).expect("Invalid KaTeX token regex");

    token_re
        .replace_all(html, |captures: &regex::Captures<'_>| {
            let token = captures
                .get(0)
                .expect("Regex match should always contain the full match")
                .as_str();
            replacements
                .get(token)
                .cloned()
                .unwrap_or_else(|| token.to_string())
        })
        .into_owned()
}

fn normalize_content(content: &str) -> String {
    content
        .chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{2009}'
                    | '\u{200A}'
                    | '\u{200B}'
                    | '\u{200C}'
                    | '\u{200D}'
                    | '\u{2061}'
                    | '\u{202F}'
                    | '\u{2060}'
                    | '\u{FEFF}'
            )
        })
        .collect()
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn repair_unparsed_strong_markers(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    loop {
        let code_start = [remaining.find("<pre"), remaining.find("<code")]
            .into_iter()
            .flatten()
            .min();

        match code_start {
            None => {
                result.push_str(&apply_unparsed_strong_markers(remaining));
                break;
            }
            Some(start) => {
                result.push_str(&apply_unparsed_strong_markers(&remaining[..start]));

                let close_tag = if remaining[start..].starts_with("<pre") {
                    "</pre>"
                } else {
                    "</code>"
                };

                match remaining[start..].find(close_tag) {
                    Some(close_offset) => {
                        let end = start + close_offset + close_tag.len();
                        result.push_str(&remaining[start..end]);
                        remaining = &remaining[end..];
                    }
                    None => {
                        result.push_str(&remaining[start..]);
                        break;
                    }
                }
            }
        }
    }

    repair_nested_adjacent_strong_tags(&result)
}

fn apply_unparsed_strong_markers(html: &str) -> String {
    static STRONG_KATEX_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?s)\*\*((?:<span class="okawak-katex-(?:inline|display)">.*?</span>))\*\*"#)
            .expect("Invalid KaTeX strong marker regex")
    });

    STRONG_KATEX_RE
        .replace_all(html, "<strong>$1</strong>")
        .into_owned()
}

fn repair_nested_adjacent_strong_tags(html: &str) -> String {
    static NESTED_STRONG_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<strong>([^<]+)<strong>([^<]+)</strong>([^<]+)</strong>")
            .expect("Invalid nested strong regex")
    });
    static RAW_STRONG_SPLIT_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\*\*([^*<]+)<strong>([^<]+)</strong>([^*<]+)\*\*")
            .expect("Invalid raw strong split regex")
    });

    let html = RAW_STRONG_SPLIT_RE
        .replace_all(html, "<strong>$1</strong>$2<strong>$3</strong>")
        .into_owned();

    NESTED_STRONG_RE
        .replace_all(&html, "<strong>$1</strong>$2<strong>$3</strong>")
        .into_owned()
}
