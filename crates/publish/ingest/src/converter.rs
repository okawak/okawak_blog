use crate::error::Result;
use pulldown_cmark::{Event, Options, Parser, html};
use regex::Regex;
use std::{collections::HashMap, sync::LazyLock};

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

    let parser = Parser::new_ext(markdown_content, options).map(disable_raw_html);
    let mut html_output = String::with_capacity(markdown_content.len() * 2);
    html::push_html(&mut html_output, parser);

    let html_with_katex = process_katex_math(&html_output);

    Ok(html_with_katex)
}

fn disable_raw_html(event: Event<'_>) -> Event<'_> {
    match event {
        Event::Html(html) | Event::InlineHtml(html) => Event::Text(html),
        other => other,
    }
}

fn process_katex_math(html_content: &str) -> String {
    let mut result = String::with_capacity(html_content.len() + 200);
    result.push_str(html_content);

    while let Some(start) = result.find("$$") {
        if let Some(end) = result[start + 2..].find("$$") {
            let math_content = &result[start + 2..start + 2 + end];
            let replacement = format!(r#"<div class="katex-display">{math_content}</div>"#);
            result.replace_range(start..start + 2 + end + 2, &replacement);
        } else {
            break;
        }
    }

    let mut pos = 0;
    while let Some(start) = result[pos..].find('$') {
        let actual_start = pos + start;
        if let Some(end) = result[actual_start + 1..].find('$') {
            let actual_end = actual_start + 1 + end;
            let math_content = &result[actual_start + 1..actual_end];
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
}
