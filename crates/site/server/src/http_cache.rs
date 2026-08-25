//! Conditional HTTP caching for artifact-backed responses.

use infra::{ArtifactSourceConfig, DynArtifactReader, DynArtifactSnapshot};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    process,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use topcoat::router::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};

const CACHE_CONTROL_VALUE: &str = "public, max-age=0, must-revalidate";

/// Enables release validators only when an S3 snapshot cache can provide a stable identity.
pub fn artifact_validators_enabled(config: &ArtifactSourceConfig) -> bool {
    matches!(
        config,
        ArtifactSourceConfig::S3 { cache_ttl, .. } if !cache_ttl.is_zero()
    )
}

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

pub(crate) struct ArtifactConditionalGetDecision {
    snapshot: Option<DynArtifactSnapshot>,
    validators: Option<ArtifactValidators>,
    etag_matches: bool,
    not_modified_since: bool,
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

    async fn snapshot_for_request(&self) -> Option<DynArtifactSnapshot> {
        if !self.enabled {
            return None;
        }
        self.artifact_reader.snapshot().await.ok()
    }

    fn validators_for(
        &self,
        snapshot: &DynArtifactSnapshot,
        uri: &Uri,
    ) -> Option<ArtifactValidators> {
        let identity = snapshot.cache_identity()?;
        Some(ArtifactValidators {
            etag: build_weak_etag(&self.process_tag, identity, uri),
            last_modified: representation_last_modified(
                snapshot.last_modified(),
                self.process_started_at,
            ),
        })
    }

    pub(crate) async fn conditional_get(
        &self,
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Option<ArtifactConditionalGetDecision> {
        if !is_artifact_request(method, uri.path()) {
            return None;
        }

        let snapshot = self.snapshot_for_request().await;
        let validators = snapshot
            .as_ref()
            .and_then(|snapshot| self.validators_for(snapshot, uri));
        let has_if_none_match = headers.contains_key(header::IF_NONE_MATCH);
        let etag_matches = has_if_none_match
            && validators
                .as_ref()
                .is_some_and(|validators| if_none_match_matches(headers, &validators.etag));
        let not_modified_since = !has_if_none_match
            && validators
                .as_ref()
                .and_then(|validators| validators.last_modified)
                .is_some_and(|last_modified| {
                    if_modified_since_not_modified(headers, last_modified)
                });

        Some(ArtifactConditionalGetDecision {
            snapshot,
            validators,
            etag_matches,
            not_modified_since,
        })
    }
}

impl ArtifactConditionalGetDecision {
    pub(crate) fn snapshot(&self) -> Option<DynArtifactSnapshot> {
        self.snapshot.clone()
    }

    pub(crate) fn should_short_circuit(&self) -> bool {
        self.etag_matches
    }

    pub(crate) fn should_return_not_modified_after_response(&self, status: StatusCode) -> bool {
        status == StatusCode::OK && self.not_modified_since
    }

    pub(crate) fn should_attach_validators(&self, status: StatusCode) -> bool {
        status == StatusCode::OK && self.validators.is_some()
    }

    pub(crate) fn insert_headers(&self, headers: &mut HeaderMap) {
        if let Some(validators) = self.validators.as_ref() {
            insert_cache_headers(headers, validators);
        }
    }
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
    path == "/_topcoat/assets" || path.starts_with("/_topcoat/assets/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::{
        ArticleIndexDocument, Category, CategoryArtifactDocument, HomeFragmentArtifactDocument,
        PageArtifactDocument, PageKey, SiteMetadataDocument, Slug,
    };
    use infra::{ArtifactReader, ArtifactSnapshot, DynArtifactSnapshot, Result};

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

        async fn read_category_document(
            &self,
            _category: &Category,
        ) -> Result<CategoryArtifactDocument> {
            unreachable!()
        }

        async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
            unreachable!()
        }

        async fn read_article_html(&self, _category: &Category, _slug: &Slug) -> Result<String> {
            unreachable!()
        }

        async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
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

    async fn decision(
        identity: Option<&str>,
        enabled: bool,
        uri: &'static str,
        headers: HeaderMap,
    ) -> Option<ArtifactConditionalGetDecision> {
        cache_state(identity, enabled)
            .conditional_get(&Method::GET, &Uri::from_static(uri), &headers)
            .await
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
        assert!(!is_artifact_request(
            &Method::GET,
            "/_topcoat/assets/topcoat.js"
        ));
    }

    #[tokio::test]
    async fn successful_response_has_validators_and_matching_etag_short_circuits() {
        let first = decision(Some("release-1"), true, "/", HeaderMap::new())
            .await
            .unwrap();
        assert!(!first.should_short_circuit());
        assert!(first.should_attach_validators(StatusCode::OK));

        let mut response_headers = HeaderMap::new();
        first.insert_headers(&mut response_headers);
        let etag = response_headers.get(header::ETAG).unwrap().clone();
        assert_eq!(
            response_headers.get(header::CACHE_CONTROL).unwrap(),
            CACHE_CONTROL_VALUE
        );
        assert_eq!(
            response_headers.get(header::LAST_MODIFIED).unwrap(),
            RELEASE_LAST_MODIFIED
        );

        let mut request_headers = HeaderMap::new();
        request_headers.insert(header::IF_NONE_MATCH, etag);
        let second = decision(Some("release-1"), true, "/", request_headers)
            .await
            .unwrap();
        assert!(second.should_short_circuit());
    }

    #[tokio::test]
    async fn if_modified_since_only_converts_successful_responses() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_MODIFIED_SINCE,
            HeaderValue::from_static(RELEASE_LAST_MODIFIED),
        );
        let result = decision(Some("release-1"), true, "/", headers)
            .await
            .unwrap();
        assert!(result.should_return_not_modified_after_response(StatusCode::OK));
        assert!(!result.should_return_not_modified_after_response(StatusCode::NOT_FOUND));
        assert!(
            !result.should_return_not_modified_after_response(StatusCode::INTERNAL_SERVER_ERROR)
        );
    }

    #[tokio::test]
    async fn if_none_match_takes_precedence_over_if_modified_since() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::IF_NONE_MATCH,
            HeaderValue::from_static("\"different\""),
        );
        headers.insert(
            header::IF_MODIFIED_SINCE,
            HeaderValue::from_static(RELEASE_LAST_MODIFIED),
        );
        let result = decision(Some("release-1"), true, "/", headers)
            .await
            .unwrap();
        assert!(!result.should_short_circuit());
        assert!(!result.should_return_not_modified_after_response(StatusCode::OK));
    }

    #[tokio::test]
    async fn stale_invalid_or_multiple_if_modified_since_values_are_ignored() {
        for values in [
            vec!["Tue, 14 Nov 2023 22:13:19 GMT"],
            vec!["invalid"],
            vec![RELEASE_LAST_MODIFIED, RELEASE_LAST_MODIFIED],
        ] {
            let mut headers = HeaderMap::new();
            for value in values {
                headers.append(header::IF_MODIFIED_SINCE, HeaderValue::from_static(value));
            }
            let result = decision(Some("release-1"), true, "/", headers)
                .await
                .unwrap();
            assert!(!result.should_return_not_modified_after_response(StatusCode::OK));
        }
    }

    #[tokio::test]
    async fn missing_identity_disabled_cache_and_excluded_routes_have_no_validators() {
        for (identity, enabled, uri) in [(None, true, "/"), (Some("release-1"), false, "/")] {
            let result = decision(identity, enabled, uri, HeaderMap::new())
                .await
                .unwrap();
            assert!(!result.should_attach_validators(StatusCode::OK));
        }

        assert!(
            decision(Some("release-1"), true, "/api/health", HeaderMap::new())
                .await
                .is_none()
        );
    }
}
