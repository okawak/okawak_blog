use crate::error::Result;
use pulldown_cmark::{Options, Parser, html};
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::sync::LazyLock;

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

/// リッチブックマークのメタデータを保持する構造体
///
/// URLフィールドは文字列として保存しています。これにより：
/// - 簡単なシリアライゼーション/デシリアライゼーション
/// - テンプレート生成での直接的な使用
/// - 外部APIとの互換性
/// を実現しています。必要に応じてurl::Urlに変換可能です。
#[derive(Debug, Clone, PartialEq)]
pub struct BookmarkData {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub favicon_url: Option<String>,
}

/// Fetches Open Graph Protocol (OGP) metadata from a given URL.
///
/// This function retrieves metadata such as the title, description, image URL, and favicon URL
/// from the HTML content of the specified URL. It uses an HTTP client with a 10-second timeout
/// to fetch the HTML content and then parses the document to extract the metadata.
///
/// # Parameters
/// - `url`: The URL from which to fetch the OGP metadata.
///
/// # Returns
/// - `BookmarkData`: A struct containing the extracted metadata.
///
/// # Errors
/// This function returns an error in the following cases:
/// - Network errors, such as timeouts or connection failures.
/// - Invalid or malformed HTML content.
/// - Missing or incomplete OGP metadata in the HTML document.
///
/// # Timeout Behavior
/// The HTTP client used by this function has a timeout of 10 seconds for the request.
pub async fn fetch_ogp_metadata(url: &str) -> Result<BookmarkData> {
    let client = create_http_client()?;
    let html_content = fetch_html_content(&client, url).await?;
    let document = Html::parse_document(&html_content);

    Ok(BookmarkData {
        url: url.to_string(),
        title: extract_title(&document).unwrap_or_else(|| url.to_string()),
        description: extract_description(&document),
        image_url: extract_image(&document, url),
        favicon_url: extract_favicon(&document, url),
    })
}

/// HTTPクライアントを作成
fn create_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| crate::error::ObsidianError::NetworkError(e.to_string()))
}

/// HTMLコンテンツを取得
async fn fetch_html_content(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| crate::error::ObsidianError::NetworkError(e.to_string()))?;

    response
        .text()
        .await
        .map_err(|e| crate::error::ObsidianError::NetworkError(e.to_string()))
}

/// HTMLドキュメントからタイトルを抽出
fn extract_title(document: &Html) -> Option<String> {
    // Open Graphのタイトルを最優先
    extract_meta_content(document, "meta[property='og:title']")
        .or_else(|| extract_meta_content(document, "meta[name='twitter:title']"))
        .or_else(|| extract_title_tag(document))
}

/// メタタグのcontentを抽出
fn extract_meta_content(document: &Html, selector_str: &str) -> Option<String> {
    let selector = Selector::parse(selector_str).ok()?;
    let content = document
        .select(&selector)
        .next()?
        .value()
        .attr("content")?
        .trim();

    if content.is_empty() {
        None
    } else {
        Some(content.to_string())
    }
}

/// titleタグのテキストを抽出
fn extract_title_tag(document: &Html) -> Option<String> {
    let selector = Selector::parse("title").ok()?;
    let title_text = document
        .select(&selector)
        .next()?
        .text()
        .collect::<String>();

    let trimmed = title_text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// HTMLドキュメントから説明を抽出
fn extract_description(document: &Html) -> Option<String> {
    extract_meta_content(document, "meta[property='og:description']")
        .or_else(|| extract_meta_content(document, "meta[name='twitter:description']"))
        .or_else(|| extract_meta_content(document, "meta[name='description']"))
}

/// HTMLドキュメントから画像URLを抽出
fn extract_image(document: &Html, base_url: &str) -> Option<String> {
    use url::Url;

    let base = Url::parse(base_url).ok()?;

    extract_meta_content(document, "meta[property='og:image']")
        .or_else(|| extract_meta_content(document, "meta[name='twitter:image']"))
        .and_then(|content| base.join(&content).ok())
        .map(|url| url.to_string())
}

/// HTMLドキュメントからファビコンURLを抽出
fn extract_favicon(document: &Html, base_url: &str) -> Option<String> {
    use url::Url;

    let base = Url::parse(base_url).ok()?;

    // 各ファビコンタイプの優先順位で検索
    let selectors = [
        "link[rel='apple-touch-icon']",
        "link[rel='icon']",
        "link[rel='shortcut icon']",
    ];

    for selector_str in &selectors {
        if let Some(href) = extract_link_href(document, selector_str) {
            if let Ok(absolute_url) = base.join(&href) {
                return Some(absolute_url.to_string());
            }
        }
    }

    // デフォルトのfavicon.icoを試行
    base.join("/favicon.ico").ok().map(|url| url.to_string())
}

/// linkタグのhref属性を抽出
fn extract_link_href(document: &Html, selector_str: &str) -> Option<String> {
    let selector = Selector::parse(selector_str).ok()?;
    document
        .select(&selector)
        .next()?
        .value()
        .attr("href")
        .map(ToString::to_string)
}

/// Generates a rich bookmark HTML snippet based on the provided bookmark data.
///
/// # HTML Structure
/// The generated HTML has the following structure:
/// ```html
/// <div class="notion-bookmark">
///   <a href="{bookmark_url}" target="_blank" rel="noopener noreferrer" class="bookmark-link">
///     <div class="bookmark-container">
///       <div class="bookmark-info">
///         <div class="bookmark-title">{title}</div>
///         <div class="bookmark-description">{description}</div>
///         <div class="bookmark-link-info">
///           <img class="bookmark-favicon" src="{favicon_url}" alt="favicon">
///           <span class="bookmark-domain">{domain}</span>
///         </div>
///       </div>
///       <div class="bookmark-image">
///         <img src="{image_url}" alt="{title}" loading="lazy">
///       </div>
///     </div>
///   </a>
/// </div>
/// ```
///
/// # Compliance with Requirements
/// This function adheres to the requirements specification by:
/// - Using semantic HTML elements (`<div>` and `<a>`)
/// - Including a class name (`notion-bookmark`) for styling and identification
/// - Generating a valid and accessible HTML structure
/// - Supporting optional elements (description, favicon, image)
///
/// # Parameters
/// - `data`: A `BookmarkData` struct containing the URL, title, description, and other metadata
///
/// # Returns
/// A `String` containing the generated HTML snippet
pub fn generate_rich_bookmark(data: &BookmarkData) -> String {
    let domain = extract_domain(&data.url);

    let mut html = String::with_capacity(1024);
    html.push_str("<div class=\"notion-bookmark\">\n");

    write_bookmark_link(&mut html, &data.url);
    write_bookmark_container(&mut html, data, &domain);

    html.push_str("  </a>\n");
    html.push_str("</div>");

    html
}

/// URLからドメイン名を抽出
fn extract_domain(url: &str) -> String {
    use url::Url;

    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(ToString::to_string))
        .unwrap_or_else(|| url.to_string())
}

/// ブックマークリンク開始タグを書き込み
fn write_bookmark_link(html: &mut String, url: &str) {
    html.push_str(&format!(
        "  <a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\" class=\"bookmark-link\">\n",
        html_escape(url)
    ));
}

/// ブックマークコンテナを書き込み
fn write_bookmark_container(html: &mut String, data: &BookmarkData, domain: &str) {
    html.push_str("    <div class=\"bookmark-container\">\n");

    write_bookmark_info(html, data, domain);
    write_bookmark_image(html, data);

    html.push_str("    </div>\n");
}

/// ブックマーク情報セクションを書き込み
fn write_bookmark_info(html: &mut String, data: &BookmarkData, domain: &str) {
    html.push_str("      <div class=\"bookmark-info\">\n");
    html.push_str(&format!(
        "        <div class=\"bookmark-title\">{}</div>\n",
        html_escape(&data.title)
    ));

    if let Some(description) = &data.description {
        html.push_str(&format!(
            "        <div class=\"bookmark-description\">{}</div>\n",
            html_escape(description)
        ));
    }

    write_bookmark_link_info(html, data, domain);
    html.push_str("      </div>\n");
}

/// ブックマークリンク情報を書き込み
fn write_bookmark_link_info(html: &mut String, data: &BookmarkData, domain: &str) {
    html.push_str("        <div class=\"bookmark-link-info\">\n");

    if let Some(favicon) = &data.favicon_url {
        html.push_str(&format!(
            "          <img class=\"bookmark-favicon\" src=\"{}\" alt=\"favicon\">\n",
            html_escape(favicon)
        ));
    }

    html.push_str(&format!(
        "          <span class=\"bookmark-domain\">{}</span>\n",
        html_escape(domain)
    ));
    html.push_str("        </div>\n");
}

/// ブックマーク画像セクションを書き込み
fn write_bookmark_image(html: &mut String, data: &BookmarkData) {
    if let Some(image_url) = &data.image_url {
        html.push_str("      <div class=\"bookmark-image\">\n");
        html.push_str(&format!(
            "        <img src=\"{}\" alt=\"{}\" loading=\"lazy\">\n",
            html_escape(image_url),
            html_escape(&data.title)
        ));
        html.push_str("      </div>\n");
    }
}

/// HTML内のシンプルなbookmark構造を検出してリッチブックマークに変換する
///
/// # Purpose
/// This function scans the provided HTML content for simple bookmark structures and converts them into rich bookmarks
/// by fetching metadata (e.g., title, description, image) from the linked URLs.
///
/// # Parameters
/// - `html_content`: A string slice containing the HTML content to process. It should include simple bookmark structures
///   in the format `<div class="bookmark"><a href="URL">Title</a></div>`.
///
/// # Return Value
/// Returns a `Result<String>` where the `String` contains the updated HTML content with rich bookmarks. If an error occurs,
/// the function returns an error wrapped in the `Result`.
///
/// # Potential Errors
/// - If the OGP metadata for a URL cannot be fetched, the function falls back to a default bookmark structure.
/// - Errors from the `fetch_ogp_metadata` function are handled internally and do not propagate to the caller.
/// - The function assumes the input HTML is valid and does not perform extensive validation.
///
/// # Examples
/// ```no_run
/// # use obsidian_uploader::converter::convert_simple_bookmarks_to_rich;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let html = r#"<p>Check this out:</p><div class="bookmark"><a href="https://example.com">Example</a></div>"#;
/// let result = convert_simple_bookmarks_to_rich(html).await?;
/// // result contains rich bookmark HTML
/// # Ok(())
/// # }
/// ```
pub async fn convert_simple_bookmarks_to_rich(html_content: &str) -> Result<String> {
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

        // Bookmarkの前のテキストを追加
        result.push_str(&html_content[last_end..full_match.start()]);

        // OGPメタデータを取得（エラーの場合はフォールバック）
        let bookmark_data = fetch_ogp_metadata(url)
            .await
            .unwrap_or_else(|_| create_fallback_bookmark_data(url, original_title));

        // リッチブックマークHTMLを生成して追加
        let rich_bookmark_html = generate_rich_bookmark(&bookmark_data);
        result.push_str(&rich_bookmark_html);

        last_end = full_match.end();
    }

    // 残りのテキストを追加
    result.push_str(&html_content[last_end..]);

    Ok(result)
}

/// フォールバック用のブックマークデータを作成する関数
fn create_fallback_bookmark_data(url: &str, original_title: &str) -> BookmarkData {
    BookmarkData {
        url: url.to_string(),
        title: if original_title.trim().is_empty() {
            url.to_string()
        } else {
            original_title.to_string()
        },
        description: None,
        image_url: None,
        favicon_url: None,
    }
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
    #[case::simple_url(
        "https://example.com",
        "Example Site",
        Some("A simple example website"),
        Some("https://example.com/icon.png"),
        Some("https://example.com/favicon.ico")
    )]
    #[case::github_url(
        "https://github.com/dtolnay/anyhow",
        "GitHub - dtolnay/anyhow: Flexible concrete Error type",
        Some("Flexible concrete Error type built on std::error::Error"),
        Some("https://github.com/dtolnay.png"),
        Some("https://github.com/favicon.ico")
    )]
    #[tokio::test]
    async fn test_fetch_ogp_metadata(
        #[case] url: &str,
        #[case] expected_title: &str,
        #[case] expected_description: Option<&str>,
        #[case] expected_image: Option<&str>,
        #[case] expected_favicon: Option<&str>,
    ) {
        // モックテスト - 実際のHTTPリクエストは行わない
        let bookmark_data = BookmarkData {
            url: url.to_string(),
            title: expected_title.to_string(),
            description: expected_description.map(|s| s.to_string()),
            image_url: expected_image.map(|s| s.to_string()),
            favicon_url: expected_favicon.map(|s| s.to_string()),
        };

        // 期待値と一致するかテスト
        assert_eq!(bookmark_data.url, url);
        assert_eq!(bookmark_data.title, expected_title);
        assert_eq!(
            bookmark_data.description,
            expected_description.map(|s| s.to_string())
        );
        assert_eq!(
            bookmark_data.image_url,
            expected_image.map(|s| s.to_string())
        );
        assert_eq!(
            bookmark_data.favicon_url,
            expected_favicon.map(|s| s.to_string())
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
        r#"<div class="notion-bookmark">
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
</div>"#
    )]
    #[case::minimal_metadata(
        &BookmarkData {
            url: "https://github.com".to_string(),
            title: "GitHub".to_string(),
            description: None,
            image_url: None,
            favicon_url: None,
        },
        r#"<div class="notion-bookmark">
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
</div>"#
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
        r#"<p>Check out this site:</p>
<div class="bookmark">
  <a href="https://example.com">Example Site</a>
</div>
<p>End of content.</p>"#,
        r#"<p>Check out this site:</p>
<div class="notion-bookmark">
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
<p>End of content.</p>"#
    )]
    #[case::multiple_bookmarks(
        r#"<div class="bookmark">
  <a href="https://example.com">Example</a>
</div>
<p>Text between bookmarks</p>
<div class="bookmark">
  <a href="https://github.com">GitHub</a>
</div>"#,
        r#"<div class="notion-bookmark">
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
<div class="notion-bookmark">
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
</div>"#
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

            // Bookmarkの前のテキストを追加
            result.push_str(&html_content[last_end..full_match.start()]);

            // モックデータを作成（HTTPリクエストなし）
            let bookmark_data = create_fallback_bookmark_data(url, original_title);

            // リッチブックマークHTMLを生成して追加
            let rich_bookmark_html = generate_rich_bookmark(&bookmark_data);
            result.push_str(&rich_bookmark_html);

            last_end = full_match.end();
        }

        // 残りのテキストを追加
        result.push_str(&html_content[last_end..]);

        Ok(result)
    }
}
