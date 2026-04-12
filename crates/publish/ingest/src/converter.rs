use crate::error::Result;
use pulldown_cmark::{Event, Options, Parser, html};
use regex::Regex;
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::LazyLock,
};

/// Allow-list regex for bookmark blocks; anything beyond `<a href="URL">TITLE</a>` is escaped.
static SAFE_BOOKMARK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\A\s*<div class="bookmark">\s*<a href="[^"]+">[^<]*</a>\s*</div>\s*\z"#)
        .expect("Invalid safe bookmark regex")
});
static HREF_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]*)""#).expect("Invalid href regex"));

/// Mapping from an Obsidian file path without extension to a published article href.
pub type FileMapping = HashMap<String, String>;

pub fn convert_markdown_to_html(markdown_content: &str) -> Result<String> {
    let (markdown_content, katex_placeholders) = extract_katex_placeholders(markdown_content);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(&markdown_content, options);
    let mut html_output = String::with_capacity(markdown_content.len() * 2);
    html::push_html(&mut html_output, sanitize_html(parser).into_iter());
    let html_output = sanitize_anchor_hrefs(&html_output);
    let html_with_katex = replace_katex_placeholders(&html_output, &katex_placeholders);
    let html_with_strong = repair_unparsed_strong_markers(&html_with_katex);

    Ok(html_with_strong)
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

fn extract_katex_placeholders(markdown: &str) -> (String, Vec<KatexPlaceholder>) {
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
            in_html_tag = true;
            output.push(ch);
            line_prefix_is_whitespace = false;
            continue;
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

        if chars.peek() == Some(&'$') {
            chars.next();
            if let Some(content) = take_until_delimiter(&mut chars, "$$") {
                let token = build_katex_token(placeholders.len(), KatexMode::Display, &content);
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
            let token = build_katex_token(placeholders.len(), KatexMode::Inline, &content);
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

fn build_katex_token(index: usize, mode: KatexMode, content: &str) -> String {
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

fn replace_katex_placeholders(html: &str, placeholders: &[KatexPlaceholder]) -> String {
    if placeholders.is_empty() {
        return html.to_string();
    }

    let replacements = placeholders
        .iter()
        .map(|placeholder| {
            let content = html_escape(&normalize_katex_content(&placeholder.content));
            let replacement = match placeholder.mode {
                KatexMode::Inline => format!(r#"<span class="katex-inline">{content}</span>"#),
                KatexMode::Display => format!(r#"<span class="katex-display">{content}</span>"#),
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
        .replace_all(html, |caps: &regex::Captures<'_>| {
            let token = caps
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

fn normalize_katex_content(content: &str) -> String {
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
        Regex::new(r#"(?s)\*\*((?:<span class="katex-(?:inline|display)">.*?</span>))\*\*"#)
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

fn sanitize_anchor_hrefs(html: &str) -> String {
    HREF_ATTR_RE
        .replace_all(html, |caps: &regex::Captures| {
            let href = &caps[1];
            let sanitized_href = if is_safe_href(href) { href } else { "#" };
            format!("href=\"{sanitized_href}\"")
        })
        .to_string()
}

fn is_safe_href(href: &str) -> bool {
    let href = href.trim();

    if href.is_empty() {
        return false;
    }

    if href.starts_with('#') {
        return true;
    }

    if href.starts_with('/') {
        return !href.starts_with("//");
    }

    if href.starts_with("http://") || href.starts_with("https://") || href.starts_with("mailto:") {
        return true;
    }

    !href.contains(':') && !href.contains('\\') && !href.starts_with('.')
}

/// Escapes all raw HTML events except valid `<div class="bookmark">` blocks.
/// Accumulates each potential bookmark block and validates with SAFE_BOOKMARK_RE before passing through.
fn sanitize_html<'a>(parser: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut result: Vec<Event<'a>> = Vec::new();
    let mut in_bookmark = false;
    let mut bookmark_buffer = String::new();

    for event in parser {
        match event {
            Event::Html(html) | Event::InlineHtml(html) => {
                if !in_bookmark && !html.trim_start().starts_with(r#"<div class="bookmark">"#) {
                    result.push(Event::Text(html));
                } else {
                    if !in_bookmark {
                        in_bookmark = true;
                    }
                    bookmark_buffer.push_str(&html);

                    if let Some(close) = bookmark_buffer.find("</div>") {
                        in_bookmark = false;
                        let safe_end = close + "</div>".len();
                        let bookmark_part = bookmark_buffer[..safe_end].to_string();
                        let rest = bookmark_buffer[safe_end..].to_string();
                        bookmark_buffer.clear();

                        if SAFE_BOOKMARK_RE.is_match(&bookmark_part) {
                            result.push(Event::Html(bookmark_part.into()));
                        } else {
                            result.push(Event::Text(bookmark_part.into()));
                        }

                        if !rest.is_empty() {
                            result.push(Event::Text(rest.into()));
                        }
                    }
                }
            }
            other => {
                if in_bookmark {
                    in_bookmark = false;
                    let buffer = std::mem::take(&mut bookmark_buffer);
                    if !buffer.is_empty() {
                        result.push(Event::Text(buffer.into()));
                    }
                }
                result.push(other);
            }
        }
    }

    // Unclosed bookmark: flush buffer as escaped text.
    if !bookmark_buffer.is_empty() {
        result.push(Event::Text(bookmark_buffer.into()));
    }

    result
}

/// Convert Obsidian wiki links to published HTML links.
pub fn convert_obsidian_links(content: &str, file_mapping: &FileMapping) -> String {
    static OBSIDIAN_LINK_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("Invalid regex pattern"));

    OBSIDIAN_LINK_REGEX
        .replace_all(content, |caps: &regex::Captures| {
            let link_content = &caps[1];

            let (link_target, display_text) = if let Some(pipe_pos) = link_content.find('|') {
                let (link, display) = link_content.split_at(pipe_pos);
                (link.trim(), display[1..].trim())
            } else {
                (link_content.trim(), link_content.trim())
            };

            let href = if let Some(href) = file_mapping.get(link_target) {
                href.clone()
            } else {
                let mut found = false;
                let mut result_href = format!("/{link_target}");

                for (key, href) in file_mapping {
                    if key.ends_with(&format!("/{link_target}")) || key == link_target {
                        result_href = href.clone();
                        found = true;
                        break;
                    }
                }

                if !found {
                    log::warn!("Warning: Link target '{link_target}' not found in file mapping");
                }

                result_href
            };

            format!(
                "[{}]({})",
                escape_markdown_link_text(display_text),
                escape_markdown_link_destination(&href)
            )
        })
        .to_string()
}

fn escape_markdown_link_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_markdown_link_destination(destination: &str) -> String {
    destination.replace(')', "\\)")
}

pub fn generate_html_file(frontmatter_yaml: &str, html_body: &str) -> String {
    format!("---\n{frontmatter_yaml}\n---\n{html_body}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::*;

    #[rstest]
    #[case::basic_markdown(
        "# Hello World\n\nThis is a **bold** text and *italic* text.",
        "<h1>Hello World</h1>\n<p>This is a <strong>bold</strong> text and <em>italic</em> text.</p>\n"
    )]
    #[case::list_items(
        "- Item 1\n- Item 2\n- Item 3",
        "<ul>\n<li>Item 1</li>\n<li>Item 2</li>\n<li>Item 3</li>\n</ul>\n"
    )]
    #[case::code_block(
        "```rust\nfn main() {\n    println!(\"Hello!\");\n}\n```",
        "<pre><code class=\"language-rust\">fn main() {\n    println!(\"Hello!\");\n}\n</code></pre>\n"
    )]
    #[case::table_support(
        "| Col1 | Col2 |\n|------|------|\n| A    | B    |",
        "<table><thead><tr><th>Col1</th><th>Col2</th></tr></thead><tbody>\n<tr><td>A</td><td>B</td></tr>\n</tbody></table>\n"
    )]
    #[case::japanese_content(
        "# 日本語のタイトル\n\n**太字**のテキストです。",
        "<h1>日本語のタイトル</h1>\n<p><strong>太字</strong>のテキストです。</p>\n"
    )]
    fn test_markdown_to_html_conversion(#[case] markdown: &str, #[case] expected_html: &str) {
        let result = convert_markdown_to_html(markdown).unwrap();
        assert_eq!(result, expected_html);
    }

    #[rstest]
    fn test_obsidian_links_conversion() {
        let mut file_mapping = FileMapping::new();
        file_mapping.insert(
            "notes/another-note".to_string(),
            "/tech/abc123def".to_string(),
        );
        file_mapping.insert("docs/filename".to_string(), "/daily/xyz789abc".to_string());
        file_mapping.insert("Another Note".to_string(), "/tech/abc123def".to_string());
        file_mapping.insert("filename".to_string(), "/daily/xyz789abc".to_string());

        let result =
            convert_obsidian_links("Check out [[Another Note]] for more info.", &file_mapping);
        assert_eq!(
            result,
            "Check out [Another Note](/tech/abc123def) for more info."
        );

        let result =
            convert_obsidian_links("See [[filename|Custom Display Text]] here.", &file_mapping);
        assert_eq!(result, "See [Custom Display Text](/daily/xyz789abc) here.");

        let result = convert_obsidian_links("Link to [[nonexistent]] file.", &file_mapping);
        assert_eq!(result, "Link to [nonexistent](/nonexistent) file.");

        let result =
            convert_obsidian_links("This is normal text with no special links.", &file_mapping);
        assert_eq!(result, "This is normal text with no special links.");
    }

    #[rstest]
    #[case::basic_html_file(
        "title: Test Article\nslug: test123",
        "<h1>Test Content</h1>\n<p>This is a paragraph.</p>",
        "---\ntitle: Test Article\nslug: test123\n---\n<h1>Test Content</h1>\n<p>This is a paragraph.</p>"
    )]
    #[case::with_japanese_content(
        "title: 日本語記事\nslug: japanese456",
        "<h1>日本語のタイトル</h1>\n<p>日本語のコンテンツです。</p>",
        "---\ntitle: 日本語記事\nslug: japanese456\n---\n<h1>日本語のタイトル</h1>\n<p>日本語のコンテンツです。</p>"
    )]
    fn test_html_file_generation(
        #[case] frontmatter: &str,
        #[case] html_body: &str,
        #[case] expected: &str,
    ) {
        let result = generate_html_file(frontmatter, html_body);
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_combined_conversion() {
        let markdown_with_obsidian_links = r#"# My Article

This is a test with [[Another Article|link]] and **bold** text.

## Section Two

- Item with [[Reference Note]]
- Regular item"#;

        let mut file_mapping = FileMapping::new();
        file_mapping.insert("Another Article".to_string(), "/tech/def456".to_string());
        file_mapping.insert("Reference Note".to_string(), "/daily/ghi789".to_string());

        let with_html_links = convert_obsidian_links(markdown_with_obsidian_links, &file_mapping);
        let html = convert_markdown_to_html(&with_html_links).unwrap();

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/tech/def456\">link</a>"));
        assert!(html.contains("<a href=\"/daily/ghi789\">Reference Note</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }

    #[rstest]
    #[case::inline_math(
        "Here is some inline math: $x^2 + y^2 = z^2$ and more text.",
        "<p>Here is some inline math: <span class=\"katex-inline\">x^2 + y^2 = z^2</span> and more text.</p>\n"
    )]
    #[case::display_math(
        "Here is display math:\n$$\\int_0^1 x^2 dx = \\frac{1}{3}$$\nEnd of math.",
        "<p>Here is display math:\n<span class=\"katex-display\">\\int_0^1 x^2 dx = \\frac{1}{3}</span>\nEnd of math.</p>\n"
    )]
    #[case::mixed_math(
        "Inline $a+b$ and display $$c+d$$ math.",
        "<p>Inline <span class=\"katex-inline\">a+b</span> and display <span class=\"katex-display\">c+d</span> math.</p>\n"
    )]
    fn test_katex_math_processing(#[case] input: &str, #[case] expected: &str) {
        let result = convert_markdown_to_html(input).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_bold_text_around_math_is_preserved() {
        let markdown = "この時に使う考え方が、**「サンプリング」**と**「モデル化」**です。\n\nその身長を**$x = (x_1, x_2)$**と書きます。";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains("<strong>「サンプリング」</strong>と<strong>「モデル化」</strong>"),
            "unexpected html:\n{result}"
        );
        assert!(
            result.contains("<strong><span class=\"katex-inline\">x = (x_1, x_2)</span></strong>")
        );
        assert!(!result.contains("**"));
    }

    #[test]
    fn test_escaped_strong_markers_are_not_repaired() {
        let markdown = r#"これは \*\*literal\*\* です。"#;

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("**literal**"));
        assert!(!result.contains("<strong>literal</strong>"));
    }

    #[test]
    fn test_katex_content_normalization_removes_invisible_unicode() {
        let markdown = "inline $x\u{200B} + y\u{FEFF}$ and $$a\u{200C} + b\u{200D}$$";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains(r#"<span class="katex-inline">x + y</span>"#));
        assert!(result.contains(r#"<span class="katex-display">a + b</span>"#));
        assert!(!result.contains('\u{200B}'));
        assert!(!result.contains('\u{200C}'));
        assert!(!result.contains('\u{200D}'));
        assert!(!result.contains('\u{FEFF}'));
    }

    #[test]
    fn test_katex_placeholders_skip_inline_code_with_longer_backtick_delimiter() {
        let markdown = "``code with `$x$` inside`` and real math $y$";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("<code>code with `$x$` inside</code>"));
        assert!(result.contains(r#"<span class="katex-inline">y</span>"#));
        assert!(!result.contains(r#"<span class="katex-inline">x</span>"#));
    }

    #[test]
    fn test_katex_placeholders_skip_backticks_inside_fenced_code() {
        let markdown = "```text\nliteral ``` and $x$\n```\noutside $y$";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result
                .contains("<pre><code class=\"language-text\">literal ``` and $x$\n</code></pre>")
        );
        assert!(result.contains(r#"<span class="katex-inline">y</span>"#));
        assert!(!result.contains(r#"<span class="katex-inline">x</span>"#));
    }

    #[rstest]
    fn test_markdown_link_escaping() {
        let mut file_mapping = FileMapping::new();
        file_mapping.insert("File with <script>".to_string(), "/tech/abc123".to_string());

        let result = convert_obsidian_links("[[File with <script>|Display & test]]", &file_mapping);
        assert_eq!(result, "[Display & test](/tech/abc123)");

        let result =
            convert_obsidian_links("[[File \"quoted\"|Text with 'quotes']]", &file_mapping);
        assert_eq!(result, "[Text with 'quotes'](/File \"quoted\")");
    }

    #[rstest]
    fn test_markdown_to_html_escapes_raw_html() {
        let markdown = "<script>alert('xss')</script>\n\nHello <span>world</span>";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("&lt;script&gt;alert('xss')&lt;/script&gt;"));
        assert!(result.contains("Hello &lt;span&gt;world&lt;/span&gt;"));
        assert!(!result.contains("<script>"));
        assert!(!result.contains("<span>world</span>"));
    }

    #[test]
    fn test_markdown_to_html_sanitizes_javascript_href() {
        let markdown = "[click](javascript:alert('xss'))";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains("href=\"#\""),
            "unsafe href should be neutralized"
        );
        assert!(!result.contains("javascript:alert"));
    }

    #[test]
    fn test_katex_placeholders_do_not_rewrite_link_destinations() {
        let markdown = "[example](https://example.com/search?q=$x$)";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains(r#"href="https://example.com/search?q=$x$""#));
        assert!(!result.contains("katex-inline"));
    }

    #[test]
    fn test_katex_placeholders_do_not_rewrite_raw_html_attributes() {
        let markdown = r#"<img src="https://example.com/$x$.png" alt="img">"#;

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("&lt;img"));
        assert!(result.contains("$x$.png"));
        assert!(!result.contains("katex-inline"));
    }

    // -----------------------------------------------------------------
    // bookmark sanitize_html tests
    // -----------------------------------------------------------------

    /// The full bookmark block (opening tag, inner content, closing tag)
    /// must pass through `convert_markdown_to_html` without any HTML escaping
    /// so that the downstream `convert_simple_bookmarks_to_rich` can find it.
    #[rstest]
    fn test_bookmark_html_passes_through_unescaped() {
        // Multi-line bookmark – pulldown-cmark emits this as several
        // `Event::Html` events (one per line), so the stateful filter must
        // keep all of them unescaped.
        let markdown = "<div class=\"bookmark\">\n  <a href=\"https://example.com\">Example Site</a>\n</div>\n";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains("<div class=\"bookmark\">"),
            "bookmark opening tag should not be escaped; got:\n{result}"
        );
        assert!(
            result.contains(r#"<a href="https://example.com">"#),
            "bookmark anchor tag should not be escaped; got:\n{result}"
        );
        assert!(
            result.contains("</div>"),
            "bookmark closing tag should not be escaped; got:\n{result}"
        );
        assert!(
            !result.contains("&lt;div"),
            "no HTML entities expected for bookmark block; got:\n{result}"
        );
    }

    #[test]
    fn test_bookmark_html_sanitizes_unsafe_href_scheme() {
        let markdown = "<div class=\"bookmark\">\n  <a href=\"javascript:alert('xss')\">Example Site</a>\n</div>\n";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("<a href=\"#\">"));
        assert!(!result.contains("javascript:alert"));
    }

    /// A single-line bookmark (`<div class="bookmark">…</div>` on one line)
    /// must also pass through unescaped.
    #[rstest]
    fn test_single_line_bookmark_passes_through_unescaped() {
        let markdown =
            "<div class=\"bookmark\"><a href=\"https://example.com\">Example</a></div>\n";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains("<div class=\"bookmark\">"),
            "single-line bookmark should not be escaped; got:\n{result}"
        );
        assert!(
            !result.contains("&lt;div"),
            "no HTML entities expected; got:\n{result}"
        );
    }

    /// Raw HTML that is NOT a bookmark must still be escaped (XSS protection).
    #[rstest]
    #[case::div_with_other_class("<div class=\"custom\"><span>hello</span></div>")]
    #[case::script_tag("<script>alert('xss')</script>")]
    #[case::inline_span("<span>inline</span>")]
    fn test_non_bookmark_raw_html_is_still_escaped(#[case] markdown: &str) {
        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains("&lt;"),
            "non-bookmark HTML should be escaped; got:\n{result}"
        );
        // No literal opening angle bracket from the raw HTML should survive.
        // We cannot check for a specific tag because the test is parameterised,
        // but the presence of `&lt;` proves escaping happened.
    }

    /// Content AFTER a correctly closed bookmark block must NOT be affected by
    /// the bookmark filter (i.e. regular HTML after the block is still escaped).
    #[rstest]
    fn test_html_after_bookmark_is_escaped() {
        let markdown = "<div class=\"bookmark\">\n  <a href=\"https://example.com\">X</a>\n</div>\n\n<script>bad()</script>\n";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains("<div class=\"bookmark\">"),
            "bookmark block should pass through; got:\n{result}"
        );
        assert!(
            !result.contains("<script>"),
            "script tag after bookmark should be escaped; got:\n{result}"
        );
        assert!(
            result.contains("&lt;script&gt;"),
            "script tag should appear as entities; got:\n{result}"
        );
    }

    /// Raw HTML on the same line after `</div>` must be escaped even though
    /// the bookmark block itself passes through (P1 XSS fix).
    #[rstest]
    fn test_trailing_content_after_bookmark_close_is_escaped() {
        let markdown = "<div class=\"bookmark\"><a href=\"https://example.com\">X</a></div><script>bad()</script>\n";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains(r#"<div class="bookmark">"#),
            "bookmark should pass through; got:\n{result}"
        );
        assert!(
            !result.contains("<script>"),
            "trailing script tag should be escaped; got:\n{result}"
        );
        assert!(
            result.contains("&lt;script&gt;"),
            "trailing script tag should appear as entities; got:\n{result}"
        );
    }

    /// Bookmark blocks that contain unexpected HTML (e.g. a `<script>` tag as a
    /// sibling of `<a>`, or extra event-handler attributes on `<a>`) must be
    /// HTML-escaped. Only the strict `<div class="bookmark"><a href="…">…</a></div>`
    /// structure may pass through as raw HTML.
    #[rstest]
    #[case::script_sibling(r#"<div class="bookmark"><script>alert('xss')</script></div>"#)]
    #[case::extra_attribute_on_anchor(
        r#"<div class="bookmark"><a href="https://example.com" onmouseover="alert(1)">Hover</a></div>"#
    )]
    #[case::nested_div_inside_bookmark(
        "<div class=\"bookmark\"><div><a href=\"https://example.com\">Title</a></div></div>"
    )]
    fn test_bookmark_with_unexpected_html_is_escaped(#[case] markdown: &str) {
        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            !result.contains(r#"<div class="bookmark">"#),
            "unexpected bookmark content should cause the block to be escaped; got:\n{result}"
        );
        assert!(
            result.contains("&lt;"),
            "escaped block should contain HTML entities; got:\n{result}"
        );
    }

    /// A bookmark block that is never closed with `</div>` must be HTML-escaped
    /// in its entirety. The filter must not leave `in_bookmark = true` at
    /// end-of-stream and silently discard the buffered content.
    #[rstest]
    fn test_unclosed_bookmark_is_escaped() {
        // Deliberately omit the closing </div>.
        let markdown = "<div class=\"bookmark\">\n  <a href=\"https://example.com\">Title</a>\n";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            !result.contains(r#"<div class="bookmark">"#),
            "unclosed bookmark opening tag should be escaped; got:\n{result}"
        );
        assert!(
            result.contains("&lt;div"),
            "unclosed bookmark should appear as HTML entities; got:\n{result}"
        );
    }

    /// When the markdown contains multiple bookmark blocks, every block must
    /// reach the output HTML unescaped so that the downstream bookmark enricher
    /// can find and convert each one. The stateful `in_bookmark` filter inside
    /// `sanitize_html` must reset correctly after each block closes.
    ///
    /// Three cases are exercised:
    /// - two bookmarks separated only by a blank line
    /// - two bookmarks with prose text between them
    /// - three consecutive bookmarks (verifies the flag resets more than once)
    #[rstest]
    #[case::two_bookmarks_blank_line_between(
        indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com">Example</a>
            </div>

            <div class="bookmark">
              <a href="https://github.com">GitHub</a>
            </div>
        "#}
    )]
    #[case::two_bookmarks_prose_between(
        indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com">Example</a>
            </div>

            Some prose text here.

            <div class="bookmark">
              <a href="https://github.com">GitHub</a>
            </div>
        "#}
    )]
    #[case::three_bookmarks_in_sequence(
        indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com">Example</a>
            </div>

            <div class="bookmark">
              <a href="https://github.com">GitHub</a>
            </div>

            <div class="bookmark">
              <a href="https://rust-lang.org">Rust</a>
            </div>
        "#}
    )]
    fn test_multiple_bookmarks_all_pass_through_unescaped(#[case] markdown: &str) {
        let result = convert_markdown_to_html(markdown).unwrap();

        let input_count = markdown.matches(r#"<div class="bookmark">"#).count();
        let output_count = result.matches(r#"<div class="bookmark">"#).count();

        assert_eq!(
            output_count, input_count,
            "all {input_count} bookmark block(s) should pass through unescaped, \
             but only {output_count} did; got:\n{result}"
        );
        assert!(
            !result.contains("&lt;div"),
            "no bookmark div should be HTML-escaped; got:\n{result}"
        );
    }

    // -----------------------------------------------------------------
    // KaTeX + code block tests
    // -----------------------------------------------------------------

    /// Inline code (backtick) containing `$...$` must NOT be converted to a
    /// KaTeX span – the dollar signs are part of the code, not math.
    #[rstest]
    fn test_katex_not_processed_in_inline_code() {
        let markdown = "See `$x^2$` for the formula.";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains("<code>$x^2$</code>"),
            "inline code content should not be KaTeX-processed; got:\n{result}"
        );
        assert!(
            !result.contains("katex-inline"),
            "no KaTeX span expected inside inline code; got:\n{result}"
        );
    }

    /// Fenced code blocks containing `$...$` or `$$...$$` must NOT produce
    /// any KaTeX wrappers.
    #[rstest]
    fn test_katex_not_processed_in_fenced_code_block() {
        let markdown = "```\n$x^2$ and $$block$$ formula\n```";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            !result.contains("katex-display"),
            "fenced code block should not produce KaTeX display; got:\n{result}"
        );
        assert!(
            !result.contains("katex-inline"),
            "fenced code block should not produce KaTeX inline; got:\n{result}"
        );
        assert!(
            result.contains("$x^2$"),
            "dollar signs should remain verbatim in code block; got:\n{result}"
        );
    }

    /// Math markers that appear OUTSIDE code elements must still be converted
    /// even when code elements are present in the same document.
    #[rstest]
    fn test_katex_processed_in_text_adjacent_to_code() {
        let markdown = "The formula $a+b$ is useful. Code: `$not_math$`.";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result.contains(r#"<span class="katex-inline">a+b</span>"#),
            "math outside code should be converted to KaTeX span; got:\n{result}"
        );
        assert!(
            result.contains("<code>$not_math$</code>"),
            "dollar signs inside inline code should remain untouched; got:\n{result}"
        );
    }

    #[rstest]
    #[case::inline_code("The formula $a+b$ is useful. Code: `$not_math$`.")]
    #[case::fenced_code(indoc! {r#"
        Before $a$.

        ```rust
        let literal = "$not_math$";
        ```

        After $b$.
    "#})]
    fn test_katex_placeholders_skip_code(#[case] markdown: &str) {
        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("katex-inline"));
        assert!(result.contains("$not_math$"));
        assert!(!result.contains("<span class=\"katex-inline\">not_math</span>"));
    }
}
