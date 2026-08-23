//! Parallel Topcoat runtime shell used during the framework migration.

use infra::{DynArtifactReader, DynArtifactSnapshot};
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{
        Body, LayerFn, LayerFuture, Next, Router, StatusCode, content::Json,
        error::internal_server_error, request, response::Response, route,
    },
};

use crate::{
    article_index::{read_article_index, read_article_index_from_snapshot},
    http_cache::{ArtifactConditionalGetDecision, ArtifactHttpCacheState},
    readiness::check_artifact_readiness,
};

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
                let cx = cx.with(snapshot);
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
) -> Router {
    Router::builder()
        .route(health)
        .route(readiness)
        .route(articles)
        .layer(LayerFn::new(
            Some("/api/articles"),
            artifact_conditional_get,
        ))
        .app_context(ArtifactHttpCacheState::new(
            artifact_reader.clone(),
            validators_enabled,
        ))
        .app_context(ArtifactReaderContext(artifact_reader))
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
    use topcoat::router::{
        Body, HeaderMap, Router, StatusCode, header, request::Request, to_bytes,
    };

    use super::create_topcoat_router;

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

    #[derive(Clone)]
    struct ValidatorReader {
        inner: DynArtifactReader,
    }

    #[derive(Clone)]
    struct CountingReader {
        inner: DynArtifactReader,
        snapshot_calls: Arc<AtomicUsize>,
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
