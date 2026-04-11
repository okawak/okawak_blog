use crate::error::Result;
use pulldown_cmark::{Event, Options, Parser, html};
use regex::Regex;
use std::{collections::HashMap, sync::LazyLock};

/// Byte overhead added by a single `<span class="katex-inline">…</span>` wrapper
/// over the original `$…$` delimiters.
/// `<span class="katex-inline">` (27 bytes) + `</span>` (7 bytes) − `$` + `$` (2 bytes) = 32.
const KATEX_INLINE_OVERHEAD: usize = 32;

/// Byte overhead added by a single `<div class="katex-display">…</div>` wrapper
/// over the original `$$…$$` delimiters.
/// `<div class="katex-display">` (27 bytes) + `</div>` (6 bytes) − `$$` + `$$` (4 bytes) = 29.
const KATEX_DISPLAY_OVERHEAD: usize = 29;

/// Extra capacity budgeted per `apply_katex_math` call.
/// Sized for up to 6 inline expressions and 2 display expressions without reallocation.
/// 32 × 6 + 29 × 2 = 250 bytes. Documents with more expressions will simply reallocate.
const KATEX_EXTRA_CAPACITY: usize = KATEX_INLINE_OVERHEAD * 6 + KATEX_DISPLAY_OVERHEAD * 2;

/// Strict allow-list regex for a `<div class="bookmark">` block.
/// Only `<div class="bookmark"> <a href="URL">TITLE</a> </div>` passes through;
/// any extra tags, extra attributes, or missing structure causes the block to be
/// HTML-escaped instead. Used by `sanitize_html` to validate complete bookmark
/// blocks before emitting them as raw HTML.
static SAFE_BOOKMARK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\A\s*<div class="bookmark">\s*<a href="[^"]+">[^<]*</a>\s*</div>\s*\z"#)
        .expect("Invalid safe bookmark regex")
});

/// Mapping from an Obsidian file path without extension to a published slug.
pub type FileMapping = HashMap<String, String>;

fn generate_article_href(slug: &str) -> String {
    format!("/articles/{slug}")
}

/// Convert markdown content into HTML and apply KaTeX markers.
pub fn convert_markdown_to_html(markdown_content: &str) -> Result<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(markdown_content, options);
    let mut html_output = String::with_capacity(markdown_content.len() * 2);
    html::push_html(&mut html_output, sanitize_html(parser).into_iter());

    let html_with_katex = process_katex_math(&html_output);

    Ok(html_with_katex)
}

/// Escapes raw HTML events (XSS protection) while letting strictly valid
/// `<div class="bookmark">` blocks pass through as `Event::Html` for downstream
/// enrichment.
///
/// Each potential bookmark block is accumulated in a buffer until the first
/// `</div>` is found. The complete buffer is then validated against
/// [`SAFE_BOOKMARK_RE`]. Only if it matches exactly the expected structure
/// (`<div class="bookmark"><a href="URL">TITLE</a></div>`, with optional
/// whitespace) is it emitted as raw HTML. Any deviation — extra tags, extra
/// attributes, nested HTML, or an unclosed block — is emitted as `Event::Text`
/// and therefore HTML-escaped by the renderer.
fn sanitize_html<'a>(parser: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut result: Vec<Event<'a>> = Vec::new();
    let mut in_bookmark = false;
    let mut bookmark_buffer = String::new();

    for event in parser {
        match event {
            Event::Html(html) | Event::InlineHtml(html) => {
                if !in_bookmark && !html.trim_start().starts_with(r#"<div class="bookmark">"#) {
                    // Not a bookmark opening – escape it.
                    result.push(Event::Text(html));
                } else {
                    // Starting a new bookmark block or continuing to accumulate one.
                    if !in_bookmark {
                        in_bookmark = true;
                    }
                    bookmark_buffer.push_str(&html);

                    // Check whether the accumulated buffer now contains the closing tag.
                    if let Some(close) = bookmark_buffer.find("</div>") {
                        in_bookmark = false;
                        let safe_end = close + "</div>".len();
                        let bookmark_part = bookmark_buffer[..safe_end].to_string();
                        let rest = bookmark_buffer[safe_end..].to_string();
                        bookmark_buffer.clear();

                        if SAFE_BOOKMARK_RE.is_match(&bookmark_part) {
                            result.push(Event::Html(bookmark_part.into()));
                        } else {
                            // Structure does not match the strict allow-list – escape it.
                            result.push(Event::Text(bookmark_part.into()));
                        }

                        if !rest.is_empty() {
                            // Trailing content after </div> is always escaped.
                            result.push(Event::Text(rest.into()));
                        }
                    }
                    // No closing tag yet – keep accumulating; emit nothing for now.
                }
            }
            other => {
                // A non-HTML event arrived while inside an open bookmark block.
                // Flush the partial buffer as escaped text and process the event normally.
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

    // An unclosed bookmark was never terminated by </div>.
    // Emit the entire accumulated buffer as escaped text (fail-safe).
    if !bookmark_buffer.is_empty() {
        result.push(Event::Text(bookmark_buffer.into()));
    }

    result
}

/// Replaces KaTeX delimiters in `html_content`, skipping `<code>` / `<pre>`
/// blocks. `<pre` covers its nested `<code>` automatically.
fn process_katex_math(html_content: &str) -> String {
    let mut result = String::with_capacity(html_content.len() + KATEX_EXTRA_CAPACITY);
    let mut remaining = html_content;

    loop {
        // Find the earlier of the next <code> or <pre> start.
        let code_start = [remaining.find("<pre"), remaining.find("<code")]
            .into_iter()
            .flatten()
            .min();

        match code_start {
            None => {
                result.push_str(&apply_katex_math(remaining));
                break;
            }
            Some(start) => {
                result.push_str(&apply_katex_math(&remaining[..start]));

                let close_tag = if remaining[start..].starts_with("<pre") {
                    "</pre>"
                } else {
                    "</code>"
                };

                match remaining[start..].find(close_tag) {
                    Some(close_offset) => {
                        let end = start + close_offset + close_tag.len();
                        result.push_str(&remaining[start..end]); // verbatim
                        remaining = &remaining[end..];
                    }
                    None => {
                        result.push_str(&remaining[start..]); // verbatim
                        break;
                    }
                }
            }
        }
    }

    result
}

/// Replaces `$$...$$` and `$...$` with KaTeX class wrappers.
/// Only call on fragments with no `<code>` / `<pre>`; use `process_katex_math` for full HTML.
fn apply_katex_math(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + KATEX_EXTRA_CAPACITY);
    result.push_str(text);

    // Block math first to avoid matching the inner `$` of `$$`.
    while let Some(start) = result.find("$$") {
        if let Some(end) = result[start + 2..].find("$$") {
            let math_content = result[start + 2..start + 2 + end].to_string();
            let replacement = format!(r#"<div class="katex-display">{math_content}</div>"#);
            result.replace_range(start..start + 2 + end + 2, &replacement);
        } else {
            break;
        }
    }

    // Inline math.
    let mut pos = 0;
    while let Some(start) = result[pos..].find('$') {
        let actual_start = pos + start;
        if let Some(end) = result[actual_start + 1..].find('$') {
            let actual_end = actual_start + 1 + end;
            let math_content = result[actual_start + 1..actual_end].to_string();
            let replacement = format!(r#"<span class="katex-inline">{math_content}</span>"#);
            result.replace_range(actual_start..actual_end + 1, &replacement);
            pos = actual_start + replacement.len();
        } else {
            break;
        }
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

            let href = if let Some(slug) = file_mapping.get(link_target) {
                generate_article_href(slug)
            } else {
                let mut found = false;
                let mut result_href = format!("/{link_target}");

                for (key, slug) in file_mapping {
                    if key.ends_with(&format!("/{link_target}")) || key == link_target {
                        result_href = generate_article_href(slug);
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
        file_mapping.insert("notes/another-note".to_string(), "abc123def".to_string());
        file_mapping.insert("docs/filename".to_string(), "xyz789abc".to_string());
        file_mapping.insert("Another Note".to_string(), "abc123def".to_string());
        file_mapping.insert("filename".to_string(), "xyz789abc".to_string());

        let result =
            convert_obsidian_links("Check out [[Another Note]] for more info.", &file_mapping);
        assert_eq!(
            result,
            "Check out [Another Note](/articles/abc123def) for more info."
        );

        let result =
            convert_obsidian_links("See [[filename|Custom Display Text]] here.", &file_mapping);
        assert_eq!(
            result,
            "See [Custom Display Text](/articles/xyz789abc) here."
        );

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
        file_mapping.insert("Another Article".to_string(), "def456".to_string());
        file_mapping.insert("Reference Note".to_string(), "ghi789".to_string());

        let with_html_links = convert_obsidian_links(markdown_with_obsidian_links, &file_mapping);
        let html = convert_markdown_to_html(&with_html_links).unwrap();

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/articles/def456\">link</a>"));
        assert!(html.contains("<a href=\"/articles/ghi789\">Reference Note</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }

    #[rstest]
    #[case::inline_math(
        "Here is some inline math: $x^2 + y^2 = z^2$ and more text.",
        "Here is some inline math: <span class=\"katex-inline\">x^2 + y^2 = z^2</span> and more text."
    )]
    #[case::display_math(
        "Here is display math:\n$$\\int_0^1 x^2 dx = \\frac{1}{3}$$\nEnd of math.",
        "Here is display math:\n<div class=\"katex-display\">\\int_0^1 x^2 dx = \\frac{1}{3}</div>\nEnd of math."
    )]
    #[case::mixed_math(
        "Inline $a+b$ and display $$c+d$$ math.",
        "Inline <span class=\"katex-inline\">a+b</span> and display <div class=\"katex-display\">c+d</div> math."
    )]
    fn test_katex_math_processing(#[case] input: &str, #[case] expected: &str) {
        let result = super::process_katex_math(input);
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_markdown_link_escaping() {
        let mut file_mapping = FileMapping::new();
        file_mapping.insert("File with <script>".to_string(), "abc123".to_string());

        let result = convert_obsidian_links("[[File with <script>|Display & test]]", &file_mapping);
        assert_eq!(result, "[Display & test](/articles/abc123)");

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

    /// Verify `process_katex_math` directly on raw HTML fragments that mix
    /// math-bearing text with code elements.
    #[rstest]
    #[case::math_before_code(
        "Inline <span>$a$</span> then <code>$b$</code>",
        "Inline <span><span class=\"katex-inline\">a</span></span> then <code>$b$</code>"
    )]
    #[case::math_after_code(
        "<code>$b$</code> then $a$",
        "<code>$b$</code> then <span class=\"katex-inline\">a</span>"
    )]
    #[case::pre_block_skipped(
        "$x$ <pre><code>$y$</code></pre> $z$",
        "<span class=\"katex-inline\">x</span> <pre><code>$y$</code></pre> <span class=\"katex-inline\">z</span>"
    )]
    fn test_process_katex_math_skips_code_elements(#[case] input: &str, #[case] expected: &str) {
        let result = super::process_katex_math(input);
        assert_eq!(result, expected);
    }
}
