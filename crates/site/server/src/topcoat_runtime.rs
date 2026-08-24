//! Topcoat runtime router and HTTP integration.

use std::sync::Arc;

use infra::{DynArtifactReader, DynArtifactSnapshot};
use topcoat::{
    Result,
    asset::{AssetConfig, RouterBuilderAssetExt},
    context::{Cx, app_context, try_request_context},
    router::{
        Body, LayerFn, LayerFuture, Next, Path, Router, StatusCode, content::Json,
        error::internal_server_error, request, response::Response, route,
    },
};

use crate::{
    article_index::{read_article_index, read_article_index_from_snapshot},
    http_cache::{ArtifactConditionalGetDecision, ArtifactHttpCacheState},
    page_loader::ArtifactPageLoader,
    readiness::check_artifact_readiness,
};
use web::{PageLoaderContext, topcoat_pages};

#[derive(Clone)]
struct ArtifactReaderContext(DynArtifactReader);

#[route(GET "/api/health")]
async fn health() -> Result<&'static str> {
    Ok("OK")
}

#[route(GET "/api/ready")]
async fn readiness(cx: &Cx) -> Result<(StatusCode, &'static str)> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;

    match check_artifact_readiness(artifact_reader).await {
        Ok(()) => Ok((StatusCode::OK, "READY")),
        Err(error) => {
            eprintln!("Artifact readiness check failed: {error}");
            Ok((StatusCode::SERVICE_UNAVAILABLE, "NOT READY"))
        }
    }
}

#[route(GET "/api/articles")]
async fn articles(cx: &Cx) -> Result<Json<domain::ArticleIndexDocument>> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;
    let document = match try_request_context::<DynArtifactSnapshot>(cx) {
        Some(snapshot) => read_article_index_from_snapshot(snapshot).await,
        None => read_article_index(artifact_reader).await,
    }
    .map_err(internal_server_error)?;
    Ok(Json(document))
}

fn not_modified_response(conditional_get: &ArtifactConditionalGetDecision) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    conditional_get.insert_headers(response.headers_mut());
    response
}

fn artifact_conditional_get<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
    Box::pin(async move {
        let state = app_context::<ArtifactHttpCacheState>(cx);
        let Some(conditional_get) = state
            .conditional_get(request::method(cx), request::uri(cx), request::headers(cx))
            .await
        else {
            return next.run(cx, body).await;
        };

        if conditional_get.should_short_circuit() {
            return Ok(not_modified_response(&conditional_get));
        }

        let mut response = match conditional_get.snapshot() {
            Some(snapshot) => {
                let page_loader = PageLoaderContext(Arc::new(ArtifactPageLoader::from_snapshot(
                    snapshot.clone(),
                )));
                let cx = cx.with(snapshot).with(page_loader);
                next.run(&cx, body).await?
            }
            None => next.run(cx, body).await?,
        };
        if conditional_get.should_return_not_modified_after_response(response.status()) {
            return Ok(not_modified_response(&conditional_get));
        }
        if conditional_get.should_attach_validators(response.status()) {
            conditional_get.insert_headers(response.headers_mut());
        }
        Ok(response)
    })
}

pub fn create_topcoat_router(
    artifact_reader: DynArtifactReader,
    validators_enabled: bool,
    assets: AssetConfig,
) -> Router {
    Router::builder()
        .route(health)
        .route(readiness)
        .route(articles)
        .route(topcoat_pages::home)
        .route(topcoat_pages::article_page)
        .route(topcoat_pages::category_page)
        .route(topcoat_pages::about)
        // The framework-neutral decision filters APIs, static assets, and unsuccessful responses.
        // One global layer also avoids nested prefix layers acquiring more than one snapshot.
        .layer(LayerFn::new(None::<&Path>, artifact_conditional_get))
        .app_context(ArtifactHttpCacheState::new(
            artifact_reader.clone(),
            validators_enabled,
        ))
        .app_context(PageLoaderContext(Arc::new(
            ArtifactPageLoader::from_reader(artifact_reader.clone()),
        )))
        .app_context(ArtifactReaderContext(artifact_reader))
        .assets(assets)
        .build()
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::SystemTime,
    };
    use web::topcoat_pages;

    use async_trait::async_trait;
    use domain::{
        ArticleIndexDocument, Category, CategoryArtifactDocument, HomeFragmentArtifactDocument,
        PageArtifactDocument, PageKey, SiteMetadataDocument, Slug,
    };
    use infra::{
        ArtifactReader, ArtifactSnapshot, DynArtifactReader, DynArtifactSnapshot,
        LocalArtifactReader, Result,
    };
    use tempfile::tempdir;
    use topcoat::{
        asset::{AssetConfig, Manifest, ManifestEntry},
        router::{Body, HeaderMap, Router, StatusCode, header, request::Request, to_bytes},
    };

    use super::create_topcoat_router as create_topcoat_router_with_assets;

    struct TestResponse {
        status: StatusCode,
        headers: HeaderMap,
        content_type: Option<String>,
        body: String,
    }

    fn fixture_reader() -> DynArtifactReader {
        Arc::new(LocalArtifactReader::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../e2e/fixtures/site"),
        ))
    }

    fn empty_fixture_reader() -> DynArtifactReader {
        Arc::new(LocalArtifactReader::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../e2e/fixtures/empty-site"),
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
                        id: topcoat_pages::CLIENT_NAVIGATION_SCRIPT.id(),
                        file: "topcoat-navigation-test.js".to_string(),
                        hash: "test".to_string(),
                        content_type: "text/javascript".to_string(),
                    },
                    ManifestEntry {
                        id: topcoat_pages::STYLESHEET.id(),
                        file: "tailwind-test.css".to_string(),
                        hash: "test".to_string(),
                        content_type: "text/css".to_string(),
                    },
                    ManifestEntry {
                        id: topcoat_pages::FAVICON.id(),
                        file: "favicon-test.ico".to_string(),
                        hash: "test".to_string(),
                        content_type: "image/x-icon".to_string(),
                    },
                ],
            },
        )
    }

    fn create_topcoat_router(
        artifact_reader: DynArtifactReader,
        validators_enabled: bool,
    ) -> Router {
        create_topcoat_router_with_assets(artifact_reader, validators_enabled, test_asset_config())
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
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
    async fn readiness_succeeds_when_site_metadata_is_readable() {
        let router = create_topcoat_router(fixture_reader(), false);
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
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
        let router = create_topcoat_router(fixture_reader(), false);
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
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
        let router = create_topcoat_router(fixture_reader(), false);
        let response = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(
            response.content_type.as_deref(),
            Some("text/html; charset=utf-8")
        );
        assert!(response.body.starts_with("<!DOCTYPE html>"));
        assert!(response.body.contains("<title>ぶくせんの探窟メモ</title>"));
        assert!(response.body.contains(
            "<meta name=\"description\" content=\"1件の記事を1カテゴリで公開しています。\">"
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
            "<meta property=\"og:description\" content=\"1件の記事を1カテゴリで公開しています。\">"
        ));
        assert!(
            response
                .body
                .contains("<meta property=\"og:url\" content=\"https://www.okawak.net\">")
        );
        assert!(response.body.contains("<p>Fixture home content</p>"));
        assert!(response.body.contains("href=\"/\" aria-current=\"page\""));
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
    async fn home_shell_exposes_topcoat_mobile_navigation_contract() {
        let router = create_topcoat_router(fixture_reader(), false);
        let response = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::OK);
        assert!(response.body.contains(
            "<script type=\"module\" src=\"/_topcoat/assets/topcoat-test.js\"></script>"
        ));
        assert!(response.body.contains(
            "<script type=\"module\" src=\"/_topcoat/assets/topcoat-navigation-test.js\"></script>"
        ));
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
                .contains("aria-label=\"Open okawak GitHub profile\"")
        );
    }

    #[tokio::test]
    async fn home_renders_empty_state_without_treating_it_as_an_error() {
        let router = create_topcoat_router(empty_fixture_reader(), false);
        let response = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::OK);
        assert!(response.body.contains(
            "<meta name=\"description\" content=\"0件の記事を0カテゴリで公開しています。\">"
        ));
        assert!(response.body.contains("記事がありません"));
        assert!(!response.body.contains("記事の読み込みに失敗しました"));
    }

    #[tokio::test]
    async fn home_uses_fallback_copy_when_optional_fragment_is_missing() {
        let temp_dir = tempdir().unwrap();
        std::fs::create_dir_all(temp_dir.path().join("articles")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("metadata")).unwrap();
        let fixture_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../e2e/fixtures/site");
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);

        let response = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::OK);
        assert!(
            response
                .body
                .contains("公開済みの artifact をもとに、最近の記事とカテゴリをまとめています。")
        );
    }

    #[tokio::test]
    async fn home_returns_internal_server_error_when_required_artifact_is_missing() {
        let temp_dir = tempdir().unwrap();
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
        let response = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.body.contains("記事の読み込みに失敗しました"));
    }

    #[tokio::test]
    async fn home_returns_internal_server_error_page_when_snapshot_fails() {
        let router = create_topcoat_router(Arc::new(FailingSnapshotReader), false);
        let response = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.body.contains("記事の読み込みに失敗しました"));
    }

    #[tokio::test]
    async fn home_supports_release_aware_conditional_get() {
        let router = create_topcoat_router(validator_reader(fixture_reader()), true);
        let first = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
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
        let router = create_topcoat_router(validator_reader(reader), true);

        let response = response(
            &router,
            Request::builder().uri("/").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn category_renders_the_published_landing_and_articles_as_html() {
        let router = create_topcoat_router(fixture_reader(), false);
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
        assert!(response.body.starts_with("<!DOCTYPE html>"));
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
        assert!(response.body.contains(
            "<meta property=\"og:title\" content=\"Fixture Tech | ぶくせんの探窟メモ\">"
        ));
        assert!(response.body.contains(
            "<meta property=\"og:description\" content=\"Category fixture description\">"
        ));
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
        let router = create_topcoat_router(fixture_reader(), false);

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
        let router = create_topcoat_router(fixture_reader(), false);
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
        let router = create_topcoat_router(Arc::new(FailingSnapshotReader), false);
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
        let router = create_topcoat_router(validator_reader(fixture_reader()), true);
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
        let router = create_topcoat_router(validator_reader(reader), true);

        let response = response(
            &router,
            Request::builder().uri("/tech").body(Body::empty()).unwrap(),
        )
        .await;

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn article_renders_the_published_document_as_html() {
        let router = create_topcoat_router(fixture_reader(), false);
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
        assert!(response.body.starts_with("<!DOCTYPE html>"));
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
            response.body.contains(
                "<link rel=\"canonical\" href=\"https://www.okawak.net/tech/e2e-article\">"
            )
        );
        assert!(
            response.body.contains(
                "<meta property=\"og:title\" content=\"E2E Article | ぶくせんの探窟メモ\">"
            )
        );
        assert!(response.body.contains(
            "<meta property=\"og:description\" content=\"Article fixture description\">"
        ));
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
        let router = create_topcoat_router(fixture_reader(), false);
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
            response.body.contains(
                "<link rel=\"canonical\" href=\"https://www.okawak.net/tech/e2e-article\">"
            )
        );
        assert!(!response.body.contains(
            "<link rel=\"canonical\" href=\"https://www.okawak.net/tech/e2e-article.html\">"
        ));
    }

    #[tokio::test]
    async fn article_returns_not_found_for_invalid_or_missing_documents() {
        let router = create_topcoat_router(fixture_reader(), false);

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
        let fixture_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../e2e/fixtures/site");
        std::fs::copy(
            fixture_root.join("articles/index.json"),
            temp_dir.path().join("articles/index.json"),
        )
        .unwrap();
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
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
        let fixture_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../e2e/fixtures/site");
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(empty_dir.path())), false);
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
        let router = create_topcoat_router(Arc::new(FailingSnapshotReader), false);
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
        let router = create_topcoat_router(validator_reader(fixture_reader()), true);
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
        let router = create_topcoat_router(validator_reader(reader), true);

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
        let router = create_topcoat_router(fixture_reader(), false);
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
        assert!(response.body.starts_with("<!DOCTYPE html>"));
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
        assert!(response.body.contains(
            "<meta property=\"og:title\" content=\"Fixture About | ぶくせんの探窟メモ\">"
        ));
        assert!(
            response.body.contains(
                "<meta property=\"og:description\" content=\"About fixture description\">"
            )
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
    async fn about_shell_initializes_generated_content_renderers() {
        let router = create_topcoat_router(fixture_reader(), false);
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
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
        let router =
            create_topcoat_router(Arc::new(LocalArtifactReader::new(temp_dir.path())), false);
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
        let router = create_topcoat_router(validator_reader(fixture_reader()), true);
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
        let router = create_topcoat_router(validator_reader(reader), true);

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
        let router = create_topcoat_router(validator_reader(fixture_reader()), true);
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
        let disabled = create_topcoat_router(validator_reader(fixture_reader()), false);
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
        let enabled = create_topcoat_router(validator_reader(missing_reader), true);
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
        let router = create_topcoat_router(validator_reader(reader), true);

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
}
