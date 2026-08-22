use crate::error::Result;

use scraper::{Html, Selector};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use url::Url;

const CONCURRENT_REQUEST_LIMIT: usize = 8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const USER_AGENT: &str = "publish-bookmark/1.0 (+https://github.com/okawak/okawak_blog)";

#[derive(Clone)]
pub(super) struct Fetcher {
    client: reqwest::Client,
    permits: Arc<Semaphore>,
}

impl Fetcher {
    pub(super) fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Self {
            client,
            permits: Arc::new(Semaphore::new(CONCURRENT_REQUEST_LIMIT)),
        })
    }

    /// Fetches bookmark metadata while bounding requests across all rendered documents.
    pub(super) async fn fetch(&self, url: &str) -> Result<BookmarkMetadata> {
        let html_content = {
            let _permit = self
                .permits
                .acquire()
                .await
                .expect("bookmark request semaphore must remain open");
            fetch_html_content(&self.client, url).await?
        };

        Ok(parse_metadata(url, &html_content))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct BookmarkMetadata {
    pub(super) url: String,
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) image_url: Option<String>,
    pub(super) favicon_url: Option<String>,
}

pub(super) fn fallback(url: &str, original_title: &str) -> BookmarkMetadata {
    BookmarkMetadata {
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

async fn fetch_html_content(client: &reqwest::Client, url: &str) -> Result<String> {
    let response = client.get(url).send().await?;

    response.text().await.map_err(Into::into)
}

fn parse_metadata(url: &str, html_content: &str) -> BookmarkMetadata {
    let document = Html::parse_document(html_content);

    BookmarkMetadata {
        url: url.to_string(),
        title: extract_title(&document).unwrap_or_else(|| url.to_string()),
        description: extract_description(&document),
        image_url: extract_image(&document, url),
        favicon_url: extract_favicon(&document, url),
    }
}

fn extract_title(document: &Html) -> Option<String> {
    extract_meta_content(document, "meta[property='og:title']")
        .or_else(|| extract_meta_content(document, "meta[name='twitter:title']"))
        .or_else(|| extract_title_tag(document))
}

fn extract_meta_content(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
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

fn extract_description(document: &Html) -> Option<String> {
    extract_meta_content(document, "meta[property='og:description']")
        .or_else(|| extract_meta_content(document, "meta[name='twitter:description']"))
        .or_else(|| extract_meta_content(document, "meta[name='description']"))
}

fn extract_image(document: &Html, base_url: &str) -> Option<String> {
    let base = Url::parse(base_url).ok()?;

    extract_meta_content(document, "meta[property='og:image']")
        .or_else(|| extract_meta_content(document, "meta[name='twitter:image']"))
        .and_then(|content| resolve_url(&base, content))
}

fn extract_favicon(document: &Html, base_url: &str) -> Option<String> {
    let base = Url::parse(base_url).ok()?;
    let selectors = [
        "link[rel='apple-touch-icon']",
        "link[rel='icon']",
        "link[rel='shortcut icon']",
    ];

    selectors
        .iter()
        .find_map(|selector| {
            extract_link_href(document, selector).and_then(|href| resolve_url(&base, href))
        })
        .or_else(|| base.join("/favicon.ico").ok().map(|url| url.to_string()))
}

fn extract_link_href(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).ok()?;
    document
        .select(&selector)
        .next()?
        .value()
        .attr("href")
        .map(ToString::to_string)
}

fn resolve_url(base: &Url, value: String) -> Option<String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        Some(value)
    } else {
        base.join(&value).ok().map(|url| url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_parse_metadata_prefers_open_graph_values() {
        let html = indoc! {r#"
            <html>
              <head>
                <title>Title tag</title>
                <meta property="og:title" content="Open Graph title">
                <meta property="og:description" content="Open Graph description">
                <meta property="og:image" content="/images/card.png">
                <link rel="icon" href="/favicon.png">
              </head>
            </html>
        "#};

        let metadata = parse_metadata("https://example.com/articles/page", html);

        assert_eq!(metadata.title, "Open Graph title");
        assert_eq!(
            metadata.description.as_deref(),
            Some("Open Graph description")
        );
        assert_eq!(
            metadata.image_url.as_deref(),
            Some("https://example.com/images/card.png")
        );
        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://example.com/favicon.png")
        );
    }

    #[test]
    fn test_parse_metadata_uses_fallback_values() {
        let html = indoc! {r#"
            <html>
              <head>
                <title>  Title tag  </title>
                <meta name="description" content="Description">
              </head>
            </html>
        "#};

        let metadata = parse_metadata("https://example.com/articles/page", html);

        assert_eq!(metadata.title, "Title tag");
        assert_eq!(metadata.description.as_deref(), Some("Description"));
        assert_eq!(metadata.image_url, None);
        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://example.com/favicon.ico")
        );
    }
}
