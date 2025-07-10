use crate::error::Result;
use pulldown_cmark::{Options, Parser, html};
use regex::Regex;
use std::sync::LazyLock;

/// Markdownコンテンツを観察可能なHTMLに変換する
pub fn convert_markdown_to_html(markdown_content: &str) -> Result<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES); // 表を有効
    options.insert(Options::ENABLE_FOOTNOTES); // 脚注を有効
    options.insert(Options::ENABLE_STRIKETHROUGH); // 打ち消し線を有効
    options.insert(Options::ENABLE_TASKLISTS); // タスクリストを有効
    options.insert(Options::ENABLE_SMART_PUNCTUATION); // スマート引用符を有効

    let parser = Parser::new_ext(markdown_content, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // KaTeX数式サポートを追加
    let html_with_katex = process_katex_math(&html_output);

    // リッチブックマークを処理
    let html_with_bookmarks = process_rich_bookmarks(&html_with_katex);

    Ok(html_with_bookmarks)
}

/// KaTeX数式処理：$...$（インライン）と$$...$$（ブロック）を検出してKaTeXクラスを追加
fn process_katex_math(html_content: &str) -> String {
    // 文字列を段階的に処理して、二重ドルマークを先に処理
    let mut result = html_content.to_string();
    
    // ブロック数式を処理（$$...$$）
    while let Some(start) = result.find("$$") {
        if let Some(end) = result[start + 2..].find("$$") {
            let math_content = &result[start + 2..start + 2 + end];
            let replacement = format!(r#"<div class="katex-display">{}</div>"#, math_content);
            result.replace_range(start..start + 2 + end + 2, &replacement);
        } else {
            break;
        }
    }
    
    // インライン数式を処理（$...$）
    let mut pos = 0;
    while let Some(start) = result[pos..].find('$') {
        let actual_start = pos + start;
        if let Some(end) = result[actual_start + 1..].find('$') {
            let actual_end = actual_start + 1 + end;
            let math_content = &result[actual_start + 1..actual_end];
            let replacement = format!(r#"<span class="katex-inline">{}</span>"#, math_content);
            result.replace_range(actual_start..actual_end + 1, &replacement);
            pos = actual_start + replacement.len();
        } else {
            break;
        }
    }
    
    result
}

/// リッチブックマーク処理：シンプルなブックマークをリッチブックマークに変換
fn process_rich_bookmarks(html_content: &str) -> String {
    static SIMPLE_BOOKMARK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"<div class="bookmark">\s*<a href="([^"]+)">([^<]+)</a>\s*</div>"#)
            .expect("Invalid bookmark regex")
    });

    SIMPLE_BOOKMARK_REGEX.replace_all(html_content, |caps: &regex::Captures| {
        let url = &caps[1];
        let title = &caps[2];
        
        // URLからドメインを抽出
        let domain = extract_domain(url);
        
        format!(
            r#"<div class="bookmark">
  <a class="bookmark-link" href="{}" target="_blank" rel="noopener">
    <div class="bookmark-content">
      <div class="bookmark-title">{}</div>
      <div class="bookmark-description">外部リンク</div>
      <div class="bookmark-domain">{}</div>
    </div>
    <div class="bookmark-thumb"></div>
  </a>
</div>"#,
            url, title, domain
        )
    }).to_string()
}

/// URLからドメインを抽出する補助関数
fn extract_domain(url: &str) -> &str {
    if let Some(start) = url.find("://") {
        let after_protocol = &url[start + 3..];
        if let Some(end) = after_protocol.find('/') {
            &after_protocol[..end]
        } else {
            after_protocol
        }
    } else {
        url
    }
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
                format!("<a href=\"/{}\">{}</a>", 
                    html_escape(link), 
                    html_escape(display_text)
                )
            } else {
                // 表示テキストが指定されていない場合はリンク名を使用
                format!("<a href=\"/{}\">{}</a>", 
                    html_escape(link_content), 
                    html_escape(link_content)
                )
            }
        })
        .to_string()
}

/// HTMLエスケープ処理
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
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
    #[case::simple_bookmark(
        r#"<div class="bookmark">
  <a href="https://example.com">Example Site</a>
</div>"#,
        r#"<div class="bookmark">
  <a class="bookmark-link" href="https://example.com" target="_blank" rel="noopener">
    <div class="bookmark-content">
      <div class="bookmark-title">Example Site</div>
      <div class="bookmark-description">外部リンク</div>
      <div class="bookmark-domain">example.com</div>
    </div>
    <div class="bookmark-thumb"></div>
  </a>
</div>"#
    )]
    fn test_rich_bookmark_processing(#[case] input: &str, #[case] expected: &str) {
        let result = super::process_rich_bookmarks(input);
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::html_entities(
        "[[File with <script>|Display & test]]",
        "<a href=\"/File with &lt;script&gt;\">Display &amp; test</a>"
    )]
    #[case::quotes(
        "[[File \"quoted\"|Text with 'quotes']]",
        "<a href=\"/File &quot;quoted&quot;\">Text with &#x27;quotes&#x27;</a>"
    )]
    fn test_html_escape_in_links(#[case] input: &str, #[case] expected: &str) {
        let result = convert_obsidian_links(input);
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case::https_url("https://example.com/path", "example.com")]
    #[case::http_url("http://test.org", "test.org")]
    #[case::no_protocol("example.com", "example.com")]
    #[case::with_path("https://docs.rust-lang.org/book/", "docs.rust-lang.org")]
    fn test_extract_domain(#[case] url: &str, #[case] expected: &str) {
        let result = super::extract_domain(url);
        assert_eq!(result, expected);
    }
}
