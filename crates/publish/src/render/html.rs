use super::katex::ProtectedMarkdown;
use crate::error::Result;
use pulldown_cmark::{Event, Options, Parser, html};
use regex::Regex;
use std::sync::LazyLock;

/// Allow-list regex for bookmark blocks; anything beyond `<a href="URL">TITLE</a>` is escaped.
static SAFE_BOOKMARK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\A\s*<div class="bookmark">\s*<a href="[^"]+">[^<]*</a>\s*</div>\s*\z"#)
        .expect("Invalid safe bookmark regex")
});
static HREF_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]*)""#).expect("Invalid href regex"));

/// Converts Markdown to sanitized HTML.
pub(crate) fn convert_markdown_to_html(markdown_content: &str) -> Result<String> {
    let protected_markdown = ProtectedMarkdown::new(markdown_content);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(protected_markdown.as_str(), options);
    let mut html_output = String::with_capacity(protected_markdown.as_str().len() * 2);
    html::push_html(&mut html_output, sanitize_html(parser).into_iter());
    let html_output = sanitize_anchor_hrefs(&html_output);

    Ok(protected_markdown.restore(&html_output))
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
    #[case::inline_math(
        "Here is some inline math: $x^2 + y^2 = z^2$ and more text.",
        "<p>Here is some inline math: <span class=\"okawak-katex-inline\">x^2 + y^2 = z^2</span> and more text.</p>\n"
    )]
    #[case::display_math(
        "Here is display math:\n$$\\int_0^1 x^2 dx = \\frac{1}{3}$$\nEnd of math.",
        "<p>Here is display math:\n<span class=\"okawak-katex-display\">\\int_0^1 x^2 dx = \\frac{1}{3}</span>\nEnd of math.</p>\n"
    )]
    #[case::mixed_math(
        "Inline $a+b$ and display $$c+d$$ math.",
        "<p>Inline <span class=\"okawak-katex-inline\">a+b</span> and display <span class=\"okawak-katex-display\">c+d</span> math.</p>\n"
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
        assert!(result.contains(
            "<strong><span class=\"okawak-katex-inline\">x = (x_1, x_2)</span></strong>"
        ));
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

        assert!(result.contains(r#"<span class="okawak-katex-inline">x + y</span>"#));
        assert!(result.contains(r#"<span class="okawak-katex-display">a + b</span>"#));
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
        assert!(result.contains(r#"<span class="okawak-katex-inline">y</span>"#));
        assert!(!result.contains(r#"<span class="okawak-katex-inline">x</span>"#));
    }

    #[test]
    fn test_katex_placeholders_skip_backticks_inside_fenced_code() {
        let markdown = "```text\nliteral ``` and $x$\n```\noutside $y$";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(
            result
                .contains("<pre><code class=\"language-text\">literal ``` and $x$\n</code></pre>")
        );
        assert!(result.contains(r#"<span class="okawak-katex-inline">y</span>"#));
        assert!(!result.contains(r#"<span class="okawak-katex-inline">x</span>"#));
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
        assert!(!result.contains("okawak-katex-inline"));
    }

    #[test]
    fn test_katex_placeholders_do_not_rewrite_raw_html_attributes() {
        let markdown = r#"<img src="https://example.com/$x$.png" alt="img">"#;

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("&lt;img"));
        assert!(result.contains("$x$.png"));
        assert!(!result.contains("okawak-katex-inline"));
    }

    #[test]
    fn test_katex_placeholders_do_not_treat_comparison_as_html_tag() {
        let markdown = "x < y and $z$ > 0";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains(r#"<span class="okawak-katex-inline">z</span>"#));
    }

    #[test]
    fn test_katex_placeholders_respect_escaped_dollar_signs() {
        let markdown = r"\$100 and \$x\$";

        let result = convert_markdown_to_html(markdown).unwrap();

        assert!(result.contains("$100"));
        assert!(result.contains("$x$"));
        assert!(!result.contains("okawak-katex-inline"));
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
            !result.contains("okawak-katex-inline"),
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
            !result.contains("okawak-katex-display"),
            "fenced code block should not produce KaTeX display; got:\n{result}"
        );
        assert!(
            !result.contains("okawak-katex-inline"),
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
            result.contains(r#"<span class="okawak-katex-inline">a+b</span>"#),
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

        assert!(result.contains("okawak-katex-inline"));
        assert!(result.contains("$not_math$"));
        assert!(!result.contains("<span class=\"okawak-katex-inline\">not_math</span>"));
    }
}
