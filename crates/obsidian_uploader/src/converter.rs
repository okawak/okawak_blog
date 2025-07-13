use crate::error::Result;
use pulldown_cmark::{Options, Parser, html};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

// Re-export bookmark types and functions
pub use crate::bookmark::{BookmarkData, convert_simple_bookmarks_to_rich, generate_rich_bookmark, create_fallback_bookmark_data};

/// ファイル情報を保持する構造体（リンク解決用）
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Obsidianディレクトリからの相対パス（拡張子なし）
    pub relative_path: String,
    /// 生成されるslug
    pub slug: String,
    /// HTMLファイルへの最終的なパス
    pub html_path: String,
}

/// ファイル名（拡張子なし）からFileInfoへのマッピング
pub type FileMapping = HashMap<String, FileInfo>;

/// MarkdownコンテンツをHTMLに変換し、KaTeX数式処理を適用する
pub fn convert_markdown_to_html(markdown_content: &str) -> Result<String> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES); // 表要素の解析を有効化
    options.insert(Options::ENABLE_FOOTNOTES); // 脚注記法の解析を有効化
    options.insert(Options::ENABLE_STRIKETHROUGH); // 打ち消し線記法の解析を有効化
    options.insert(Options::ENABLE_TASKLISTS); // チェックボックス付きタスクリストの解析を有効化
    options.insert(Options::ENABLE_SMART_PUNCTUATION); // クォート記号の自動変換を有効化

    let parser = Parser::new_ext(markdown_content, options);
    let mut html_output = String::with_capacity(markdown_content.len() * 2);
    html::push_html(&mut html_output, parser);

    let html_with_katex = process_katex_math(&html_output);

    Ok(html_with_katex)
}

/// KaTeX数式処理：$...$（インライン）と$$...$$（ブロック）を検出してKaTeXクラスを追加
fn process_katex_math(html_content: &str) -> String {
    let mut result = String::with_capacity(html_content.len() + 200);
    result.push_str(html_content);

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

/// ObsidianのリンクをHTMLリンクに変換する（ファイルマッピングを使用してリンク解決）
/// [[filename]] → <a href="/actual/path/{slug}.html">filename</a>
/// [[filename|display text]] → <a href="/actual/path/{slug}.html">display text</a>
pub fn convert_obsidian_links(content: &str, file_mapping: &FileMapping) -> String {
    static OBSIDIAN_LINK_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("Invalid regex pattern"));

    OBSIDIAN_LINK_REGEX
        .replace_all(content, |caps: &regex::Captures| {
            let link_content = &caps[1];

            // パイプ記号で分割してリンク先と表示テキストを分離
            let (link_target, display_text) = if let Some(pipe_pos) = link_content.find('|') {
                let (link, display) = link_content.split_at(pipe_pos);
                (link.trim(), display[1..].trim()) // パイプ記号をスキップ
            } else {
                (link_content.trim(), link_content.trim())
            };

            // ファイルマッピングからリンク先を解決
            // まずファイル名で検索し、見つからない場合は相対パスとしても検索
            let href = if let Some(file_info) = file_mapping.get(link_target) {
                file_info.html_path.clone()
            } else {
                // 相対パス全体での検索も試行
                let mut found = false;
                let mut result_href = format!("/{}", link_target);

                for (key, file_info) in file_mapping {
                    if key.ends_with(&format!("/{}", link_target)) || key == link_target {
                        result_href = file_info.html_path.clone();
                        found = true;
                        break;
                    }
                }

                if !found {
                    eprintln!(
                        "Warning: Link target '{}' not found in file mapping",
                        link_target
                    );
                }

                result_href
            };

            format!(
                "<a href=\"{}\">{}</a>",
                html_escape(&href),
                html_escape(display_text)
            )
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
    use crate::bookmark::BookmarkData;
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
        // テスト用のファイルマッピングを作成（相対パス全体をキーとして使用）
        let mut file_mapping = FileMapping::new();
        file_mapping.insert(
            "notes/another-note".to_string(),
            FileInfo {
                relative_path: "notes/another-note".to_string(),
                slug: "abc123def".to_string(),
                html_path: "/notes/another-note.html".to_string(),
            },
        );
        file_mapping.insert(
            "docs/filename".to_string(),
            FileInfo {
                relative_path: "docs/filename".to_string(),
                slug: "xyz789abc".to_string(),
                html_path: "/docs/filename.html".to_string(),
            },
        );
        // 後方互換性のためにファイル名のみのキーも追加
        file_mapping.insert(
            "Another Note".to_string(),
            FileInfo {
                relative_path: "notes/another-note".to_string(),
                slug: "abc123def".to_string(),
                html_path: "/notes/another-note.html".to_string(),
            },
        );
        file_mapping.insert(
            "filename".to_string(),
            FileInfo {
                relative_path: "docs/filename".to_string(),
                slug: "xyz789abc".to_string(),
                html_path: "/docs/filename.html".to_string(),
            },
        );

        // 基本的なリンク変換
        let result =
            convert_obsidian_links("Check out [[Another Note]] for more info.", &file_mapping);
        assert_eq!(
            result,
            "Check out <a href=\"/notes/another-note.html\">Another Note</a> for more info."
        );

        // 表示テキスト付きリンク
        let result =
            convert_obsidian_links("See [[filename|Custom Display Text]] here.", &file_mapping);
        assert_eq!(
            result,
            "See <a href=\"/docs/filename.html\">Custom Display Text</a> here."
        );

        // 存在しないリンク（警告が出力されるが、フォールバック動作をテスト）
        let result = convert_obsidian_links("Link to [[nonexistent]] file.", &file_mapping);
        assert_eq!(
            result,
            "Link to <a href=\"/nonexistent\">nonexistent</a> file."
        );

        // リンクがないテキスト
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

        // テスト用のファイルマッピングを作成
        let mut file_mapping = FileMapping::new();
        file_mapping.insert(
            "Another Article".to_string(),
            FileInfo {
                relative_path: "articles/another".to_string(),
                slug: "def456".to_string(),
                html_path: "/articles/another.html".to_string(),
            },
        );
        file_mapping.insert(
            "Reference Note".to_string(),
            FileInfo {
                relative_path: "notes/reference".to_string(),
                slug: "ghi789".to_string(),
                html_path: "/notes/reference.html".to_string(),
            },
        );

        // まずObsidianリンクを変換
        let with_html_links = convert_obsidian_links(markdown_with_obsidian_links, &file_mapping);

        // 次にMarkdownをHTMLに変換
        let html = convert_markdown_to_html(&with_html_links).unwrap();

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/articles/another.html\">link</a>"));
        assert!(html.contains("<a href=\"/notes/reference.html\">Reference Note</a>"));
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
    fn test_html_escape_in_links() {
        // テスト用のファイルマッピングを作成
        let mut file_mapping = FileMapping::new();
        file_mapping.insert(
            "File with <script>".to_string(),
            FileInfo {
                relative_path: "files/script-file".to_string(),
                slug: "abc123".to_string(),
                html_path: "/files/script-file.html".to_string(),
            },
        );

        // HTMLエンティティのエスケープテスト（リンク先が存在する場合）
        let result = convert_obsidian_links("[[File with <script>|Display & test]]", &file_mapping);
        assert_eq!(
            result,
            "<a href=\"/files/script-file.html\">Display &amp; test</a>"
        );

        // 存在しないファイルでのHTMLエスケープテスト
        let result =
            convert_obsidian_links("[[File \"quoted\"|Text with 'quotes']]", &file_mapping);
        assert_eq!(
            result,
            "<a href=\"/File &quot;quoted&quot;\">Text with &#x27;quotes&#x27;</a>"
        );
    }

    // Bookmark tests moved to bookmark.rs module
    #[rstest]
    fn test_bookmark_data_creation() {
        let bookmark_data = BookmarkData {
            url: "https://example.com".to_string(),
            title: "Example Title".to_string(),
            description: Some("Example description".to_string()),
            image_url: Some("https://example.com/image.jpg".to_string()),
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
        };

        assert_eq!(bookmark_data.url, "https://example.com");
        assert_eq!(bookmark_data.title, "Example Title");
        assert_eq!(
            bookmark_data.description,
            Some("Example description".to_string())
        );
        assert_eq!(
            bookmark_data.image_url,
            Some("https://example.com/image.jpg".to_string())
        );
        assert_eq!(
            bookmark_data.favicon_url,
            Some("https://example.com/favicon.ico".to_string())
        );
    }

    #[rstest]
    #[case::full_metadata(
        &BookmarkData {
            url: "https://example.com".to_string(),
            title: "Example Title".to_string(),
            description: Some("This is an example description".to_string()),
            image_url: Some("https://example.com/image.jpg".to_string()),
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
        },
        indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com" target="_blank" rel="noopener noreferrer" class="bookmark-link">
                <div class="bookmark-container">
                  <div class="bookmark-info">
                    <div class="bookmark-title">Example Title</div>
                    <div class="bookmark-description">This is an example description</div>
                    <div class="bookmark-link-info">
                      <img class="bookmark-favicon" src="https://example.com/favicon.ico" alt="favicon">
                      <span class="bookmark-domain">example.com</span>
                    </div>
                  </div>
                  <div class="bookmark-image">
                    <img src="https://example.com/image.jpg" alt="Example Title" loading="lazy">
                  </div>
                </div>
              </a>
            </div>"#}.trim_end()
    )]
    #[case::minimal_metadata(
        &BookmarkData {
            url: "https://github.com".to_string(),
            title: "GitHub".to_string(),
            description: None,
            image_url: None,
            favicon_url: None,
        },
        indoc! {r#"
            <div class="bookmark">
              <a href="https://github.com" target="_blank" rel="noopener noreferrer" class="bookmark-link">
                <div class="bookmark-container">
                  <div class="bookmark-info">
                    <div class="bookmark-title">GitHub</div>
                    <div class="bookmark-link-info">
                      <span class="bookmark-domain">github.com</span>
                    </div>
                  </div>
                </div>
              </a>
            </div>"#}.trim_end()
    )]
    fn test_generate_rich_bookmark(
        #[case] bookmark_data: &BookmarkData,
        #[case] expected_html: &str,
    ) {
        let result = generate_rich_bookmark(bookmark_data);
        assert_eq!(result, expected_html);
    }

    #[rstest]
    #[case::single_bookmark(
        indoc! {r#"
            <p>Check out this site:</p>
            <div class="bookmark">
              <a href="https://example.com">Example Site</a>
            </div>
            <p>End of content.</p>
        "#},
        indoc! {r#"
            <p>Check out this site:</p>
            <div class="bookmark">
              <a href="https://example.com" target="_blank" rel="noopener noreferrer" class="bookmark-link">
                <div class="bookmark-container">
                  <div class="bookmark-info">
                    <div class="bookmark-title">Example Site</div>
                    <div class="bookmark-link-info">
                      <span class="bookmark-domain">example.com</span>
                    </div>
                  </div>
                </div>
              </a>
            </div>
            <p>End of content.</p>
        "#}
    )]
    #[case::multiple_bookmarks(
        indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com">Example</a>
            </div>
            <p>Text between bookmarks</p>
            <div class="bookmark">
              <a href="https://github.com">GitHub</a>
            </div>
        "#},
        indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com" target="_blank" rel="noopener noreferrer" class="bookmark-link">
                <div class="bookmark-container">
                  <div class="bookmark-info">
                    <div class="bookmark-title">Example</div>
                    <div class="bookmark-link-info">
                      <span class="bookmark-domain">example.com</span>
                    </div>
                  </div>
                </div>
              </a>
            </div>
            <p>Text between bookmarks</p>
            <div class="bookmark">
              <a href="https://github.com" target="_blank" rel="noopener noreferrer" class="bookmark-link">
                <div class="bookmark-container">
                  <div class="bookmark-info">
                    <div class="bookmark-title">GitHub</div>
                    <div class="bookmark-link-info">
                      <span class="bookmark-domain">github.com</span>
                    </div>
                  </div>
                </div>
              </a>
            </div>
        "#}
    )]
    #[case::no_bookmarks(
        "<p>This content has no bookmarks.</p>",
        "<p>This content has no bookmarks.</p>"
    )]
    #[tokio::test]
    async fn test_convert_simple_bookmarks_to_rich(#[case] input: &str, #[case] expected: &str) {
        // モック関数として実装：実際のHTTPリクエストを行わずフォールバックデータを使用
        let result = convert_simple_bookmarks_to_rich_mock(input).await.unwrap();
        assert_eq!(result, expected);
    }

    /// テスト専用のモック変換関数
    async fn convert_simple_bookmarks_to_rich_mock(html_content: &str) -> Result<String> {
        use crate::bookmark::create_fallback_bookmark_data;
        use regex::Regex;
        use std::sync::LazyLock;

        static BOOKMARK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"<div class="bookmark">\s*<a href="([^"]+)">([^<]*)</a>\s*</div>"#)
                .expect("Invalid bookmark regex pattern")
        });

        let mut result = String::with_capacity(html_content.len() + 2048);
        let mut last_end = 0;

        for capture in BOOKMARK_REGEX.captures_iter(html_content) {
            let full_match = capture.get(0).unwrap();
            let url = &capture[1];
            let original_title = &capture[2];

            result.push_str(&html_content[last_end..full_match.start()]);

            let bookmark_data = create_fallback_bookmark_data(url, original_title);
            let rich_bookmark_html = generate_rich_bookmark(&bookmark_data);
            result.push_str(&rich_bookmark_html);

            last_end = full_match.end();
        }

        result.push_str(&html_content[last_end..]);

        Ok(result)
    }
}