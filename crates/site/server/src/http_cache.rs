//! Conditional HTTP caching for artifact-backed responses.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use infra::DynArtifactReader;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    process,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CACHE_CONTROL_VALUE: &str = "public, max-age=0, must-revalidate";

#[derive(Clone)]
pub struct ArtifactHttpCacheState {
    artifact_reader: DynArtifactReader,
    enabled: bool,
    process_tag: Arc<str>,
    process_started_at: SystemTime,
}

struct ArtifactValidators {
    etag: String,
    last_modified: Option<SystemTime>,
}

impl ArtifactHttpCacheState {
    pub fn new(artifact_reader: DynArtifactReader, enabled: bool) -> Self {
        let process_started_at = SystemTime::now();
        let started_at = process_started_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            artifact_reader,
            enabled,
            process_tag: format!("{}-{started_at}", process::id()).into(),
            process_started_at,
        }
    }

    #[cfg(test)]
    fn with_process_tag(
        artifact_reader: DynArtifactReader,
        enabled: bool,
        process_tag: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            artifact_reader,
            enabled,
            process_tag: process_tag.into(),
            process_started_at: UNIX_EPOCH,
        }
    }

    async fn validators_for(&self, uri: &Uri) -> Option<ArtifactValidators> {
        if !self.enabled {
            return None;
        }

        let snapshot = self.artifact_reader.snapshot().await.ok()?;
        let identity = snapshot.cache_identity()?;
        Some(ArtifactValidators {
            etag: build_weak_etag(&self.process_tag, identity, uri),
            last_modified: representation_last_modified(
                snapshot.last_modified(),
                self.process_started_at,
            ),
        })
    }
}

pub async fn artifact_conditional_get(
    State(state): State<ArtifactHttpCacheState>,
    request: Request,
    next: Next,
) -> Response {
    if !is_artifact_request(request.method(), request.uri().path()) {
        return next.run(request).await;
    }

    let validators = state.validators_for(request.uri()).await;
    let has_if_none_match = request.headers().contains_key(header::IF_NONE_MATCH);
    if has_if_none_match
        && let Some(validators) = validators.as_ref()
        && if_none_match_matches(request.headers(), &validators.etag)
    {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        insert_cache_headers(response.headers_mut(), validators);
        return response;
    }
    let not_modified_since = !has_if_none_match
        && validators
            .as_ref()
            .and_then(|validators| validators.last_modified)
            .is_some_and(|last_modified| {
                if_modified_since_not_modified(request.headers(), last_modified)
            });

    let mut response = next.run(request).await;
    if response.status() == StatusCode::OK
        && let Some(validators) = validators.as_ref()
    {
        if not_modified_since {
            let mut response = StatusCode::NOT_MODIFIED.into_response();
            insert_cache_headers(response.headers_mut(), validators);
            return response;
        }

        insert_cache_headers(response.headers_mut(), validators);
    }
    response
}

fn build_weak_etag(process_tag: &str, snapshot_identity: &str, uri: &Uri) -> String {
    let mut hasher = DefaultHasher::new();
    process_tag.hash(&mut hasher);
    snapshot_identity.hash(&mut hasher);
    uri.hash(&mut hasher);
    format!("W/\"{:016x}\"", hasher.finish())
}

fn if_none_match_matches(headers: &HeaderMap, current_etag: &str) -> bool {
    let current_opaque = weak_opaque_tag(current_etag);
    headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|candidate| {
            let candidate = candidate.trim();
            candidate != "*" && weak_opaque_tag(candidate) == current_opaque
        })
}

fn weak_opaque_tag(etag: &str) -> Option<&str> {
    let etag = etag.trim().strip_prefix("W/").unwrap_or(etag.trim());
    (etag.len() >= 2 && etag.starts_with('"') && etag.ends_with('"')).then_some(etag)
}

fn if_modified_since_not_modified(headers: &HeaderMap, last_modified: SystemTime) -> bool {
    let mut values = headers.get_all(header::IF_MODIFIED_SINCE).iter();
    let Some(value) = values.next() else {
        return false;
    };
    if values.next().is_some() {
        return false;
    }

    value
        .to_str()
        .ok()
        .and_then(|value| httpdate::parse_http_date(value).ok())
        .is_some_and(|if_modified_since| last_modified <= if_modified_since)
}

fn truncate_to_http_seconds(value: SystemTime) -> Option<SystemTime> {
    let seconds = value.duration_since(UNIX_EPOCH).ok()?.as_secs();
    UNIX_EPOCH.checked_add(Duration::from_secs(seconds))
}

fn representation_last_modified(
    release_generated_at: Option<SystemTime>,
    process_started_at: SystemTime,
) -> Option<SystemTime> {
    release_generated_at
        .map(|release_generated_at| release_generated_at.max(process_started_at))
        .and_then(truncate_to_http_seconds)
}

fn insert_cache_headers(headers: &mut HeaderMap, validators: &ArtifactValidators) {
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(&validators.etag).expect("generated ETag is a valid header value"),
    );
    if let Some(last_modified) = validators.last_modified {
        headers.insert(
            header::LAST_MODIFIED,
            HeaderValue::from_str(&httpdate::fmt_http_date(last_modified))
                .expect("generated Last-Modified is a valid header value"),
        );
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL_VALUE),
    );
}

fn is_artifact_request(method: &Method, path: &str) -> bool {
    if method != Method::GET && method != Method::HEAD {
        return false;
    }
    if path == "/api/articles" {
        return true;
    }
    if path == "/api" || path.starts_with("/api/") {
        return false;
    }

    !is_static_path(path)
}

fn is_static_path(path: &str) -> bool {
    path == "/pkg"
        || path.starts_with("/pkg/")
        || path == "/assets"
        || path.starts_with("/assets/")
        || path == "/favicon.ico"
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::{Router, body::Body, routing::get};
    use domain::{
        ArticleIndexDocument, Category, CategoryIndexDocument, PageArtifactDocument, PageKey,
        SiteMetadataDocument, Slug,
    };
    use infra::{ArtifactReader, ArtifactSnapshot, DynArtifactSnapshot, Result};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    const RELEASE_LAST_MODIFIED: &str = "Tue, 14 Nov 2023 22:13:20 GMT";

    fn release_last_modified() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    #[derive(Clone)]
    struct FixedReader {
        snapshot: DynArtifactSnapshot,
    }

    #[async_trait]
    impl ArtifactReader for FixedReader {
        async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
            Ok(Arc::clone(&self.snapshot))
        }
    }

    struct FixedSnapshot {
        identity: Option<String>,
        last_modified: Option<SystemTime>,
    }

    #[async_trait]
    impl ArtifactSnapshot for FixedSnapshot {
        fn cache_identity(&self) -> Option<&str> {
            self.identity.as_deref()
        }

        fn last_modified(&self) -> Option<SystemTime> {
            self.last_modified
        }

        async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
            unreachable!()
        }

        async fn read_category_index(&self, _category: &str) -> Result<CategoryIndexDocument> {
            unreachable!()
        }

        async fn read_category_html(&self, _category: &Category) -> Result<String> {
            unreachable!()
        }

        async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
            unreachable!()
        }

        async fn read_article_html(&self, _category: &Category, _slug: &Slug) -> Result<String> {
            unreachable!()
        }

        async fn read_page_document(&self, _page: &PageKey) -> Result<PageArtifactDocument> {
            unreachable!()
        }
    }

    fn cache_state(identity: Option<&str>, enabled: bool) -> ArtifactHttpCacheState {
        let snapshot: DynArtifactSnapshot = Arc::new(FixedSnapshot {
            identity: identity.map(str::to_string),
            last_modified: identity.map(|_| release_last_modified()),
        });
        let reader: DynArtifactReader = Arc::new(FixedReader { snapshot });
        ArtifactHttpCacheState::with_process_tag(reader, enabled, "process-1")
    }

    fn test_app(identity: Option<&str>, enabled: bool, calls: Arc<AtomicUsize>) -> Router {
        Router::new()
            .route(
                "/",
                get(move || {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        "artifact page"
                    }
                }),
            )
            .route("/missing", get(|| async { StatusCode::NOT_FOUND }))
            .route(
                "/error",
                get(|| async { StatusCode::INTERNAL_SERVER_ERROR }),
            )
            .route("/api/health", get(|| async { "OK" }))
            .layer(axum::middleware::from_fn_with_state(
                cache_state(identity, enabled),
                artifact_conditional_get,
            ))
    }

    #[test]
    fn weak_etag_changes_with_process_release_and_uri() {
        let uri = Uri::from_static("/tech/article");
        let etag = build_weak_etag("process-1", "release-1", &uri);

        assert_eq!(etag, build_weak_etag("process-1", "release-1", &uri));
        assert_ne!(etag, build_weak_etag("process-2", "release-1", &uri));
        assert_ne!(etag, build_weak_etag("process-1", "release-2", &uri));
        assert_ne!(
            etag,
            build_weak_etag("process-1", "release-1", &Uri::from_static("/about"))
        );
    }

    #[test]
    fn last_modified_uses_the_newer_release_or_process_time() {
        let release = release_last_modified();
        let newer_process = release + Duration::from_secs(60);

        assert_eq!(
            representation_last_modified(Some(release), UNIX_EPOCH),
            Some(release)
        );
        assert_eq!(
            representation_last_modified(Some(release), newer_process),
            Some(newer_process)
        );
        assert_eq!(representation_last_modified(None, newer_process), None);
    }

    #[test]
    fn if_none_match_uses_weak_comparison_and_ignores_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"other\", \"current\""),
        );

        assert!(if_none_match_matches(&headers, "W/\"current\""));

        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));
        assert!(!if_none_match_matches(&headers, "W/\"current\""));
    }

    #[test]
    fn only_get_and_head_artifact_routes_are_eligible() {
        assert!(is_artifact_request(&Method::GET, "/"));
        assert!(is_artifact_request(&Method::HEAD, "/tech/article"));
        assert!(is_artifact_request(&Method::GET, "/api/articles"));
        assert!(!is_artifact_request(&Method::POST, "/"));
        assert!(!is_artifact_request(&Method::GET, "/api/health"));
        assert!(!is_artifact_request(&Method::GET, "/api/ready"));
        assert!(!is_artifact_request(&Method::GET, "/api/server-fn"));
        assert!(!is_artifact_request(&Method::GET, "/pkg/web.js"));
        assert!(!is_artifact_request(&Method::GET, "/assets/logo.png"));
        assert!(!is_artifact_request(&Method::GET, "/favicon.ico"));
    }

    #[tokio::test]
    async fn successful_response_has_etag_and_matching_request_short_circuits() {
        let calls = Arc::new(AtomicUsize::new(0));
        let app = test_app(Some("release-1"), true, Arc::clone(&calls));
        let first = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let etag = first.headers().get(header::ETAG).unwrap().clone();

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(
            first.headers().get(header::CACHE_CONTROL).unwrap(),
            CACHE_CONTROL_VALUE
        );
        assert_eq!(
            first.headers().get(header::LAST_MODIFIED).unwrap(),
            RELEASE_LAST_MODIFIED
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::IF_NONE_MATCH, &etag)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(second.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(second.headers().get(header::ETAG), Some(&etag));
        assert_eq!(
            second.headers().get(header::LAST_MODIFIED).unwrap(),
            RELEASE_LAST_MODIFIED
        );
        assert_eq!(
            second.headers().get(header::CACHE_CONTROL).unwrap(),
            CACHE_CONTROL_VALUE
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn if_modified_since_returns_304_after_selecting_a_successful_response() {
        let calls = Arc::new(AtomicUsize::new(0));
        let response = test_app(Some("release-1"), true, Arc::clone(&calls))
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::IF_MODIFIED_SINCE, RELEASE_LAST_MODIFIED)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            response.headers().get(header::LAST_MODIFIED).unwrap(),
            RELEASE_LAST_MODIFIED
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn if_none_match_takes_precedence_over_if_modified_since() {
        let calls = Arc::new(AtomicUsize::new(0));
        let response = test_app(Some("release-1"), true, Arc::clone(&calls))
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::IF_NONE_MATCH, "\"different\"")
                    .header(header::IF_MODIFIED_SINCE, RELEASE_LAST_MODIFIED)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn stale_invalid_or_multiple_if_modified_since_values_are_ignored() {
        for values in [
            vec!["Tue, 14 Nov 2023 22:13:19 GMT"],
            vec!["invalid"],
            vec![RELEASE_LAST_MODIFIED, RELEASE_LAST_MODIFIED],
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let app = test_app(Some("release-1"), true, Arc::clone(&calls));
            let mut request = Request::builder().uri("/").body(Body::empty()).unwrap();
            for value in values {
                request
                    .headers_mut()
                    .append(header::IF_MODIFIED_SINCE, HeaderValue::from_static(value));
            }

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        }
    }

    #[tokio::test]
    async fn if_modified_since_does_not_turn_missing_or_error_responses_into_304() {
        for uri in ["/missing", "/error"] {
            let response = test_app(Some("release-1"), true, Arc::new(AtomicUsize::new(0)))
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .header(header::IF_MODIFIED_SINCE, RELEASE_LAST_MODIFIED)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_ne!(response.status(), StatusCode::NOT_MODIFIED, "{uri}");
            assert!(response.headers().get(header::LAST_MODIFIED).is_none());
        }
    }

    #[tokio::test]
    async fn missing_identity_disabled_cache_exclusions_and_errors_have_no_etag() {
        for (identity, enabled, uri) in [
            (None, true, "/"),
            (Some("release-1"), false, "/"),
            (Some("release-1"), true, "/api/health"),
            (Some("release-1"), true, "/missing"),
            (Some("release-1"), true, "/error"),
        ] {
            let response = test_app(identity, enabled, Arc::new(AtomicUsize::new(0)))
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert!(response.headers().get(header::ETAG).is_none(), "{uri}");
            assert!(
                response.headers().get(header::CACHE_CONTROL).is_none(),
                "{uri}"
            );
        }
    }
}
