use crate::error::{ObsidianError, Result};
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::LazyLock;

/// リッチブックマークのメタデータを保持する構造体
#[derive(Debug, Clone, PartialEq)]
pub struct BookmarkData {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub favicon_url: Option<String>,
}

/// URLからOGPメタデータを取得する
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
    let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| ObsidianError::NetworkError(e.to_string()))
}

/// HTMLコンテンツを取得
async fn fetch_html_content(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| ObsidianError::NetworkError(e.to_string()))?;

    response
        .text()
        .await
        .map_err(|e| ObsidianError::NetworkError(e.to_string()))
}

/// HTMLドキュメントからタイトルを抽出
fn extract_title(document: &Html) -> Option<String> {
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

/// リッチブックマークHTMLを生成する
pub fn generate_rich_bookmark(data: &BookmarkData) -> String {
    let domain = extract_domain(&data.url);

    let mut html = String::with_capacity(1024);
    html.push_str("<div class=\"bookmark\">\n");

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

/// HTMLエスケープ処理
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// HTML内のシンプルなbookmark構造を検出してリッチブックマークに変換する
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

        result.push_str(&html_content[last_end..full_match.start()]);

        let bookmark_data = fetch_ogp_metadata(url)
            .await
            .unwrap_or_else(|_| create_fallback_bookmark_data(url, original_title));

        let rich_bookmark_html = generate_rich_bookmark(&bookmark_data);
        result.push_str(&rich_bookmark_html);

        last_end = full_match.end();
    }

    result.push_str(&html_content[last_end..]);

    Ok(result)
}

/// フォールバック用のブックマークデータを作成する関数
pub fn create_fallback_bookmark_data(url: &str, original_title: &str) -> BookmarkData {
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