use crate::error::Result;
use pulldown_cmark::{Options, Parser, html};
use regex::Regex;
use std::sync::LazyLock;

/// Markdownコンテンツを観察可能なHTMLに変換する
pub fn convert_markdown_to_html(markdown_content: &str) -> Result<String> {
    // pulldown-cmarkのオプションを設定（テーブルサポートを有効化）
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    // Markdownパーサーを作成
    let parser = Parser::new_ext(markdown_content, options);

    // HTMLに変換
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(html_output)
}

/// ObsidianのリンクをHTMLリンクに変換する
/// [[filename]] → <a href="/filename">filename</a>
/// [[filename|display text]] → <a href="/filename">display text</a>
pub fn convert_obsidian_links(content: &str) -> String {
    // LazyLockを使用してRegexを一度だけコンパイル
    static OBSIDIAN_LINK_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("Invalid regex pattern"));

    OBSIDIAN_LINK_REGEX
        .replace_all(content, |caps: &regex::Captures| {
            let link_content = &caps[1];

            // パイプ記号で分割してリンク先と表示テキストを分離
            if let Some(pipe_pos) = link_content.find('|') {
                let (link, display_text) = link_content.split_at(pipe_pos);
                let display_text = &display_text[1..]; // パイプ記号をスキップ
                format!("<a href=\"/{}\">{}</a>", link, display_text)
            } else {
                // 表示テキストが指定されていない場合はリンク名を使用
                format!("<a href=\"/{}\">{}</a>", link_content, link_content)
            }
        })
        .to_string()
}

/// フロントマターとHTMLボディを結合してHTMLファイルを生成する
pub fn generate_html_file(frontmatter_yaml: &str, html_body: &str) -> String {
    format!("---\n{}\n---\n{}", frontmatter_yaml, html_body)
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
    #[case::simple_link(
        "Check out [[Another Note]] for more info.",
        "Check out <a href=\"/Another Note\">Another Note</a> for more info."
    )]
    #[case::link_with_display_text(
        "See [[filename|Custom Display Text]] here.",
        "See <a href=\"/filename\">Custom Display Text</a> here."
    )]
    #[case::multiple_links(
        "Links: [[First Note]] and [[Second Note|Second]] and [[Third]].",
        "Links: <a href=\"/First Note\">First Note</a> and <a href=\"/Second Note\">Second</a> and <a href=\"/Third\">Third</a>."
    )]
    #[case::no_links(
        "This is normal text with no special links.",
        "This is normal text with no special links."
    )]
    #[case::japanese_links(
        "日本語ノートは[[日本語ノート]]です。",
        "日本語ノートは<a href=\"/日本語ノート\">日本語ノート</a>です。"
    )]
    fn test_obsidian_links_conversion(#[case] content: &str, #[case] expected: &str) {
        let result = convert_obsidian_links(content);
        assert_eq!(result, expected);
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

        // まずObsidianリンクを変換
        let with_html_links = convert_obsidian_links(markdown_with_obsidian_links);

        // 次にMarkdownをHTMLに変換
        let html = convert_markdown_to_html(&with_html_links).unwrap();

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/Another Article\">link</a>"));
        assert!(html.contains("<a href=\"/Reference Note\">Reference Note</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }
}
