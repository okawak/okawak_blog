use super::ogp::{self, BookmarkMetadata};
use crate::error::Result;

use futures::future::BoxFuture;
use html_escape::{encode_double_quoted_attribute, encode_text};
use regex::Regex;
use std::future::Future;
use std::ops::Range;
use std::sync::Arc;
use std::sync::LazyLock;

const HTML_INITIAL_CAPACITY: usize = 1024;
const HTML_EXTENSION_CAPACITY: usize = 2048;
const SIMPLE_BOOKMARK_OPEN: &str = r#"<div class="bookmark">"#;
const SIMPLE_BOOKMARK_CLOSE: &str = "</div>";
static SIMPLE_BOOKMARK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"{SIMPLE_BOOKMARK_OPEN}\s*<a href="([^"]+)">([^<]*)</a>\s*{SIMPLE_BOOKMARK_CLOSE}"#
    ))
    .expect("Invalid bookmark regex pattern")
});

/// Async function that enriches page HTML with rich bookmark cards.
pub type BookmarkEnricher = Arc<dyn Fn(String) -> BoxFuture<'static, Result<String>> + Send + Sync>;

pub(crate) fn rich_bookmark_enricher() -> BookmarkEnricher {
    Arc::new(|html: String| {
        Box::pin(async move { Ok(convert_simple_bookmarks_to_rich(&html).await) })
    })
}

pub(super) struct SimpleBookmark<'a> {
    source: &'a str,
    range: Range<usize>,
    href_range: Range<usize>,
    href: &'a str,
    title: &'a str,
}

impl SimpleBookmark<'_> {
    fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    pub(super) fn href(&self) -> &str {
        self.href
    }

    fn title(&self) -> &str {
        self.title
    }

    pub(super) fn with_href(&self, href: &str) -> String {
        let mut output = String::with_capacity(self.source.len());
        output.push_str(&self.source[..self.href_range.start]);
        output.push_str(href);
        output.push_str(&self.source[self.href_range.end..]);
        output
    }
}

pub(super) fn parse_simple_bookmark(html: &str) -> Option<SimpleBookmark<'_>> {
    let bookmark = simple_bookmarks(html).next()?;
    let range = bookmark.range();
    (html[..range.start].trim().is_empty() && html[range.end..].trim().is_empty())
        .then_some(bookmark)
}

pub(super) fn is_simple_bookmark_start(html: &str) -> bool {
    html.trim_start().starts_with(SIMPLE_BOOKMARK_OPEN)
}

pub(super) fn simple_bookmark_end(html: &str) -> Option<usize> {
    html.find(SIMPLE_BOOKMARK_CLOSE)
        .map(|start| start + SIMPLE_BOOKMARK_CLOSE.len())
}

fn simple_bookmarks(html: &str) -> impl Iterator<Item = SimpleBookmark<'_>> {
    SIMPLE_BOOKMARK_RE.captures_iter(html).map(|captures| {
        let full_match = captures
            .get(0)
            .expect("Bookmark regex must capture a match");
        let href = captures
            .get(1)
            .expect("Bookmark regex must capture an href");
        let title = captures
            .get(2)
            .expect("Bookmark regex must capture a title");

        SimpleBookmark {
            source: html,
            range: full_match.range(),
            href_range: href.range(),
            href: href.as_str(),
            title: title.as_str(),
        }
    })
}

/// Generates rich bookmark HTML using the `bookmark` class.
fn generate_rich_bookmark(data: &BookmarkMetadata) -> String {
    let domain = extract_domain(&data.url);

    let mut html = String::with_capacity(HTML_INITIAL_CAPACITY);
    html.push_str("<div class=\"bookmark\">\n");

    write_bookmark_link(&mut html, &data.url);
    write_bookmark_container(&mut html, data, &domain);

    html.push_str("  </a>\n");
    html.push_str("</div>");

    html
}

fn extract_domain(url: &str) -> String {
    use url::Url;

    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(ToString::to_string))
        .unwrap_or_else(|| url.to_string())
}

fn write_bookmark_link(html: &mut String, url: &str) {
    html.push_str(&format!(
        "  <a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\" class=\"bookmark-link\">\n",
        encode_double_quoted_attribute(url)
    ));
}

fn write_bookmark_container(html: &mut String, data: &BookmarkMetadata, domain: &str) {
    html.push_str("    <div class=\"bookmark-container\">\n");

    write_bookmark_info(html, data, domain);
    write_bookmark_image(html, data);

    html.push_str("    </div>\n");
}

fn write_bookmark_info(html: &mut String, data: &BookmarkMetadata, domain: &str) {
    html.push_str("      <div class=\"bookmark-info\">\n");
    html.push_str(&format!(
        "        <div class=\"bookmark-title\">{}</div>\n",
        encode_text(&data.title)
    ));

    if let Some(description) = &data.description {
        html.push_str(&format!(
            "        <div class=\"bookmark-description\">{}</div>\n",
            encode_text(description)
        ));
    }

    write_bookmark_link_info(html, data, domain);
    html.push_str("      </div>\n");
}

fn write_bookmark_link_info(html: &mut String, data: &BookmarkMetadata, domain: &str) {
    html.push_str("        <div class=\"bookmark-link-info\">\n");

    if let Some(favicon) = &data.favicon_url {
        html.push_str(&format!(
            "          <img class=\"bookmark-favicon\" src=\"{}\" alt=\"favicon\">\n",
            encode_double_quoted_attribute(favicon)
        ));
    }

    html.push_str(&format!(
        "          <span class=\"bookmark-domain\">{}</span>\n",
        encode_text(domain)
    ));
    html.push_str("        </div>\n");
}

fn write_bookmark_image(html: &mut String, data: &BookmarkMetadata) {
    if let Some(image_url) = &data.image_url {
        html.push_str("      <div class=\"bookmark-image\">\n");
        html.push_str(&format!(
            "        <img src=\"{}\" alt=\"{}\" loading=\"lazy\">\n",
            encode_double_quoted_attribute(image_url),
            encode_double_quoted_attribute(&data.title)
        ));
        html.push_str("      </div>\n");
    }
}

/// Replaces simple bookmark markup using metadata supplied by `fetch_data`.
async fn convert_simple_bookmarks_with<F, Fut>(html_content: &str, fetch_data: F) -> String
where
    F: Fn(String, String) -> Fut,
    Fut: Future<Output = BookmarkMetadata>,
{
    let mut result = String::with_capacity(html_content.len() + HTML_EXTENSION_CAPACITY);
    let mut last_end = 0;

    for bookmark in simple_bookmarks(html_content) {
        let range = bookmark.range();
        let url = bookmark.href().to_string();
        let original_title = bookmark.title().to_string();

        result.push_str(&html_content[last_end..range.start]);

        let metadata = fetch_data(url, original_title).await;
        let rich_bookmark_html = generate_rich_bookmark(&metadata);
        result.push_str(&rich_bookmark_html);

        last_end = range.end;
    }

    result.push_str(&html_content[last_end..]);

    result
}

/// Replaces simple bookmark markup with rich bookmark cards fetched from OGP metadata.
async fn convert_simple_bookmarks_to_rich(html_content: &str) -> String {
    convert_simple_bookmarks_with(html_content, |url, original_title| async move {
        ogp::fetch(&url).await.unwrap_or_else(|error| {
            tracing::warn!(%url, %error, "failed to fetch OGP metadata");
            ogp::fallback(&url, &original_title)
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::*;

    #[rstest]
    #[case::single_line(
        r#"<div class="bookmark"><a href="https://example.com">Example</a></div>"#,
        true
    )]
    #[case::multiline(
        indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com">Example</a>
            </div>
        "#},
        true
    )]
    #[case::extra_anchor_attribute(
        r#"<div class="bookmark"><a href="https://example.com" target="_blank">Example</a></div>"#,
        false
    )]
    #[case::nested_content(
        r#"<div class="bookmark"><a href="https://example.com"><strong>Example</strong></a></div>"#,
        false
    )]
    fn test_parse_simple_bookmark(#[case] html: &str, #[case] expected: bool) {
        assert_eq!(parse_simple_bookmark(html).is_some(), expected);
    }

    #[rstest]
    #[case::full_metadata(
        &BookmarkMetadata {
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
            </div>"#}
    )]
    #[case::minimal_metadata(
        &BookmarkMetadata {
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
            </div>"#}
    )]
    fn test_generate_rich_bookmark(
        #[case] bookmark_data: &BookmarkMetadata,
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
        let result = convert_simple_bookmarks_with(input, |url, title| async move {
            ogp::fallback(&url, &title)
        })
        .await;

        assert_eq!(result, expected);
    }
}
