use crate::error::Result;

use scraper::{ElementRef, Html, Selector};
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
    pub(super) async fn fetch(&self, url: &str) -> Result<Metadata> {
        let html_content = {
            let _permit = self
                .permits
                .acquire()
                .await
                .expect("bookmark request semaphore must remain open");
            self.client
                .get(url)
                .send()
                .await?
                .error_for_status()?
                .text()
                .await?
        };

        Ok(parse_metadata(url, &html_content))
    }
}

#[derive(Debug, Default)]
pub(super) struct Metadata {
    pub(super) title: Option<String>,
    pub(super) description: Option<String>,
    pub(super) image_url: Option<String>,
    pub(super) favicon_url: Option<String>,
}

fn parse_metadata(url: &str, html_content: &str) -> Metadata {
    let document = Html::parse_document(html_content);
    let base_url = Url::parse(url).ok();

    Metadata {
        title: extract_first_attribute(
            &document,
            &["meta[property='og:title']", "meta[name='twitter:title']"],
            "content",
        )
        .or_else(|| extract_text(&document, "title")),
        description: extract_first_attribute(
            &document,
            &[
                "meta[property='og:description']",
                "meta[name='twitter:description']",
                "meta[name='description']",
            ],
            "content",
        ),
        image_url: base_url.as_ref().and_then(|base_url| {
            extract_first_url(
                &document,
                base_url,
                &["meta[property='og:image']", "meta[name='twitter:image']"],
                "content",
            )
        }),
        favicon_url: base_url.as_ref().and_then(|base_url| {
            extract_first_url(
                &document,
                base_url,
                &["link[rel~='icon']", "link[rel~='apple-touch-icon']"],
                "href",
            )
        }),
    }
}

fn extract_first_attribute(document: &Html, selectors: &[&str], attribute: &str) -> Option<String> {
    selectors.iter().find_map(|selector| {
        let selector = Selector::parse(selector).expect("metadata selector must be valid");
        document
            .select(&selector)
            .find_map(|element| extract_attribute(element, attribute))
    })
}

fn extract_attribute(element: ElementRef<'_>, attribute: &str) -> Option<String> {
    element
        .value()
        .attr(attribute)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn extract_text(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).expect("metadata selector must be valid");
    let text = document
        .select(&selector)
        .next()?
        .text()
        .collect::<String>();
    let text = text.trim();

    (!text.is_empty()).then(|| text.to_owned())
}

fn extract_first_url(
    document: &Html,
    base_url: &Url,
    selectors: &[&str],
    attribute: &str,
) -> Option<String> {
    selectors.iter().find_map(|selector| {
        let selector = Selector::parse(selector).expect("metadata selector must be valid");
        document.select(&selector).find_map(|element| {
            let value = extract_attribute(element, attribute)?;
            let url = base_url.join(&value).ok()?;

            matches!(url.scheme(), "http" | "https").then(|| url.to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use rstest::rstest;

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

        assert_eq!(metadata.title.as_deref(), Some("Open Graph title"));
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
    fn test_parse_metadata_uses_standard_metadata() {
        let html = indoc! {r#"
            <html>
              <head>
                <title>  Title tag  </title>
                <meta name="description" content="Description">
              </head>
            </html>
        "#};

        let metadata = parse_metadata("https://example.com/articles/page", html);

        assert_eq!(metadata.title.as_deref(), Some("Title tag"));
        assert_eq!(metadata.description.as_deref(), Some("Description"));
        assert_eq!(metadata.image_url, None);
        assert_eq!(metadata.favicon_url, None);
    }

    #[test]
    fn test_parse_metadata_leaves_missing_title_unresolved() {
        let metadata = parse_metadata("https://example.com/articles/page", "<html></html>");

        assert_eq!(metadata.title, None);
    }

    #[rstest]
    #[case::apple_touch("apple-touch-icon")]
    #[case::icon("icon")]
    #[case::shortcut_icon("shortcut icon")]
    #[case::reordered_tokens("icon shortcut")]
    fn test_parse_metadata_matches_favicon_rel_tokens(#[case] rel: &str) {
        let html = format!(r#"<link rel="{rel}" href="/favicon.png">"#);

        let metadata = parse_metadata("https://example.com/articles/page", &html);

        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://example.com/favicon.png")
        );
    }

    #[test]
    fn test_parse_metadata_prefers_standard_icon() {
        let html = indoc! {r#"
            <link rel="apple-touch-icon" href="/apple-touch-icon.png">
            <link rel="icon" href="/favicon.png">
        "#};

        let metadata = parse_metadata("https://example.com/articles/page", html);

        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://example.com/favicon.png")
        );
    }

    #[test]
    fn test_parse_metadata_skips_unsafe_icon_url() {
        let html = indoc! {r#"
            <link rel="icon" href="data:image/png;base64,unsafe">
            <link rel="shortcut icon" href="/favicon.png">
        "#};

        let metadata = parse_metadata("https://example.com/articles/page", html);

        assert_eq!(
            metadata.favicon_url.as_deref(),
            Some("https://example.com/favicon.png")
        );
    }

    #[rstest]
    #[case::relative("/images/card.png", Some("https://example.com/images/card.png"))]
    #[case::absolute(
        "https://cdn.example.com/card.png",
        Some("https://cdn.example.com/card.png")
    )]
    #[case::unsafe_scheme("javascript:alert(1)", None)]
    fn test_parse_metadata_resolves_image_url(#[case] value: &str, #[case] expected: Option<&str>) {
        let html = format!(r#"<meta property="og:image" content="{value}">"#);

        let metadata = parse_metadata("https://example.com/articles/page", &html);

        assert_eq!(metadata.image_url.as_deref(), expected);
    }

    #[test]
    fn test_parse_metadata_skips_unsafe_image_url() {
        let html = indoc! {r#"
            <meta property="og:image" content="javascript:alert(1)">
            <meta name="twitter:image" content="/images/card.png">
        "#};

        let metadata = parse_metadata("https://example.com/articles/page", html);

        assert_eq!(
            metadata.image_url.as_deref(),
            Some("https://example.com/images/card.png")
        );
    }
}
