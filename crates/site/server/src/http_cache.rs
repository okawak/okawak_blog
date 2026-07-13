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
    time::{SystemTime, UNIX_EPOCH},
};

const CACHE_CONTROL_VALUE: &str = "public, max-age=0, must-revalidate";

#[derive(Clone)]
pub struct ArtifactHttpCacheState {
    artifact_reader: DynArtifactReader,
    enabled: bool,
    process_tag: Arc<str>,
}

impl ArtifactHttpCacheState {
    pub fn new(artifact_reader: DynArtifactReader, enabled: bool) -> Self {
        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self {
            artifact_reader,
            enabled,
            process_tag: format!("{}-{started_at}", process::id()).into(),
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
        }
    }

    async fn etag_for(&self, uri: &Uri) -> Option<String> {
        if !self.enabled {
            return None;
        }

        let snapshot = self.artifact_reader.snapshot().await.ok()?;
        let identity = snapshot.cache_identity()?;
        Some(build_weak_etag(&self.process_tag, identity, uri))
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

    let etag = state.etag_for(request.uri()).await;
    if let Some(etag) = etag.as_deref()
        && if_none_match_matches(request.headers(), etag)
    {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        insert_cache_headers(response.headers_mut(), etag);
        return response;
    }

    let mut response = next.run(request).await;
    if response.status() == StatusCode::OK
        && let Some(etag) = etag.as_deref()
    {
        insert_cache_headers(response.headers_mut(), etag);
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

fn insert_cache_headers(headers: &mut HeaderMap, etag: &str) {
    headers.insert(
        header::ETAG,
        HeaderValue::from_str(etag).expect("generated ETag is a valid header value"),
    );
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
    use tower::ServiceExt;

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
    }

    #[async_trait]
    impl ArtifactSnapshot for FixedSnapshot {
        fn cache_identity(&self) -> Option<&str> {
            self.identity.as_deref()
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
            second.headers().get(header::CACHE_CONTROL).unwrap(),
            CACHE_CONTROL_VALUE
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
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
