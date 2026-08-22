use super::ogp::{self, BookmarkMetadata};

use futures::future::{BoxFuture, join_all};
use html_escape::{encode_double_quoted_attribute, encode_text};
use indoc::formatdoc;
use regex::Regex;
use std::{
    future::Future,
    ops::Range,
    sync::{Arc, LazyLock},
};

const SIMPLE_BOOKMARK_OPEN: &str = r#"<div class="bookmark">"#;
const SIMPLE_BOOKMARK_CLOSE: &str = "</div>";
static SIMPLE_BOOKMARK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r#"{SIMPLE_BOOKMARK_OPEN}\s*<a href="([^"]+)">([^<]*)</a>\s*{SIMPLE_BOOKMARK_CLOSE}"#
    ))
    .expect("Invalid bookmark regex pattern")
});

/// Async function that enriches page HTML with rich bookmark cards.
pub type BookmarkEnricher = Arc<dyn Fn(String) -> BoxFuture<'static, String> + Send + Sync>;

pub(crate) fn rich_bookmark_enricher() -> BookmarkEnricher {
    let fetcher = match ogp::Fetcher::new() {
        Ok(fetcher) => Some(fetcher),
        Err(error) => {
            tracing::warn!(%error, "failed to initialize OGP metadata fetcher");
            None
        }
    };

    Arc::new(move |html: String| {
        let fetcher = fetcher.clone();
        Box::pin(async move { convert_simple_bookmarks_to_rich(&html, fetcher).await })
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

/// Replaces simple bookmark markup with rich bookmark cards fetched from OGP metadata.
async fn convert_simple_bookmarks_to_rich(
    html_content: &str,
    fetcher: Option<ogp::Fetcher>,
) -> String {
    convert_simple_bookmarks_with(html_content, move |url, original_title| {
        let fetcher = fetcher.clone();

        async move {
            match fetcher {
                Some(fetcher) => fetcher.fetch(&url).await.unwrap_or_else(|error| {
                    tracing::warn!(%url, %error, "failed to fetch OGP metadata");
                    BookmarkMetadata::fallback(&url, &original_title)
                }),
                None => BookmarkMetadata::fallback(&url, &original_title),
            }
        }
    })
    .await
}

/// Replaces simple bookmark markup using metadata supplied by `fetch_data`.
async fn convert_simple_bookmarks_with<F, Fut>(html_content: &str, fetch_data: F) -> String
where
    F: Fn(String, String) -> Fut,
    Fut: Future<Output = BookmarkMetadata>,
{
    let mut result = String::with_capacity(html_content.len());
    let mut last_end = 0;
    let bookmarks = simple_bookmarks(html_content).map(|bookmark| {
        let range = bookmark.range();
        let metadata = fetch_data(bookmark.href().to_string(), bookmark.title().to_string());

        async move { (range, metadata.await) }
    });

    // `join_all` polls metadata fetches concurrently and returns them in source order.
    for (range, metadata) in join_all(bookmarks).await {
        result.push_str(&html_content[last_end..range.start]);
        let rich_bookmark_html = generate_rich_bookmark(&metadata);
        result.push_str(&rich_bookmark_html);

        last_end = range.end;
    }

    result.push_str(&html_content[last_end..]);

    result
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
    let description_html = data
        .description
        .as_ref()
        .map_or_else(String::new, |description| {
            format!(
                r#"<div class="bookmark-description">{}</div>"#,
                encode_text(description),
            )
        });
    let favicon_html = data
        .favicon_url
        .as_ref()
        .map_or_else(String::new, |favicon| {
            format!(
                r#"<img class="bookmark-favicon" src="{}" alt="favicon">"#,
                encode_double_quoted_attribute(favicon),
            )
        });
    let image_html = data
        .image_url
        .as_ref()
        .map_or_else(String::new, |image_url| {
            formatdoc! {r#"
                <div class="bookmark-image">
                  <img src="{}" alt="{}" loading="lazy">
                </div>"#,
                encode_double_quoted_attribute(image_url),
                encode_double_quoted_attribute(&data.title),
            }
        });

    formatdoc! {r#"
        <div class="bookmark">
          <a href="{url}" target="_blank" rel="noopener noreferrer" class="bookmark-link">
            <div class="bookmark-container">
              <div class="bookmark-info">
                <div class="bookmark-title">{title}</div>
                {description_html}
                <div class="bookmark-link-info">
                  {favicon_html}
                  <span class="bookmark-domain">{domain}</span>
                </div>
              </div>
              {image_html}
            </div>
          </a>
        </div>"#,
        url = encode_double_quoted_attribute(&data.url),
        title = encode_text(&data.title),
        domain = encode_text(&domain),
    }
}

fn extract_domain(url: &str) -> String {
    use url::Url;

    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(ToString::to_string))
        .unwrap_or_else(|| url.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use regex::Regex;
    use rstest::*;
    use std::sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::{
        sync::Barrier,
        time::{Duration, sleep, timeout},
    };

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
        assert_html_eq(&result, expected_html);
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
            BookmarkMetadata::fallback(&url, &title)
        })
        .await;

        assert_html_eq(&result, expected);
    }

    #[tokio::test]
    async fn test_convert_simple_bookmarks_to_rich_without_fetcher() {
        let input = r#"<div class="bookmark"><a href="https://example.com">Example</a></div>"#;

        let result = convert_simple_bookmarks_to_rich(input, None).await;

        assert!(result.contains("class=\"bookmark-link\""));
        assert!(result.contains("https://example.com"));
        assert!(result.contains(">Example</div>"));
    }

    #[tokio::test]
    async fn test_convert_simple_bookmarks_fetches_concurrently_and_preserves_order() {
        let input = indoc! {r#"
            <div class="bookmark"><a href="https://example.com">Example</a></div>
            <div class="bookmark"><a href="https://github.com">GitHub</a></div>
        "#};
        let barrier = Arc::new(Barrier::new(2));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let result = timeout(
            Duration::from_secs(1),
            convert_simple_bookmarks_with(input, {
                let barrier = Arc::clone(&barrier);
                let active = Arc::clone(&active);
                let max_active = Arc::clone(&max_active);

                move |url, title| {
                    let barrier = Arc::clone(&barrier);
                    let active = Arc::clone(&active);
                    let max_active = Arc::clone(&max_active);

                    async move {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(current, Ordering::SeqCst);
                        barrier.wait().await;

                        if url == "https://example.com" {
                            sleep(Duration::from_millis(20)).await;
                        }

                        active.fetch_sub(1, Ordering::SeqCst);
                        BookmarkMetadata::fallback(&url, &title)
                    }
                }
            }),
        )
        .await
        .expect("bookmark metadata fetches should run concurrently");

        assert_eq!(max_active.load(Ordering::SeqCst), 2);
        assert!(
            result.find("https://example.com").unwrap()
                < result.find("https://github.com").unwrap()
        );
    }

    fn assert_html_eq(actual: &str, expected: &str) {
        static INTER_TAG_WHITESPACE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r">\s+<").expect("Invalid test regex pattern"));

        assert_eq!(
            INTER_TAG_WHITESPACE_RE.replace_all(actual, "><"),
            INTER_TAG_WHITESPACE_RE.replace_all(expected, "><"),
        );
    }
}
