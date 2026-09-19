use async_trait::async_trait;
use domain::{
    ArticleIndexDocument, Category, CategoryArtifactDocument, HomeFragmentArtifactDocument,
    PageArtifactDocument, PageKey, SiteMetadataDocument, Slug,
};
use infra::{
    ArtifactReader, ArtifactSnapshot, DynArtifactReader, DynArtifactSnapshot, LocalArtifactReader,
    Result,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::SystemTime,
};
use tempfile::tempdir;
use topcoat::{
    asset::{AssetConfig, Manifest, ManifestEntry},
    router::{Body, HeaderMap, Method, Router, StatusCode, header, request::Request, to_bytes},
};

use server::{app::create_router as create_router_with_assets, assets};

#[tokio::test]
async fn home_selects_browser_language_with_english_default_and_saved_preference() {
    let router = create_router(fixture_reader(), false);
    for (language, cookie, expected) in [
        (None, None, "en"),
        (Some("ja-JP, en;q=0.8"), None, "ja"),
        (Some("ja;q=0.5, en-US;q=0.9"), None, "en"),
        (Some("JA-jp;q=1, en;q=0.8"), None, "ja"),
        (Some("fr-FR"), None, "en"),
        (Some("ja;q=0, en;q=0.5"), None, "en"),
        (Some("ja;q=bogus, en;q=0.5"), None, "en"),
        (Some("en"), Some("okawak_locale=ja"), "ja"),
        (Some("ja"), Some("unrelated=x; okawak_locale=en"), "en"),
        (Some("ja"), Some("okawak_locale=unknown"), "ja"),
    ] {
        let mut request = Request::builder().uri("/?from=entry%20link");
        if let Some(language) = language {
            request = request.header(header::ACCEPT_LANGUAGE, language);
        }
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let result = response(&router, request.body(Body::empty()).unwrap()).await;
        if expected == "en" {
            assert_eq!(
                result.status,
                StatusCode::TEMPORARY_REDIRECT,
                "{language:?} {cookie:?}"
            );
            assert_eq!(result.headers[header::LOCATION], "/en?from=entry%20link");
            assert_eq!(result.headers[header::CACHE_CONTROL], "no-store");
        } else {
            assert_eq!(result.status, StatusCode::OK, "{language:?} {cookie:?}");
            assert!(result.body.contains("<html lang=\"ja\">"));
            assert_eq!(result.headers[header::CACHE_CONTROL], "private, no-cache");
        }
        assert_eq!(result.headers[header::VARY], "Accept-Language, Cookie");
        assert!(!result.headers.contains_key(header::SET_COOKIE));
    }
}

#[tokio::test]
async fn manual_language_choice_is_saved_and_falls_back_to_the_published_home() {
    let router = create_router(validator_reader(fixture_reader()), true);
    for (path, language, location) in [
        (
            "/tech/e2e-article?lang=en&from=menu%20link",
            "en",
            "/en/tech/e2e-article?from=menu%20link",
        ),
        ("/daily?lang=en", "en", "/en"),
        ("/?lang=ja", "ja", "/"),
        ("/en/tech/e2e-article?lang=ja", "ja", "/tech/e2e-article"),
    ] {
        let result = response(
            &router,
            Request::builder()
                .uri(path)
                .header(header::ACCEPT_LANGUAGE, "en")
                .header(header::COOKIE, "okawak_locale=en")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(result.status, StatusCode::SEE_OTHER, "{path}");
        assert_eq!(result.headers[header::LOCATION], location);
        assert_eq!(
            result.headers[header::SET_COOKIE],
            format!("okawak_locale={language}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Lax")
        );
        assert_eq!(result.headers[header::CACHE_CONTROL], "no-store");
        assert!(!result.headers.contains_key(header::ETAG));
    }
}

#[tokio::test]
async fn language_selection_precedes_conditional_get_and_shares_the_snapshot() {
    let calls = Arc::new(AtomicUsize::new(0));
    let reader: DynArtifactReader = Arc::new(CountingReader {
        inner: fixture_reader(),
        snapshot_calls: calls.clone(),
    });
    let router = create_router(validator_reader(reader), true);
    let first = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for (method, language, expected) in [
        (Method::GET, "en", StatusCode::TEMPORARY_REDIRECT),
        (Method::HEAD, "en", StatusCode::TEMPORARY_REDIRECT),
        (Method::GET, "ja", StatusCode::NOT_MODIFIED),
    ] {
        calls.store(0, Ordering::SeqCst);
        let result = response(
            &router,
            Request::builder()
                .method(method)
                .uri("/")
                .header(header::ACCEPT_LANGUAGE, language)
                .header(header::IF_NONE_MATCH, &first.headers[header::ETAG])
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(result.status, expected);
        assert_eq!(result.headers[header::VARY], "Accept-Language, Cookie");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn explicit_page_urls_and_legacy_japanese_releases_keep_their_language() {
    let router = create_router(fixture_reader(), false);
    for (path, locale) in [("/en", "en"), ("/tech/e2e-article", "ja"), ("/about", "ja")] {
        let result = response(
            &router,
            Request::builder()
                .uri(path)
                .header(header::ACCEPT_LANGUAGE, "fr")
                .header(header::COOKIE, "okawak_locale=en")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(result.status, StatusCode::OK);
        assert!(result.body.contains(&format!("<html lang=\"{locale}\">")));
    }
    let legacy = create_router(empty_fixture_reader(), false);
    let result = response(
        &legacy,
        Request::builder().uri("/").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(result.status, StatusCode::OK);
    assert!(result.body.contains("<html lang=\"ja\">"));
}

#[tokio::test]
async fn language_negotiation_shares_a_snapshot_without_http_validators() {
    for (uri, language) in [("/", "ja"), ("/", "en"), ("/about?lang=en", "ja")] {
        let calls = Arc::new(AtomicUsize::new(0));
        let reader: DynArtifactReader = Arc::new(CountingReader {
            inner: fixture_reader(),
            snapshot_calls: calls.clone(),
        });
        let router = create_router(reader, false);
        let result = response(
            &router,
            Request::builder()
                .uri(uri)
                .header(header::ACCEPT_LANGUAGE, language)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert!(result.status.is_success() || result.status.is_redirection());
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{uri} {language}");
    }
}

#[tokio::test]
async fn language_choice_does_not_modify_api_or_post_responses() {
    let router = create_router(fixture_reader(), false);
    for (uri, method) in [
        ("/api/articles?lang=en", Method::GET),
        ("/api/health?lang=ja", Method::GET),
        ("/tech?lang=en", Method::POST),
    ] {
        let result = response(
            &router,
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert!(!result.headers.contains_key(header::SET_COOKIE));
        assert!(!result.headers.contains_key(header::LOCATION));
    }
}

struct TestResponse {
    status: StatusCode,
    headers: HeaderMap,
    content_type: Option<String>,
    body: String,
}

#[tokio::test]
async fn language_choices_preserve_the_existing_trailing_slash_redirect() {
    let router = create_router(validator_reader(fixture_reader()), true);
    for (path, canonical, destination) in [
        (
            "/about/?lang=en&from=nav%20link",
            "/about?lang=en&from=nav%20link",
            "/en/about?from=nav%20link",
        ),
        ("/en/about/?lang=ja", "/en/about?lang=ja", "/about"),
        ("/en/?lang=ja", "/en?lang=ja", "/"),
    ] {
        for method in [Method::GET, Method::HEAD] {
            let first = response(
                &router,
                Request::builder()
                    .method(method.clone())
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(first.status, StatusCode::PERMANENT_REDIRECT, "{path}");
            assert_eq!(first.headers[header::LOCATION], canonical);
            assert!(!first.headers.contains_key(header::SET_COOKIE));
            assert!(!first.headers.contains_key(header::ETAG));
            let selected = response(
                &router,
                Request::builder()
                    .method(method)
                    .uri(canonical)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(selected.status, StatusCode::SEE_OTHER);
            assert_eq!(selected.headers[header::LOCATION], destination);
            assert!(selected.headers.contains_key(header::SET_COOKIE));
        }
    }
}

fn fixture_reader() -> DynArtifactReader {
    Arc::new(LocalArtifactReader::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/site"),
    ))
}

fn empty_fixture_reader() -> DynArtifactReader {
    Arc::new(LocalArtifactReader::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/empty-site"),
    ))
}

fn test_asset_config() -> AssetConfig {
    AssetConfig::hosted_at(
        "/_topcoat/assets",
        Manifest {
            version: 1,
            assets: vec![
                ManifestEntry {
                    id: topcoat::runtime::SCRIPT.id(),
                    file: "topcoat-test.js".to_string(),
                    hash: "test".to_string(),
                    content_type: "text/javascript".to_string(),
                },
                ManifestEntry {
                    id: assets::STYLESHEET.id(),
                    file: "tailwind-test.css".to_string(),
                    hash: "test".to_string(),
                    content_type: "text/css".to_string(),
                },
                ManifestEntry {
                    id: assets::FAVICON.id(),
                    file: "favicon-test.ico".to_string(),
                    hash: "test".to_string(),
                    content_type: "image/x-icon".to_string(),
                },
            ],
        },
    )
}

fn create_router(artifact_reader: DynArtifactReader, validators_enabled: bool) -> Router {
    create_router_with_assets(artifact_reader, validators_enabled, test_asset_config())
}

fn assert_html_document(body: &str) {
    // Topcoat 0.8 hoists signal declarations before the component's HTML.
    // Comments before the doctype are valid and must not switch the browser to quirks mode.
    let mut body = body;
    while let Some(comment) = body.strip_prefix("<!--::topcoat::signal(") {
        body = comment.split_once("-->").expect("closed signal comment").1;
    }
    assert!(body.starts_with("<!DOCTYPE html>"));
}

#[tokio::test]
async fn trailing_slashes_redirect_to_canonical_routes_without_artifact_validators() {
    let router = create_router(validator_reader(fixture_reader()), true);
    for path in [
        "/about",
        "/tech",
        "/tech/e2e-article",
        "/api/articles",
        "/api/health",
    ] {
        for method in [Method::GET, Method::HEAD] {
            let response = response(
                &router,
                Request::builder()
                    .method(method.clone())
                    .uri(format!("{path}/?from=slash%20test"))
                    .header(header::IF_MODIFIED_SINCE, "Wed, 01 Jan 2098 00:00:00 GMT")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_eq!(
                response.status,
                StatusCode::PERMANENT_REDIRECT,
                "{method} {path}"
            );
            assert_eq!(
                response.headers[header::LOCATION],
                format!("{path}?from=slash%20test")
            );
            assert!(!response.headers.contains_key(header::ETAG));
            assert!(!response.headers.contains_key(header::LAST_MODIFIED));
        }
    }
}

#[derive(Clone)]
struct ValidatorReader {
    inner: DynArtifactReader,
}

#[derive(Clone)]
struct CountingReader {
    inner: DynArtifactReader,
    snapshot_calls: Arc<AtomicUsize>,
}

#[derive(Clone)]
struct FailingSnapshotReader;

#[async_trait]
impl ArtifactReader for FailingSnapshotReader {
    async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
        Err(infra::InfraError::Io(std::io::Error::other(
            "snapshot unavailable",
        )))
    }
}

#[async_trait]
impl ArtifactReader for CountingReader {
    async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
        self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.snapshot().await
    }
}

#[async_trait]
impl ArtifactReader for ValidatorReader {
    async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
        Ok(Arc::new(ValidatorSnapshot {
            inner: self.inner.snapshot().await?,
        }))
    }
}

struct ValidatorSnapshot {
    inner: DynArtifactSnapshot,
}

#[async_trait]
impl ArtifactSnapshot for ValidatorSnapshot {
    async fn localized(&self, locale: domain::Locale) -> Result<Option<DynArtifactSnapshot>> {
        self.inner.localized(locale).await
    }
    async fn read_locales(&self) -> Result<domain::SiteLocalesDocument> {
        self.inner.read_locales().await
    }
    async fn read_tag_labels(&self) -> Result<domain::TagLabels> {
        self.inner.read_tag_labels().await
    }
    async fn read_content_asset(&self, name: &domain::ContentAssetName) -> Result<Option<Vec<u8>>> {
        self.inner.read_content_asset(name).await
    }

    fn cache_identity(&self) -> Option<&str> {
        Some("release-1")
    }

    fn last_modified(&self) -> Option<SystemTime> {
        self.inner.last_modified().or(Some(SystemTime::UNIX_EPOCH))
    }

    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.inner.read_article_index().await
    }

    async fn read_category_document(
        &self,
        category: &Category,
    ) -> Result<CategoryArtifactDocument> {
        self.inner.read_category_document(category).await
    }

    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        self.inner.read_site_metadata().await
    }

    async fn read_article_html(&self, category: &Category, slug: &Slug) -> Result<String> {
        self.inner.read_article_html(category, slug).await
    }

    async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
        self.inner.read_home_fragment().await
    }

    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
        self.inner.read_page_document(page).await
    }
}

fn validator_reader(inner: DynArtifactReader) -> DynArtifactReader {
    Arc::new(ValidatorReader { inner })
}

async fn response(router: &Router, request: Request<Body>) -> TestResponse {
    let response = router.handle(request).await;
    let status = response.status();
    let headers = response.headers().clone();
    let content_type = response.headers().get("content-type").map(|value| {
        value
            .to_str()
            .expect("content type should be valid")
            .to_owned()
    });
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");

    TestResponse {
        status,
        headers,
        content_type,
        body: String::from_utf8(body.to_vec()).expect("response body should be UTF-8"),
    }
}

#[tokio::test]
async fn health_does_not_require_artifacts() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, "OK");
}

#[tokio::test]
async fn static_routes_take_precedence_over_dynamic_page_routes() {
    let router = create_router(fixture_reader(), false);

    let about = response(
        &router,
        Request::builder()
            .uri("/about")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;
    assert_eq!(about.status, StatusCode::OK);
    assert!(
        about
            .body
            .contains("<title>Fixture About | ぶくせんの探窟メモ</title>")
    );

    let health = response(
        &router,
        Request::builder()
            .uri("/api/health")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;
    assert_eq!(health.status, StatusCode::OK);
    assert_eq!(health.body, "OK");
}

#[tokio::test]
async fn page_routes_reject_unsupported_methods() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .method(Method::POST)
            .uri("/about")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn unmatched_paths_render_the_site_not_found_page() {
    let router = create_router(fixture_reader(), true);
    let response = response(
        &router,
        Request::builder()
            .uri("/unknown/nested/path?ignored=true")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(
        response.content_type.as_deref(),
        Some("text/html; charset=utf-8")
    );
    assert!(
        response
            .body
            .contains("<title>ページが見つかりません | ぶくせんの探窟メモ</title>")
    );
    assert!(
        response.body.contains(
            "<link rel=\"canonical\" href=\"https://www.okawak.net/unknown/nested/path\">"
        )
    );
    assert!(response.body.contains(
        "<meta property=\"og:url\" content=\"https://www.okawak.net/unknown/nested/path\">"
    ));
    assert!(response.body.contains("ページが見つかりませんでした。"));
    assert!(response.headers.get(header::ETAG).is_none());
    assert!(response.headers.get(header::LAST_MODIFIED).is_none());
}

#[tokio::test]
async fn unmatched_api_and_asset_paths_keep_plain_not_found_responses() {
    let router = create_router(fixture_reader(), true);

    for path in ["/api/unknown/extra", "/_topcoat/assets/unknown.js"] {
        let response = response(
            &router,
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request should be valid"),
        )
        .await;

        assert_eq!(response.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(
            response.content_type.as_deref(),
            Some("text/plain; charset=utf-8"),
            "{path}"
        );
        assert_eq!(response.body, "not found", "{path}");
    }
}

#[tokio::test]
async fn readiness_succeeds_when_site_metadata_is_readable() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/api/ready")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body, "READY");
}

#[tokio::test]
async fn readiness_fails_when_site_metadata_is_missing() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/api/ready")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.body, "NOT READY");
}

#[tokio::test]
async fn articles_returns_the_published_index_as_json() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/api/articles")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.content_type.as_deref(), Some("application/json"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&response.body)
            .expect("article index should be JSON"),
        serde_json::json!({
            "articles": [{
                "slug": "e2e-article",
                "title": "E2E Article",
                "category": "tech",
                "section_path": ["rust", "async"],
                "description": "Article fixture description",
                "tags": ["rust", "e2e"],
                "priority": 10,
                "created_at": "2026-01-01T00:00:00+09:00",
                "updated_at": "2026-01-02T00:00:00+09:00"
            }]
        })
    );
}

#[tokio::test]
async fn articles_returns_internal_server_error_when_index_is_missing() {
    let temp_dir = tempdir().expect("temp dir should be created");
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/api/articles")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn home_renders_the_published_summary_as_html() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.content_type.as_deref(),
        Some("text/html; charset=utf-8")
    );
    assert_html_document(&response.body);
    assert!(response.body.contains("<title>ぶくせんの探窟メモ</title>"));
    assert!(response.body.contains(
        "<meta name=\"description\" content=\"1カテゴリで1件の記事を公開しています。\">"
    ));
    assert!(
        response
            .body
            .contains("<link rel=\"canonical\" href=\"https://www.okawak.net\">")
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:title\" content=\"ぶくせんの探窟メモ\">")
    );
    assert!(response.body.contains(
        "<meta property=\"og:description\" content=\"1カテゴリで1件の記事を公開しています。\">"
    ));
    assert!(
        response
            .body
            .contains("<meta property=\"og:url\" content=\"https://www.okawak.net\">")
    );
    assert!(response.body.contains("<p>Fixture home content</p>"));
    assert!(
        response
            .body
            .contains("href=\"/?lang=ja\" aria-current=\"page\"")
    );
    assert!(response.body.contains("href=\"/tech\""));
    assert!(response.body.contains("href=\"/tech/e2e-article\""));
    assert!(response.body.contains(">E2E Article</h3>"));
    assert!(response.body.contains("Article fixture description"));
    assert!(response.body.contains("#rust"));
    assert!(response.body.contains("2026年1月1日"));
    assert!(response.body.contains("2026年1月2日"));
    assert!(!response.body.contains("&lt;p&gt;Fixture home content"));
}

#[tokio::test]
async fn navigation_marks_only_the_current_destination_including_query_urls() {
    let router = create_router(fixture_reader(), false);
    for (path, expected) in [
        ("/?utm_source=test", Some("/")),
        ("/about?utm_source=test", Some("/about")),
        ("/tech", None),
        ("/tech/e2e-article.html", None),
        ("/unknown/nested/path?utm_source=test", None),
    ] {
        let response = response(
            &router,
            Request::builder()
                .uri(path)
                .header(header::ACCEPT_LANGUAGE, "ja")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        for destination in ["/", "/about"] {
            let href = if destination == "/" {
                "/?lang=ja"
            } else {
                destination
            };
            assert_eq!(
                response
                    .body
                    .contains(&format!("href=\"{href}\" aria-current=\"page\"")),
                expected == Some(destination),
                "{path}: current destination {destination}",
            );
        }
    }
}

#[tokio::test]
async fn error_pages_keep_navigation_tied_to_the_requested_url() {
    let router = create_router(Arc::new(FailingSnapshotReader), false);
    for path in ["/", "/about"] {
        let href = if path == "/" { "/?lang=ja" } else { path };
        let response = response(
            &router,
            Request::builder()
                .uri(format!("{path}?utm_source=test"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            response
                .body
                .contains(&format!("href=\"{href}\" aria-current=\"page\""))
        );
        assert_eq!(response.body.matches("aria-current=\"page\"").count(), 1);
    }
}

#[tokio::test]
async fn home_shell_exposes_topcoat_mobile_navigation_contract() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(
        response
            .body
            .contains("<script type=\"module\" src=\"/_topcoat/assets/topcoat-test.js\"></script>")
    );
    assert!(!response.body.contains("navigation-test.js"));
    assert!(!response.body.contains("okawak-shell-version"));
    assert!(response.body.contains("aria-controls=\"site-header-nav\""));
    assert!(response.body.contains("aria-expanded=\"false\""));
    assert!(
        response
            .body
            .contains("aria-label=\"ナビゲーションメニューを開く\"")
    );
    assert!(response.body.contains("data-topcoat-on:click="));
    assert!(response.body.contains("data-topcoat-bind:aria-expanded="));
    assert!(response.body.contains("data-topcoat-bind:aria-label="));
    assert!(response.body.contains("<nav id=\"site-header-nav\""));
    assert!(response.body.contains("data-topcoat-bind:class="));
    assert!(response.body.contains("class=\"hidden absolute inset-x-4"));
    assert!(
        response
            .body
            .contains("aria-label=\"okawakのGitHubプロフィールを開く\"")
    );
    assert!(!response.body.contains("font-awesome"));
    assert!(!response.body.contains("fa-github"));
    assert!(response.body.contains("<svg"));
    assert!(response.body.contains("Noto+Sans+JP:wght@400..700"));
}

#[tokio::test]
async fn home_renders_empty_state_without_treating_it_as_an_error() {
    let router = create_router(empty_fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains(
        "<meta name=\"description\" content=\"0カテゴリで0件の記事を公開しています。\">"
    ));
    assert!(response.body.contains("記事がありません"));
    assert!(!response.body.contains("記事の読み込みに失敗しました"));
}

#[tokio::test]
async fn home_uses_fallback_copy_when_optional_fragment_is_missing() {
    let temp_dir = tempdir().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("articles")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("metadata")).unwrap();
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/site");
    std::fs::copy(
        fixture_root.join("articles/index.json"),
        temp_dir.path().join("articles/index.json"),
    )
    .unwrap();
    std::fs::copy(
        fixture_root.join("metadata/site.json"),
        temp_dir.path().join("metadata/site.json"),
    )
    .unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);

    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(
        response
            .body
            .contains("最近の記事とカテゴリをまとめています。")
    );
}

#[tokio::test]
async fn home_returns_internal_server_error_for_invalid_optional_fragment() {
    let temp_dir = tempdir().unwrap();
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/site");
    for relative_path in ["articles/index.json", "metadata/site.json"] {
        let destination = temp_dir.path().join(relative_path);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(fixture_root.join(relative_path), destination).unwrap();
    }
    std::fs::write(temp_dir.path().join("home.json"), "invalid JSON").unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body.contains("記事の読み込みに失敗しました"));
}

#[tokio::test]
async fn home_returns_internal_server_error_when_required_artifact_is_missing() {
    let temp_dir = tempdir().unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body.contains("記事の読み込みに失敗しました"));
}

#[tokio::test]
async fn home_returns_internal_server_error_page_when_snapshot_fails() {
    let router = create_router(Arc::new(FailingSnapshotReader), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body.contains("記事の読み込みに失敗しました"));
}

#[tokio::test]
async fn home_supports_release_aware_conditional_get() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let first = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let etag = first
        .headers
        .get(header::ETAG)
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_string();

    let cached = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .header(header::IF_NONE_MATCH, etag)
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());
}

#[tokio::test]
async fn conditional_get_and_home_share_one_snapshot() {
    let snapshot_calls = Arc::new(AtomicUsize::new(0));
    let reader = Arc::new(CountingReader {
        inner: fixture_reader(),
        snapshot_calls: snapshot_calls.clone(),
    });
    let router = create_router(validator_reader(reader), true);

    let response = response(
        &router,
        Request::builder()
            .uri("/")
            .header(header::ACCEPT_LANGUAGE, "ja")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn category_renders_the_published_landing_and_articles_as_html() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder().uri("/tech").body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.content_type.as_deref(),
        Some("text/html; charset=utf-8")
    );
    assert_html_document(&response.body);
    assert!(
        response
            .body
            .contains("<title>Fixture Tech | ぶくせんの探窟メモ</title>")
    );
    assert!(
        response
            .body
            .contains("<meta name=\"description\" content=\"Category fixture description\">")
    );
    assert!(
        response
            .body
            .contains("<link rel=\"canonical\" href=\"https://www.okawak.net/tech\">")
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:title\" content=\"Fixture Tech | ぶくせんの探窟メモ\">")
    );
    assert!(
        response.body.contains(
            "<meta property=\"og:description\" content=\"Category fixture description\">"
        )
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:url\" content=\"https://www.okawak.net/tech\">")
    );
    assert!(response.body.contains("<h2>Tech landing</h2>"));
    assert!(response.body.contains("rust / async"));
    assert!(response.body.contains("href=\"/tech/e2e-article\""));
    assert!(response.body.contains(">E2E Article</h3>"));
    assert!(response.body.contains("Article fixture description"));
    assert!(response.body.contains("#rust"));
    assert!(response.body.contains("2026年1月1日"));
    assert!(response.body.contains("2026年1月2日"));
    assert!(!response.body.contains("&lt;h2&gt;Tech landing"));
}

#[tokio::test]
async fn category_returns_not_found_for_invalid_or_missing_categories() {
    let router = create_router(fixture_reader(), false);

    for path in ["/unknown", "/daily", "/not%20a%20category"] {
        let response = response(
            &router,
            Request::builder().uri(path).body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::NOT_FOUND, "{path}");
        assert!(response.body.contains("ページが見つかりませんでした。"));
        assert!(response.body.contains(&format!(
            "<link rel=\"canonical\" href=\"https://www.okawak.net{path}\">"
        )));
    }
}

#[tokio::test]
async fn category_returns_internal_server_error_for_invalid_artifact() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/physics")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body.contains("カテゴリの読み込みに失敗しました"));
    assert!(
        response
            .body
            .contains("<link rel=\"canonical\" href=\"https://www.okawak.net/physics\">")
    );
}

#[tokio::test]
async fn category_returns_internal_server_error_page_when_snapshot_fails() {
    let router = create_router(Arc::new(FailingSnapshotReader), false);
    let response = response(
        &router,
        Request::builder().uri("/tech").body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body.contains("カテゴリの読み込みに失敗しました"));
}

#[tokio::test]
async fn category_supports_release_aware_conditional_get() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let first = response(
        &router,
        Request::builder().uri("/tech").body(Body::empty()).unwrap(),
    )
    .await;
    let etag = first
        .headers
        .get(header::ETAG)
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_string();

    let cached = response(
        &router,
        Request::builder()
            .uri("/tech")
            .header(header::IF_NONE_MATCH, etag)
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());
}

#[tokio::test]
async fn conditional_get_and_category_share_one_snapshot() {
    let snapshot_calls = Arc::new(AtomicUsize::new(0));
    let reader = Arc::new(CountingReader {
        inner: fixture_reader(),
        snapshot_calls: snapshot_calls.clone(),
    });
    let router = create_router(validator_reader(reader), true);

    let response = response(
        &router,
        Request::builder().uri("/tech").body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn category_and_inline_shard_share_one_snapshot_without_validators() {
    let snapshot_calls = Arc::new(AtomicUsize::new(0));
    let router = create_router(
        Arc::new(CountingReader {
            inner: fixture_reader(),
            snapshot_calls: snapshot_calls.clone(),
        }),
        false,
    );
    let response = response(
        &router,
        Request::builder().uri("/tech").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
    assert!(response.body.contains("記事を絞り込む"));
    assert!(response.body.contains("E2E Article"));
}

#[tokio::test]
async fn category_shard_validates_browser_arguments_and_keeps_errors_out_of_the_site_shell() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let page = response(
        &router,
        Request::builder().uri("/tech").body(Body::empty()).unwrap(),
    )
    .await;
    let (_, marker) = page
        .body
        .split_once("::topcoat::shard::start(")
        .expect("shard marker");
    let mut arguments = marker.split('"');
    let shard = arguments.nth(1).expect("shard id");
    let identity = arguments.nth(1).expect("invocation identity");
    for (category, expected) in [
        ("tech", StatusCode::OK),
        ("../../private", StatusCode::BAD_REQUEST),
        ("daily", StatusCode::NOT_FOUND),
        ("physics", StatusCode::INTERNAL_SERVER_ERROR),
    ] {
        let response = response(
            &router,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/_topcoat/runtime/shards/{shard}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(topcoat::router::request::IDENTITY_HEADER, identity)
                .body(Body::from(
                    serde_json::json!({ "args": [category, "ja"], "signals": {} }).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status, expected, "{category}");
        assert!(!response.headers.contains_key(header::ETAG));
        assert!(!response.headers.contains_key(header::LAST_MODIFIED));
        assert!(!response.body.contains("<!DOCTYPE html>"));
    }
}

#[tokio::test]
async fn page_rerun_does_not_reuse_the_unfiltered_get_validator_after_rewrite() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let page = response(
        &router,
        Request::builder().uri("/tech").body(Body::empty()).unwrap(),
    )
    .await;
    let rerun = response(
        &router,
        Request::builder()
            .method(Method::POST)
            .uri("/_topcoat/runtime/pages/tech")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::IF_NONE_MATCH, &page.headers[header::ETAG])
            .body(Body::from(r#"{"signals":{}}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(rerun.status, StatusCode::OK);
    assert!(!rerun.headers.contains_key(header::ETAG));
    assert!(!rerun.headers.contains_key(header::LAST_MODIFIED));
    assert!(!rerun.headers.contains_key(header::CACHE_CONTROL));
    assert!(rerun.body.contains("E2E Article"));
}

#[tokio::test]
async fn article_renders_the_published_document_as_html() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.content_type.as_deref(),
        Some("text/html; charset=utf-8")
    );
    assert_html_document(&response.body);
    assert!(
        response
            .body
            .contains("<title>E2E Article | ぶくせんの探窟メモ</title>")
    );
    assert!(
        response
            .body
            .contains("<meta name=\"description\" content=\"Article fixture description\">")
    );
    assert!(
        response
            .body
            .contains("<link rel=\"canonical\" href=\"https://www.okawak.net/tech/e2e-article\">")
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:title\" content=\"E2E Article | ぶくせんの探窟メモ\">")
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:description\" content=\"Article fixture description\">")
    );
    assert!(response.body.contains(
        "<meta property=\"og:url\" content=\"https://www.okawak.net/tech/e2e-article\">"
    ));
    assert!(
        response
            .body
            .contains("<meta property=\"og:type\" content=\"article\">")
    );
    assert!(response.body.contains(">技術</p>"));
    assert!(response.body.contains(">E2E Article</h1>"));
    assert!(response.body.contains("Article fixture description"));
    assert!(response.body.contains("#rust"));
    assert!(response.body.contains("2026年1月1日"));
    assert!(response.body.contains("2026年1月2日"));
    assert!(response.body.contains(
        "<h1>Article artifact</h1><p>Article fixture body with <code>inline_code()</code>."
    ));
    assert!(response.body.contains("data-testid=\"article-wide-code\""));
    assert!(!response.body.contains("&lt;h1&gt;Article artifact"));
}

#[tokio::test]
async fn article_accepts_html_suffix_and_uses_the_normalized_canonical_url() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article.html")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(
        response
            .body
            .contains("<link rel=\"canonical\" href=\"https://www.okawak.net/tech/e2e-article\">")
    );
    assert!(!response.body.contains(
        "<link rel=\"canonical\" href=\"https://www.okawak.net/tech/e2e-article.html\">"
    ));
}

#[tokio::test]
async fn article_returns_not_found_for_invalid_or_missing_documents() {
    let router = create_router(fixture_reader(), false);

    for path in [
        "/unknown/e2e-article",
        "/tech/not%20a%20slug",
        "/tech/missing-article",
    ] {
        let response = response(
            &router,
            Request::builder().uri(path).body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::NOT_FOUND, "{path}");
        assert!(response.body.contains("ページが見つかりませんでした。"));
        assert!(response.body.contains(&format!(
            "<link rel=\"canonical\" href=\"https://www.okawak.net{path}\">"
        )));
    }

    let temp_dir = tempdir().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("articles")).unwrap();
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/site");
    std::fs::copy(
        fixture_root.join("articles/index.json"),
        temp_dir.path().join("articles/index.json"),
    )
    .unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert!(response.body.contains("ページが見つかりませんでした。"));
}

#[tokio::test]
async fn article_returns_internal_server_error_for_invalid_artifacts() {
    let temp_dir = tempdir().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("articles/tech")).unwrap();
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/site");
    std::fs::copy(
        fixture_root.join("articles/index.json"),
        temp_dir.path().join("articles/index.json"),
    )
    .unwrap();
    std::fs::write(
        temp_dir.path().join("articles/tech/e2e-article.html"),
        "   ",
    )
    .unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let blank_html_response = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(
        blank_html_response.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        blank_html_response
            .body
            .contains("記事の読み込みに失敗しました")
    );
    assert!(
        blank_html_response
            .body
            .contains("<meta property=\"og:type\" content=\"article\">")
    );

    let empty_dir = tempdir().unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(empty_dir.path())), false);
    let missing_index_response = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(
        missing_index_response.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        missing_index_response
            .body
            .contains("記事の読み込みに失敗しました")
    );
}

#[tokio::test]
async fn article_returns_internal_server_error_page_when_snapshot_fails() {
    let router = create_router(Arc::new(FailingSnapshotReader), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body.contains("記事の読み込みに失敗しました"));
}

#[tokio::test]
async fn article_supports_release_aware_conditional_get() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let first = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let etag = first
        .headers
        .get(header::ETAG)
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_string();

    let cached = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .header(header::IF_NONE_MATCH, etag)
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());
}

#[tokio::test]
async fn conditional_get_and_article_share_one_snapshot() {
    let snapshot_calls = Arc::new(AtomicUsize::new(0));
    let reader = Arc::new(CountingReader {
        inner: fixture_reader(),
        snapshot_calls: snapshot_calls.clone(),
    });
    let router = create_router(validator_reader(reader), true);

    let response = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn about_renders_the_published_page_as_html() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/about")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        response.content_type.as_deref(),
        Some("text/html; charset=utf-8")
    );
    assert_html_document(&response.body);
    assert!(response.body.contains("<html lang=\"ja\">"));
    assert!(
        response
            .body
            .contains("<title>Fixture About | ぶくせんの探窟メモ</title>")
    );
    assert!(
        response
            .body
            .contains("<meta name=\"description\" content=\"About fixture description\">")
    );
    assert!(
        response
            .body
            .contains("<link rel=\"canonical\" href=\"https://www.okawak.net/about\">")
    );
    assert!(
        response.body.contains(
            "<meta property=\"og:title\" content=\"Fixture About | ぶくせんの探窟メモ\">"
        )
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:description\" content=\"About fixture description\">")
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:url\" content=\"https://www.okawak.net/about\">")
    );
    assert!(
        response
            .body
            .contains("<meta property=\"og:type\" content=\"website\">")
    );
    assert!(
        response
            .body
            .contains("<link rel=\"stylesheet\" href=\"/_topcoat/assets/tailwind-test.css\">")
    );
    assert!(response.body.contains(
        "<link rel=\"icon\" href=\"/_topcoat/assets/favicon-test.ico\" type=\"image/x-icon\""
    ));
    assert!(response.body.contains(">Fixture About</h1>"));
    assert!(
        response
            .body
            .contains("<h1>About artifact</h1><p>About fixture body</p>")
    );
    assert!(!response.body.contains("&lt;h1&gt;About artifact"));
}

#[tokio::test]
async fn about_shell_initializes_content_enhancements() {
    let router = create_router(fixture_reader(), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/about")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert!(response.body.contains("katex@0.16.22/dist/katex.min.css"));
    assert!(response.body.contains("katex@0.16.22/dist/katex.min.js"));
    assert!(response.body.contains("window.okawakRenderMath"));
    assert!(response.body.contains("window.okawakScheduleMathRender"));
    assert!(
        response
            .body
            .contains("if (window.katex && window.okawakRenderMath)")
    );
    assert!(
        response
            .body
            .contains("highlight.js/11.11.1/styles/github-dark.min.css")
    );
    assert!(
        response
            .body
            .contains("highlight.js/11.11.1/highlight.min.js")
    );
    assert!(response.body.contains("window.okawakHighlightCode"));
    assert!(response.body.contains("window.okawakScheduleCodeHighlight"));
    assert!(!response.body.contains("window.katex &amp;&amp;"));
}

#[tokio::test]
async fn about_returns_not_found_page_when_artifact_is_missing() {
    let temp_dir = tempdir().unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/about")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert!(
        response
            .body
            .contains("<title>ページが見つかりません | ぶくせんの探窟メモ</title>")
    );
    assert!(response.body.contains("ページが見つかりませんでした。"));
}

#[tokio::test]
async fn about_returns_internal_server_error_page_for_invalid_artifact() {
    let temp_dir = tempdir().unwrap();
    std::fs::create_dir_all(temp_dir.path().join("pages")).unwrap();
    std::fs::write(temp_dir.path().join("pages/about.json"), "not json").unwrap();
    let router = create_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
    let response = response(
        &router,
        Request::builder()
            .uri("/about")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body.contains("ページの読み込みに失敗しました"));
}

#[tokio::test]
async fn about_supports_release_aware_conditional_get() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let first = response(
        &router,
        Request::builder()
            .uri("/about")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let etag = first
        .headers
        .get(header::ETAG)
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_string();

    let cached = response(
        &router,
        Request::builder()
            .uri("/about")
            .header(header::IF_NONE_MATCH, etag)
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());
}

#[tokio::test]
async fn conditional_get_and_about_share_one_snapshot() {
    let snapshot_calls = Arc::new(AtomicUsize::new(0));
    let reader = Arc::new(CountingReader {
        inner: fixture_reader(),
        snapshot_calls: snapshot_calls.clone(),
    });
    let router = create_router(validator_reader(reader), true);

    let response = response(
        &router,
        Request::builder()
            .uri("/about")
            .body(Body::empty())
            .unwrap(),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn articles_supports_release_aware_conditional_get() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let first = response(
        &router,
        Request::builder()
            .uri("/api/articles")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;
    let etag = first
        .headers
        .get(header::ETAG)
        .expect("successful artifact response should have an ETag")
        .clone();
    let last_modified = first
        .headers
        .get(header::LAST_MODIFIED)
        .expect("successful artifact response should have Last-Modified")
        .clone();

    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(
        first.headers.get(header::CACHE_CONTROL).unwrap(),
        "public, max-age=0, must-revalidate"
    );
    assert!(first.headers.contains_key(header::LAST_MODIFIED));

    let second = response(
        &router,
        Request::builder()
            .uri("/api/articles")
            .header(header::IF_NONE_MATCH, etag)
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(second.status, StatusCode::NOT_MODIFIED);
    assert!(second.body.is_empty());
    assert!(second.headers.contains_key(header::ETAG));
    assert!(second.headers.contains_key(header::LAST_MODIFIED));

    let third = response(
        &router,
        Request::builder()
            .uri("/api/articles")
            .header(header::IF_MODIFIED_SINCE, &last_modified)
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(third.status, StatusCode::NOT_MODIFIED);
    assert!(third.body.is_empty());

    let etag_precedence = response(
        &router,
        Request::builder()
            .uri("/api/articles")
            .header(header::IF_NONE_MATCH, "\"different\"")
            .header(header::IF_MODIFIED_SINCE, last_modified)
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(etag_precedence.status, StatusCode::OK);
    assert!(!etag_precedence.body.is_empty());

    let head = response(
        &router,
        Request::builder()
            .method("HEAD")
            .uri("/api/articles")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(head.status, StatusCode::OK);
    assert!(head.headers.contains_key(header::ETAG));
}

#[tokio::test]
async fn articles_omits_validators_when_disabled_or_unsuccessful() {
    let disabled = create_router(validator_reader(fixture_reader()), false);
    let disabled_response = response(
        &disabled,
        Request::builder()
            .uri("/api/articles")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(disabled_response.status, StatusCode::OK);
    assert!(!disabled_response.headers.contains_key(header::ETAG));
    assert!(
        !disabled_response
            .headers
            .contains_key(header::CACHE_CONTROL)
    );

    let temp_dir = tempdir().expect("temp dir should be created");
    let missing_reader: DynArtifactReader = Arc::new(LocalArtifactReader::new(temp_dir.path()));
    let enabled = create_router(validator_reader(missing_reader), true);
    let error_response = response(
        &enabled,
        Request::builder()
            .uri("/api/articles")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(error_response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!error_response.headers.contains_key(header::ETAG));
    assert!(!error_response.headers.contains_key(header::CACHE_CONTROL));
}

#[tokio::test]
async fn conditional_get_and_articles_share_one_snapshot() {
    let snapshot_calls = Arc::new(AtomicUsize::new(0));
    let reader: DynArtifactReader = Arc::new(CountingReader {
        inner: fixture_reader(),
        snapshot_calls: Arc::clone(&snapshot_calls),
    });
    let router = create_router(validator_reader(reader), true);

    let response = response(
        &router,
        Request::builder()
            .uri("/api/articles")
            .body(Body::empty())
            .expect("request should be valid"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn english_routes_render_matching_metadata_links_and_release_validators() {
    let router = create_router(validator_reader(fixture_reader()), true);
    for path in ["/en", "/en/about", "/en/tech", "/en/tech/e2e-article"] {
        let response = response(
            &router,
            Request::builder().uri(path).body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(response.status, StatusCode::OK, "{path}");
        assert!(response.body.contains("<html lang=\"en\">"), "{path}");
        assert!(
            response.body.contains(&format!(
                "<link rel=\"canonical\" href=\"https://www.okawak.net{path}\">"
            )),
            "{path}"
        );
        assert!(response.body.contains("hreflang=\"ja\""));
        assert!(response.body.contains("hreflang=\"en\""));
        assert!(response.body.contains("content=\"en_US\""));
        assert!(response.body.contains("Main navigation"));
        assert!(!response.body.contains("記事を絞り込む"));
        assert!(response.headers.contains_key(header::ETAG));
    }
    let ja = response(
        &router,
        Request::builder()
            .uri("/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let en = response(
        &router,
        Request::builder()
            .uri("/en/tech/e2e-article")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_ne!(ja.headers[header::ETAG], en.headers[header::ETAG]);
    assert!(en.body.contains("#Rust programming"));
    assert!(en.body.contains("Translated E2E Article"));
    let conditional = response(
        &router,
        Request::builder()
            .uri("/en/tech/e2e-article")
            .header(header::IF_NONE_MATCH, &en.headers[header::ETAG])
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(conditional.status, StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn untranslated_english_pages_return_english_404_without_fake_alternates() {
    let router = create_router(fixture_reader(), false);
    let missing = response(
        &router,
        Request::builder()
            .uri("/en/daily")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.body.contains("<html lang=\"en\">"));
    assert!(missing.body.contains("Page not found."));
    assert!(!missing.body.contains("rel=\"alternate\""));
    let legacy = create_router(empty_fixture_reader(), false);
    let missing = response(
        &legacy,
        Request::builder().uri("/en").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.body.contains("<html lang=\"en\">"));
}

#[tokio::test]
async fn english_shard_keeps_labels_and_rejects_unsupported_locale() {
    let router = create_router(fixture_reader(), false);
    let page = response(
        &router,
        Request::builder()
            .uri("/en/tech")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let (_, marker) = page.body.split_once("::topcoat::shard::start(").unwrap();
    let mut arguments = marker.split('"');
    let shard = arguments.nth(1).unwrap();
    let identity = arguments.nth(1).unwrap();
    for (locale, expected) in [("en", StatusCode::OK), ("fr", StatusCode::BAD_REQUEST)] {
        let result = response(
            &router,
            Request::builder()
                .method(Method::POST)
                .uri(format!("/_topcoat/runtime/shards/{shard}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(topcoat::router::request::IDENTITY_HEADER, identity)
                .body(Body::from(
                    serde_json::json!({"args":["tech",locale],"signals":{}}).to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(result.status, expected);
        if expected == StatusCode::OK {
            assert!(result.body.contains("Filter articles"));
            assert!(result.body.contains("#Browser testing"));
            assert!(result.body.contains("1 article"));
            assert!(!result.body.contains("記事を絞り込む"));
        }
    }
}

#[tokio::test]
async fn content_images_are_shared_across_locales_and_do_not_use_page_validators() {
    let router = create_router(validator_reader(fixture_reader()), true);
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../e2e/fixtures/site/content-assets");
    let path = std::fs::read_dir(root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let res = router
        .handle(
            Request::builder()
                .uri(format!(
                    "/content-assets/{}",
                    path.file_name().unwrap().to_string_lossy()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "image/png");
    assert_eq!(
        res.headers()[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    assert_eq!(bytes.as_ref(), std::fs::read(path).unwrap());
}

#[tokio::test]
async fn missing_content_assets_do_not_render_the_site_error_page() {
    let router = create_router(validator_reader(fixture_reader()), true);
    for path in [
        format!("/content-assets/{}.png", "0".repeat(64)),
        "/content-assets/not-a-hash.png".into(),
        "/content-assets".into(),
        "/content-assets/nested/missing.png".into(),
    ] {
        let result = response(
            &router,
            Request::builder().uri(&path).body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(result.status, StatusCode::NOT_FOUND, "{path}");
        assert!(
            !result
                .content_type
                .as_deref()
                .unwrap_or_default()
                .starts_with("text/html"),
            "{path}"
        );
        assert!(!result.body.contains("<!DOCTYPE html>"), "{path}");
        assert!(result.headers.get(header::ETAG).is_none());
    }
    let page = response(
        &router,
        Request::builder()
            .uri("/content-assets-missing")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(page.status, StatusCode::NOT_FOUND);
    assert!(
        page.content_type
            .as_deref()
            .unwrap_or_default()
            .starts_with("text/html")
    );
}
