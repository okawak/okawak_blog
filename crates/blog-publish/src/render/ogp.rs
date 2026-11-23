use crate::error::{PublishError, Result};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

use scraper::{ElementRef, Html, Selector};
use std::{
    io,
    net::IpAddr,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::sync::Semaphore;
use url::Url;

const CONCURRENT_REQUEST_LIMIT: usize = 8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HTML_BYTES: usize = 1024 * 1024;
const USER_AGENT: &str = "publish-bookmark/1.0 (+https://github.com/okawak/okawak_blog)";

static TITLE_SELECTORS: LazyLock<[Selector; 2]> = LazyLock::new(|| {
    ["meta[property='og:title']", "meta[name='twitter:title']"].map(parse_selector)
});
static TITLE_ELEMENT: LazyLock<Selector> = LazyLock::new(|| parse_selector("title"));
static DESCRIPTION_SELECTORS: LazyLock<[Selector; 3]> = LazyLock::new(|| {
    [
        "meta[property='og:description']",
        "meta[name='twitter:description']",
        "meta[name='description']",
    ]
    .map(parse_selector)
});
static IMAGE_SELECTORS: LazyLock<[Selector; 2]> = LazyLock::new(|| {
    ["meta[property='og:image']", "meta[name='twitter:image']"].map(parse_selector)
});
static ICON_SELECTORS: LazyLock<[Selector; 2]> =
    LazyLock::new(|| ["link[rel~='icon']", "link[rel~='apple-touch-icon']"].map(parse_selector));

fn parse_selector(selector: &str) -> Selector {
    Selector::parse(selector).expect("metadata selector must be valid")
}

// Permit ordinary Internet unicast destinations. Transition and special-use
// ranges can represent destinations on a different network than the visible IP.
fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_documentation()
                && a != 0
                && a < 224
                && !(a == 100 && (64..128).contains(&b))
                && !matches!((a, b, c), (192, 0, 0) | (192, 88, 99) | (198, 18..=19, _))
        }
        IpAddr::V6(ip) => {
            let [a, b, ..] = ip.segments();
            (0x2000..0x4000).contains(&a)
                && !(a == 0x2001 && (b < 0x0200 || b == 0x0db8))
                && a != 0x2002
                && !(a == 0x3fff && b < 0x1000)
        }
    }
}

fn public_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && match url.host() {
            Some(url::Host::Domain(_)) => true, // The connector resolves and checks this name.
            Some(url::Host::Ipv4(ip)) => public_address(ip.into()),
            Some(url::Host::Ipv6(ip)) => public_address(ip.into()),
            None => false,
        }
}

struct PublicResolver;

impl Resolve for PublicResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let addresses: Vec<_> = tokio::net::lookup_host((name.as_str(), 0)).await?.collect();
            if addresses.is_empty() || addresses.iter().any(|addr| !public_address(addr.ip())) {
                return Err(io::Error::other("bookmark address is not public").into());
            }
            // Return exactly the checked addresses to the socket connector.
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

#[derive(Clone)]
pub(super) struct Fetcher {
    client: reqwest::Client,
    permits: Arc<Semaphore>,
}

impl Fetcher {
    pub(super) fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .dns_resolver(Arc::new(PublicResolver))
            .redirect(reqwest::redirect::Policy::custom(|hop| {
                if hop.previous().len() >= 5 || !public_url(hop.url()) {
                    hop.error("bookmark redirect rejected")
                } else {
                    hop.follow()
                }
            }))
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
        let url = Url::parse(url)
            .ok()
            .filter(public_url)
            .ok_or_else(|| PublishError::Parse("invalid public bookmark URL".into()))?;
        let _permit = self
            .permits
            .acquire()
            .await
            .expect("bookmark semaphore stays open");
        read_metadata(self.client.get(url).send().await?).await
    }
}

async fn read_metadata(response: reqwest::Response) -> Result<Metadata> {
    let mut response = response.error_for_status()?;
    let url = response.url().clone();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        append_html(&mut bytes, &chunk)?;
    }
    // Keep reqwest's Content-Type charset and byte-order-mark handling.
    let response: http::Response<reqwest::Body> = response.into();
    let text = reqwest::Response::from(response.map(|_| bytes))
        .text()
        .await?;
    Ok(parse_metadata(url.as_str(), &text))
}

fn append_html(bytes: &mut Vec<u8>, chunk: &[u8]) -> Result<()> {
    if chunk.len() > MAX_HTML_BYTES - bytes.len() {
        return Err(PublishError::Parse(
            "bookmark response exceeds size limit".into(),
        ));
    }
    bytes.extend_from_slice(chunk);
    Ok(())
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
        title: extract_first_attribute(&document, &*TITLE_SELECTORS, "content")
            .or_else(|| extract_text(&document, &TITLE_ELEMENT)),
        description: extract_first_attribute(&document, &*DESCRIPTION_SELECTORS, "content"),
        image_url: base_url.as_ref().and_then(|base_url| {
            extract_first_url(&document, base_url, &*IMAGE_SELECTORS, "content")
        }),
        favicon_url: base_url
            .as_ref()
            .and_then(|base_url| extract_first_url(&document, base_url, &*ICON_SELECTORS, "href")),
    }
}

fn extract_first_attribute(
    document: &Html,
    selectors: &[Selector],
    attribute: &str,
) -> Option<String> {
    selectors.iter().find_map(|selector| {
        document
            .select(selector)
            .find_map(|element| extract_attribute(element, attribute))
            .map(str::to_owned)
    })
}

fn extract_attribute<'a>(element: ElementRef<'a>, attribute: &str) -> Option<&'a str> {
    element
        .value()
        .attr(attribute)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn extract_text(document: &Html, selector: &Selector) -> Option<String> {
    let text = document.select(selector).next()?.text().collect::<String>();
    let text = text.trim();

    (!text.is_empty()).then(|| text.to_owned())
}

fn extract_first_url(
    document: &Html,
    base_url: &Url,
    selectors: &[Selector],
    attribute: &str,
) -> Option<String> {
    selectors.iter().find_map(|selector| {
        document.select(selector).find_map(|element| {
            let value = extract_attribute(element, attribute)?;
            let url = base_url.join(value).ok()?;

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
    fn checks_special_use_and_public_addresses() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "169.254.1.1",
            "100.64.0.1",
            "100.127.255.255",
            "198.18.0.1",
            "224.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "64:ff9b::1",
            "2002::1",
            "2001:1ff::1",
            "2001:db8::1",
            "3fff:fff::1",
            "fc00::1",
            "fe80::1",
        ] {
            assert!(!public_address(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "1.1.1.1",
            "100.63.255.255",
            "100.128.0.1",
            "8.64.0.1",
            "2001:200::1",
            "2606:4700:4700::1111",
            "2606:db8::1",
            "3fff:1000::1",
        ] {
            assert!(public_address(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_url(
            &Url::parse("https://example.com/article").unwrap()
        ));
        assert!(!public_url(&Url::parse("http://[::1]/").unwrap()));
    }

    #[tokio::test]
    async fn resolver_returns_only_public_addresses() {
        let public = "1.1.1.1".parse::<std::net::Ipv4Addr>().unwrap();
        let addresses: Vec<_> = PublicResolver
            .resolve("1.1.1.1".parse().unwrap())
            .await
            .unwrap()
            .collect();
        assert_eq!(addresses, [std::net::SocketAddr::from((public, 0))]);
        assert!(
            PublicResolver
                .resolve("127.0.0.1".parse().unwrap())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn fetch_rejects_a_non_public_bookmark_before_sending() {
        let fetcher = Fetcher::new().unwrap();
        std::assert_matches!(
            fetcher.fetch("http://127.0.0.1/").await,
            Err(PublishError::Parse(_))
        );
    }

    #[tokio::test]
    async fn reads_metadata_with_declared_charset_and_rejects_http_errors() {
        let page = http::Response::builder()
            .header("content-type", "text/html; charset=windows-1252")
            .body(b"<title>Caf\xe9</title>".to_vec())
            .unwrap();
        assert_eq!(
            read_metadata(page.into()).await.unwrap().title.as_deref(),
            Some("Café")
        );
        let forbidden = http::Response::builder()
            .status(403)
            .body("denied")
            .unwrap();
        assert!(read_metadata(forbidden.into()).await.is_err());
    }

    #[test]
    fn enforces_size_across_chunks() {
        // Bookmark HTML is capped at one MiB, including previous chunks.
        let limit = 1 << 20;
        let mut bytes = Vec::new();
        append_html(&mut bytes, &vec![0; limit - 1]).unwrap();
        append_html(&mut bytes, &[0]).unwrap();
        assert!(append_html(&mut bytes, &[0]).is_err());
        assert_eq!(bytes.len(), limit);
    }

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
