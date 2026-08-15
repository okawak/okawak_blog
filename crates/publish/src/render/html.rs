use super::sanitize;
use crate::links::{self, Index};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, html};
use std::{borrow::Cow, ops::Range};

/// Converts Markdown to sanitized HTML.
pub(crate) fn convert_markdown_to_html(markdown_content: &str, link_index: &Index) -> String {
    let markdown_content = escape_wikilink_pipes_for_table_parser(markdown_content);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_MATH);
    options.insert(Options::ENABLE_WIKILINKS);

    let parser = Parser::new_ext(&markdown_content, options);
    let parser = links::resolve_wikilinks(parser, link_index);
    let mut html_output = String::with_capacity(markdown_content.len() * 2);
    html::push_html(&mut html_output, sanitize::events(parser).into_iter());

    html_output
}

fn escape_wikilink_pipes_for_table_parser(markdown: &str) -> Cow<'_, str> {
    let protected_ranges = wikilink_ranges(markdown);

    if protected_ranges.is_empty() {
        return Cow::Borrowed(markdown);
    }

    // A one-byte replacement keeps parser offsets aligned with the original Markdown.
    let table_probe = replace_pipes_in_ranges(markdown, &protected_ranges, "/");
    let table_ranges = Parser::new_ext(
        &table_probe,
        Options::ENABLE_TABLES | Options::ENABLE_WIKILINKS,
    )
    .into_offset_iter()
    .filter_map(|(event, range)| match event {
        Event::Start(Tag::Table(_)) => Some(range),
        _ => None,
    })
    .collect::<Vec<_>>();

    let table_ranges_to_escape = protected_ranges
        .into_iter()
        .filter(|range| {
            table_ranges
                .iter()
                .any(|table_range| table_range.start <= range.start && range.end <= table_range.end)
        })
        .collect::<Vec<_>>();

    if table_ranges_to_escape.is_empty() {
        return Cow::Borrowed(markdown);
    }

    Cow::Owned(replace_pipes_in_ranges(
        markdown,
        &table_ranges_to_escape,
        r"\|",
    ))
}

fn wikilink_ranges(markdown: &str) -> Vec<Range<usize>> {
    let mut ranges = Parser::new_ext(markdown, Options::ENABLE_WIKILINKS)
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(
                Tag::Link {
                    link_type: LinkType::WikiLink { has_pothole: true },
                    ..
                }
                | Tag::Image {
                    link_type: LinkType::WikiLink { has_pothole: true },
                    ..
                },
            ) => Some(range),
            _ => None,
        })
        .collect::<Vec<_>>();

    ranges.sort_unstable_by_key(|range| range.start);
    let mut merged_ranges: Vec<Range<usize>> = Vec::with_capacity(ranges.len());

    for range in ranges {
        if let Some(previous) = merged_ranges.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
            continue;
        }
        merged_ranges.push(range);
    }

    merged_ranges
}

fn replace_pipes_in_ranges(markdown: &str, ranges: &[Range<usize>], replacement: &str) -> String {
    let mut replaced = String::with_capacity(markdown.len());
    let mut previous_end = 0;

    for range in ranges {
        replaced.push_str(&markdown[previous_end..range.start]);
        replaced.push_str(&markdown[range.clone()].replace('|', replacement));
        previous_end = range.end;
    }

    replaced.push_str(&markdown[previous_end..]);
    replaced
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::*;

    fn convert_markdown_to_html(markdown: &str) -> String {
        super::convert_markdown_to_html(markdown, &Index::default())
    }

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
        let result = convert_markdown_to_html(markdown);
        assert_eq!(result, expected_html);
    }

    #[rstest]
    #[case::inline_math(
        "Here is some inline math: $x^2 + y^2 = z^2$ and more text.",
        "<p>Here is some inline math: <span class=\"math math-inline\">x^2 + y^2 = z^2</span> and more text.</p>\n"
    )]
    #[case::display_math(
        "Here is display math:\n$$\\int_0^1 x^2 dx = \\frac{1}{3}$$\nEnd of math.",
        "<p>Here is display math:\n<span class=\"math math-display\">\\int_0^1 x^2 dx = \\frac{1}{3}</span>\nEnd of math.</p>\n"
    )]
    #[case::mixed_math(
        "Inline $a+b$ and display $$c+d$$ math.",
        "<p>Inline <span class=\"math math-inline\">a+b</span> and display <span class=\"math math-display\">c+d</span> math.</p>\n"
    )]
    #[case::pipe_in_inline_math(
        r"Inline $a\|b$ math.",
        "<p>Inline <span class=\"math math-inline\">a\\|b</span> math.</p>\n"
    )]
    #[case::unescaped_pipe_outside_table(
        "Inline $a|b$ math.",
        "<p>Inline <span class=\"math math-inline\">a|b</span> math.</p>\n"
    )]
    fn test_math_processing(#[case] input: &str, #[case] expected: &str) {
        let result = convert_markdown_to_html(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_math_markup_uses_semantic_classes() {
        let result = convert_markdown_to_html("Inline $a+b$ and display $$c+d$$.");

        assert!(result.contains(r#"class="math math-inline">a+b</span>"#));
        assert!(result.contains(r#"class="math math-display">c+d</span>"#));
    }

    #[test]
    fn test_escaped_math_pipe_inside_table_cell_is_preserved() {
        let markdown = "| Type | Expression |\n| --- | --- |\n| math | $a\\|b$ |";

        let result = convert_markdown_to_html(markdown);

        assert!(
            result.contains(r#"<td><span class="math math-inline">a|b</span></td>"#),
            "unexpected html:\n{result}"
        );
    }

    #[test]
    fn test_escaped_math_pipe_inside_table_header_is_preserved() {
        let markdown = "| $a\\|b$ | Value |\n| --- | --- |\n| math | result |";

        let result = convert_markdown_to_html(markdown);

        assert!(
            result.contains(r#"<th><span class="math math-inline">a|b</span></th>"#),
            "unexpected html:\n{result}"
        );
        assert!(result.starts_with("<table>"), "unexpected html:\n{result}");
    }

    #[rstest]
    #[case::inline("$unclosed", "<p>$unclosed</p>\n")]
    #[case::display("$$unclosed", "<p>$$unclosed</p>\n")]
    fn test_unclosed_math_delimiter_remains_text(#[case] input: &str, #[case] expected: &str) {
        let result = convert_markdown_to_html(input);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_math_content_is_html_escaped() {
        let result = convert_markdown_to_html(r#"$x < y & "quoted"$"#);

        assert_eq!(
            result,
            "<p><span class=\"math math-inline\">x &lt; y &amp; &quot;quoted&quot;</span></p>\n"
        );
    }

    #[test]
    fn test_commonmark_bold_text_around_math_is_preserved() {
        let markdown = "この時に使う考え方が、 **「サンプリング」** と **「モデル化」** です。\n\nその身長を **$x = (x_1, x_2)$** と書きます。";

        let result = convert_markdown_to_html(markdown);

        assert!(
            result.contains("<strong>「サンプリング」</strong> と <strong>「モデル化」</strong>"),
            "unexpected html:\n{result}"
        );
        assert!(
            result.contains(
                "<strong><span class=\"math math-inline\">x = (x_1, x_2)</span></strong>"
            )
        );
        assert!(!result.contains("**"));
    }

    #[rstest]
    #[case::text(r#"これは \*\*literal\*\* です。"#)]
    #[case::math(r#"これは \*\*$x$\*\* です。"#)]
    fn test_escaped_strong_markers_remain_literal(#[case] markdown: &str) {
        let result = convert_markdown_to_html(markdown);

        assert!(result.contains("**"));
        assert!(!result.contains("<strong>"));
    }

    #[test]
    fn test_math_parser_skips_inline_code_with_longer_backtick_delimiter() {
        let markdown = "``code with `$x$` inside`` and real math $y$";

        let result = convert_markdown_to_html(markdown);

        assert!(result.contains("<code>code with `$x$` inside</code>"));
        assert!(result.contains(r#"<span class="math math-inline">y</span>"#));
        assert!(!result.contains(r#"<span class="math math-inline">x</span>"#));
    }

    #[test]
    fn test_math_parser_skips_backticks_inside_fenced_code() {
        let markdown = "```text\nliteral ``` and $x$\n```\noutside $y$";

        let result = convert_markdown_to_html(markdown);

        assert!(
            result
                .contains("<pre><code class=\"language-text\">literal ``` and $x$\n</code></pre>")
        );
        assert!(result.contains(r#"<span class="math math-inline">y</span>"#));
        assert!(!result.contains(r#"<span class="math math-inline">x</span>"#));
    }

    #[test]
    fn test_math_parser_skips_link_destinations() {
        let markdown = "[example](https://example.com/search?q=$x$)";

        let result = convert_markdown_to_html(markdown);

        assert!(result.contains(r#"href="https://example.com/search?q=$x$""#));
        assert!(!result.contains("math math-inline"));
    }

    #[test]
    fn test_math_parser_skips_raw_html_attributes() {
        let markdown = r#"<img src="https://example.com/$x$.png" alt="img">"#;

        let result = convert_markdown_to_html(markdown);

        assert!(result.contains("&lt;img"));
        assert!(result.contains("$x$.png"));
        assert!(!result.contains("math math-inline"));
    }

    #[test]
    fn test_math_parser_handles_comparison_text() {
        let markdown = "x < y and $z$ > 0";

        let result = convert_markdown_to_html(markdown);

        assert!(result.contains(r#"<span class="math math-inline">z</span>"#));
    }

    #[test]
    fn test_math_parser_respects_escaped_dollar_signs() {
        let markdown = r"\$100 and \$x\$";

        let result = convert_markdown_to_html(markdown);

        assert!(result.contains("$100"));
        assert!(result.contains("$x$"));
        assert!(!result.contains("math math-inline"));
    }

    // -----------------------------------------------------------------
    // Math + code block tests
    // -----------------------------------------------------------------

    /// Inline code (backtick) containing `$...$` must NOT be converted to a
    /// math span – the dollar signs are part of the code, not math.
    #[rstest]
    fn test_math_not_processed_in_inline_code() {
        let markdown = "See `$x^2$` for the formula.";

        let result = convert_markdown_to_html(markdown);

        assert!(
            result.contains("<code>$x^2$</code>"),
            "inline code content should not be parsed as math; got:\n{result}"
        );
        assert!(
            !result.contains("math math-inline"),
            "no math span expected inside inline code; got:\n{result}"
        );
    }

    /// Fenced code blocks containing `$...$` or `$$...$$` must NOT produce
    /// any math wrappers.
    #[rstest]
    fn test_math_not_processed_in_fenced_code_block() {
        let markdown = "```\n$x^2$ and $$block$$ formula\n```";

        let result = convert_markdown_to_html(markdown);

        assert!(
            !result.contains("math math-display"),
            "fenced code block should not produce display math; got:\n{result}"
        );
        assert!(
            !result.contains("math math-inline"),
            "fenced code block should not produce inline math; got:\n{result}"
        );
        assert!(
            result.contains("$x^2$"),
            "dollar signs should remain verbatim in code block; got:\n{result}"
        );
    }

    /// Math markers that appear OUTSIDE code elements must still be converted
    /// even when code elements are present in the same document.
    #[rstest]
    fn test_math_processed_in_text_adjacent_to_code() {
        let markdown = "The formula $a+b$ is useful. Code: `$not_math$`.";

        let result = convert_markdown_to_html(markdown);

        assert!(
            result.contains(r#"<span class="math math-inline">a+b</span>"#),
            "math outside code should be converted to math span; got:\n{result}"
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
    fn test_math_parser_skips_code(#[case] markdown: &str) {
        let result = convert_markdown_to_html(markdown);

        assert!(result.contains("math math-inline"));
        assert!(result.contains("$not_math$"));
        assert!(!result.contains("<span class=\"math math-inline\">not_math</span>"));
    }
}
