use crate::{ArtifactReader, ArtifactSnapshot, DynArtifactReader, DynArtifactSnapshot, Result};
use async_trait::async_trait;
use domain::{
    ArticleIndexDocument, Category, CategoryArtifactDocument, ContentAssetName,
    HomeFragmentArtifactDocument, Locale, PageArtifactDocument, PageKey, SiteLocalesDocument,
    SiteMetadataDocument, Slug,
};
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::{Mutex, OnceCell};

pub struct CachingArtifactReader {
    inner: DynArtifactReader,
    snapshot_ttl: Duration,
    cached: Mutex<Option<CachedSnapshot>>,
}

struct CachedSnapshot {
    checked_at: Instant,
    snapshot: DynArtifactSnapshot,
}

impl CachedSnapshot {
    fn is_fresh(&self, now: Instant, ttl: Duration) -> bool {
        now.duration_since(self.checked_at) < ttl
    }

    fn has_same_identity(&self, candidate: &DynArtifactSnapshot) -> bool {
        self.snapshot
            .cache_identity()
            .is_some_and(|identity| Some(identity) == candidate.cache_identity())
    }

    fn immutable_fallback(&mut self, checked_at: Instant) -> Option<(String, DynArtifactSnapshot)> {
        let identity = self.snapshot.cache_identity()?.to_string();
        self.checked_at = checked_at;
        Some((identity, Arc::clone(&self.snapshot)))
    }
}

impl CachingArtifactReader {
    pub fn new(inner: DynArtifactReader, snapshot_ttl: Duration) -> Self {
        Self {
            inner,
            snapshot_ttl,
            cached: Mutex::new(None),
        }
    }
}

#[async_trait]
impl ArtifactReader for CachingArtifactReader {
    async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
        if self.snapshot_ttl.is_zero() {
            return self.inner.snapshot().await;
        }

        let mut cached = self.cached.lock().await;
        if let Some(cached) = cached.as_ref()
            && cached.is_fresh(Instant::now(), self.snapshot_ttl)
        {
            return Ok(Arc::clone(&cached.snapshot));
        }

        let inner_snapshot = match self.inner.snapshot().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                if let Some((identity, snapshot)) = cached
                    .as_mut()
                    .and_then(|cached| cached.immutable_fallback(Instant::now()))
                {
                    eprintln!(
                        "Artifact snapshot refresh failed; serving stale release {identity}: {error}"
                    );
                    return Ok(snapshot);
                }
                return Err(error);
            }
        };
        if let Some(cached) = cached.as_mut()
            && cached.has_same_identity(&inner_snapshot)
        {
            cached.checked_at = Instant::now();
            return Ok(Arc::clone(&cached.snapshot));
        }
        if inner_snapshot.cache_identity().is_none()
            && let Some((identity, snapshot)) = cached
                .as_mut()
                .and_then(|cached| cached.immutable_fallback(Instant::now()))
        {
            eprintln!(
                "Artifact snapshot refresh lost current release; serving stale release {identity}"
            );
            return Ok(snapshot);
        }

        let snapshot: DynArtifactSnapshot = Arc::new(CachingArtifactSnapshot::new(inner_snapshot));
        *cached = Some(CachedSnapshot {
            checked_at: Instant::now(),
            snapshot: Arc::clone(&snapshot),
        });
        Ok(snapshot)
    }
}

struct CachingArtifactSnapshot {
    localized: KeyedCache<Option<DynArtifactSnapshot>>,
    locales: OnceCell<SiteLocalesDocument>,
    tags: OnceCell<domain::TagLabels>,
    content_assets: ContentAssetCache,
    inner: DynArtifactSnapshot,
    article_index: OnceCell<ArticleIndexDocument>,
    site_metadata: OnceCell<SiteMetadataDocument>,
    home_fragment: OnceCell<HomeFragmentArtifactDocument>,
    category_documents: KeyedCache<CategoryArtifactDocument>,
    article_html: KeyedCache<String>,
    page_documents: KeyedCache<PageArtifactDocument>,
}

impl CachingArtifactSnapshot {
    fn new(inner: DynArtifactSnapshot) -> Self {
        Self {
            localized: KeyedCache::new(),
            locales: OnceCell::new(),
            tags: OnceCell::new(),
            content_assets: ContentAssetCache::default(),
            inner,
            article_index: OnceCell::new(),
            site_metadata: OnceCell::new(),
            home_fragment: OnceCell::new(),
            category_documents: KeyedCache::new(),
            article_html: KeyedCache::new(),
            page_documents: KeyedCache::new(),
        }
    }
}

#[async_trait]
impl ArtifactSnapshot for CachingArtifactSnapshot {
    fn cache_identity(&self) -> Option<&str> {
        self.inner.cache_identity()
    }

    fn last_modified(&self) -> Option<SystemTime> {
        self.inner.last_modified()
    }

    async fn localized(&self, locale: Locale) -> Result<Option<DynArtifactSnapshot>> {
        self.localized
            .get_or_try_init(locale.to_string(), || async {
                Ok(self.inner.localized(locale).await?.map(|snapshot| {
                    Arc::new(CachingArtifactSnapshot::new(snapshot)) as DynArtifactSnapshot
                }))
            })
            .await
    }

    async fn read_locales(&self) -> Result<SiteLocalesDocument> {
        self.locales
            .get_or_try_init(|| self.inner.read_locales())
            .await
            .cloned()
    }

    async fn read_tag_labels(&self) -> Result<domain::TagLabels> {
        self.tags
            .get_or_try_init(|| self.inner.read_tag_labels())
            .await
            .cloned()
    }

    async fn read_content_asset(&self, name: &ContentAssetName) -> Result<Option<Vec<u8>>> {
        self.content_assets
            .get_or_try_init(name.as_str(), || self.inner.read_content_asset(name))
            .await
    }

    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.article_index
            .get_or_try_init(|| self.inner.read_article_index())
            .await
            .cloned()
    }

    async fn read_category_document(
        &self,
        category: &Category,
    ) -> Result<CategoryArtifactDocument> {
        self.category_documents
            .get_or_try_init(category.as_str().to_string(), || {
                self.inner.read_category_document(category)
            })
            .await
    }

    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        self.site_metadata
            .get_or_try_init(|| self.inner.read_site_metadata())
            .await
            .cloned()
    }

    async fn read_article_html(&self, category: &Category, slug: &Slug) -> Result<String> {
        self.article_html
            .get_or_try_init(format!("{}/{}", category.as_str(), slug.as_str()), || {
                self.inner.read_article_html(category, slug)
            })
            .await
    }

    async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
        self.home_fragment
            .get_or_try_init(|| self.inner.read_home_fragment())
            .await
            .cloned()
    }

    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
        self.page_documents
            .get_or_try_init(page.as_str().to_string(), || {
                self.inner.read_page_document(page)
            })
            .await
    }
}

type ContentAssetCell = Arc<OnceCell<Option<Vec<u8>>>>;

#[derive(Default)]
struct ContentAssetCache {
    // Only map access is synchronous; storage I/O happens outside this lock.
    entries: std::sync::Mutex<HashMap<String, ContentAssetCell>>,
}

impl ContentAssetCache {
    async fn get_or_try_init<F, Fut>(&self, key: &str, load: F) -> Result<Option<Vec<u8>>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Option<Vec<u8>>>>,
    {
        let cell = {
            let mut entries = self.entries.lock().unwrap();
            Arc::clone(entries.entry(key.into()).or_default())
        };
        let read = ContentAssetRead {
            cache: self,
            key,
            cell,
        };
        read.cell.get_or_try_init(load).await.cloned()
    }
}

struct ContentAssetRead<'a> {
    cache: &'a ContentAssetCache,
    key: &'a str,
    cell: ContentAssetCell,
}

impl Drop for ContentAssetRead<'_> {
    fn drop(&mut self) {
        let mut entries = self.cache.entries.lock().unwrap();
        // The map and this final reader own the last two references. Keep successes;
        // release misses, errors and cancelled loads when no reader needs the cell.
        if Arc::strong_count(&self.cell) == 2 && !matches!(self.cell.get(), Some(Some(_))) {
            entries.remove(self.key);
        }
    }
}

struct KeyedCache<T> {
    entries: Mutex<HashMap<String, Arc<OnceCell<T>>>>,
}

impl<T> KeyedCache<T>
where
    T: Clone,
{
    fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    async fn get_or_try_init<F, Fut>(&self, key: String, load: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let cell = {
            let mut entries = self.entries.lock().await;
            Arc::clone(
                entries
                    .entry(key)
                    .or_insert_with(|| Arc::new(OnceCell::new())),
            )
        };

        cell.get_or_try_init(load).await.cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InfraError;
    use domain::CategoryMetadataDocument;
    use std::collections::VecDeque;
    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::UNIX_EPOCH;

    struct CountingReader {
        snapshot_calls: Arc<AtomicUsize>,
        article_reads: Arc<AtomicUsize>,
        fail_next_article_read: Arc<AtomicBool>,
        cache_identity: Option<&'static str>,
        last_modified: Option<SystemTime>,
    }

    #[async_trait]
    impl ArtifactReader for CountingReader {
        async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
            self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(CountingSnapshot {
                article_reads: Arc::clone(&self.article_reads),
                fail_next_article_read: Arc::clone(&self.fail_next_article_read),
                cache_identity: self.cache_identity,
                last_modified: self.last_modified,
            }))
        }
    }

    struct CountingSnapshot {
        article_reads: Arc<AtomicUsize>,
        fail_next_article_read: Arc<AtomicBool>,
        cache_identity: Option<&'static str>,
        last_modified: Option<SystemTime>,
    }

    #[derive(Clone, Copy)]
    enum SnapshotOutcome {
        Success(Option<&'static str>),
        Failure,
    }

    struct SequencedReader {
        outcomes: StdMutex<VecDeque<SnapshotOutcome>>,
        snapshot_calls: Arc<AtomicUsize>,
        article_reads: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ArtifactReader for SequencedReader {
        async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
            self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
            let outcome = self
                .outcomes
                .lock()
                .unwrap()
                .pop_front()
                .expect("test snapshot outcome");

            match outcome {
                SnapshotOutcome::Success(cache_identity) => Ok(Arc::new(CountingSnapshot {
                    article_reads: Arc::clone(&self.article_reads),
                    fail_next_article_read: Arc::new(AtomicBool::new(false)),
                    cache_identity,
                    last_modified: cache_identity
                        .map(|_| UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
                })),
                SnapshotOutcome::Failure => {
                    Err(InfraError::Io(std::io::Error::other("refresh failed")))
                }
            }
        }
    }

    #[async_trait]
    impl ArtifactSnapshot for CountingSnapshot {
        async fn read_content_asset(&self, name: &ContentAssetName) -> Result<Option<Vec<u8>>> {
            self.article_reads.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            Ok(name
                .as_str()
                .starts_with(&"0".repeat(64))
                .then(|| b"image".to_vec()))
        }

        fn cache_identity(&self) -> Option<&str> {
            self.cache_identity
        }

        fn last_modified(&self) -> Option<SystemTime> {
            self.last_modified
        }

        async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
            self.article_reads.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            if self.fail_next_article_read.swap(false, Ordering::SeqCst) {
                return Err(InfraError::Io(std::io::Error::other("temporary failure")));
            }
            Ok(ArticleIndexDocument { articles: vec![] })
        }

        async fn read_category_document(
            &self,
            category: &Category,
        ) -> Result<CategoryArtifactDocument> {
            Ok(CategoryArtifactDocument {
                category: category.as_str().to_string(),
                title: category.display_name().to_string(),
                description: None,
                html: category.as_str().to_string(),
                updated_at: String::new(),
                articles: vec![],
            })
        }

        async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
            Ok(SiteMetadataDocument {
                total_articles: 0,
                categories: Vec::<CategoryMetadataDocument>::new(),
            })
        }

        async fn read_article_html(&self, category: &Category, slug: &Slug) -> Result<String> {
            Ok(format!("{}/{}", category.as_str(), slug.as_str()))
        }

        async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
            Ok(HomeFragmentArtifactDocument {
                title: "Home".to_string(),
                description: None,
                html: String::new(),
                updated_at: String::new(),
            })
        }

        async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
            Ok(PageArtifactDocument {
                page: page.clone(),
                title: page.as_str().to_string(),
                description: None,
                html: String::new(),
                updated_at: String::new(),
            })
        }
    }

    fn counting_reader(
        fail_next_article_read: bool,
    ) -> (CachingArtifactReader, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let snapshot_calls = Arc::new(AtomicUsize::new(0));
        let article_reads = Arc::new(AtomicUsize::new(0));
        let inner: DynArtifactReader = Arc::new(CountingReader {
            snapshot_calls: Arc::clone(&snapshot_calls),
            article_reads: Arc::clone(&article_reads),
            fail_next_article_read: Arc::new(AtomicBool::new(fail_next_article_read)),
            cache_identity: Some("release-1"),
            last_modified: Some(std::time::UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
        });
        (
            CachingArtifactReader::new(inner, Duration::from_secs(60)),
            snapshot_calls,
            article_reads,
        )
    }

    fn sequenced_reader(
        outcomes: impl IntoIterator<Item = SnapshotOutcome>,
        snapshot_ttl: Duration,
    ) -> (CachingArtifactReader, Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let snapshot_calls = Arc::new(AtomicUsize::new(0));
        let article_reads = Arc::new(AtomicUsize::new(0));
        let inner: DynArtifactReader = Arc::new(SequencedReader {
            outcomes: StdMutex::new(outcomes.into_iter().collect()),
            snapshot_calls: Arc::clone(&snapshot_calls),
            article_reads: Arc::clone(&article_reads),
        });
        (
            CachingArtifactReader::new(inner, snapshot_ttl),
            snapshot_calls,
            article_reads,
        )
    }

    #[tokio::test]
    async fn reuses_snapshot_and_single_flights_concurrent_artifact_reads() {
        let (reader, snapshot_calls, article_reads) = counting_reader(false);
        let first = reader.snapshot().await.unwrap();
        let second = reader.snapshot().await.unwrap();

        assert_eq!(
            first.last_modified(),
            Some(std::time::UNIX_EPOCH + Duration::from_secs(1_700_000_000))
        );

        let (first_result, second_result) =
            tokio::join!(first.read_article_index(), second.read_article_index());

        assert!(first_result.is_ok());
        assert!(second_result.is_ok());
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
        assert_eq!(article_reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn zero_ttl_disables_snapshot_and_artifact_cache() {
        let snapshot_calls = Arc::new(AtomicUsize::new(0));
        let article_reads = Arc::new(AtomicUsize::new(0));
        let inner: DynArtifactReader = Arc::new(CountingReader {
            snapshot_calls: Arc::clone(&snapshot_calls),
            article_reads: Arc::clone(&article_reads),
            fail_next_article_read: Arc::new(AtomicBool::new(false)),
            cache_identity: Some("release-1"),
            last_modified: None,
        });
        let reader = CachingArtifactReader::new(inner, Duration::ZERO);

        reader
            .snapshot()
            .await
            .unwrap()
            .read_article_index()
            .await
            .unwrap();
        reader
            .snapshot()
            .await
            .unwrap()
            .read_article_index()
            .await
            .unwrap();

        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 2);
        assert_eq!(article_reads.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn artifact_errors_are_not_cached() {
        let (reader, _, article_reads) = counting_reader(true);
        let snapshot = reader.snapshot().await.unwrap();

        assert!(snapshot.read_article_index().await.is_err());
        assert!(snapshot.read_article_index().await.is_ok());
        assert_eq!(article_reads.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn content_asset_cache_retains_only_successful_reads() {
        let root = tempfile::tempdir().unwrap();
        let assets = root.path().join("content-assets");
        std::fs::create_dir(&assets).unwrap();
        let found = ContentAssetName::new(format!("{:064x}.png", 0)).unwrap();
        std::fs::write(assets.join(found.as_str()), b"image").unwrap();
        let snapshot =
            CachingArtifactSnapshot::new(Arc::new(crate::LocalArtifactReader::new(root.path())));

        assert_eq!(
            snapshot.read_content_asset(&found).await.unwrap(),
            Some(b"image".to_vec())
        );
        std::fs::remove_file(assets.join(found.as_str())).unwrap();
        assert_eq!(
            snapshot.read_content_asset(&found).await.unwrap(),
            Some(b"image".to_vec())
        );
        for id in 1..=128 {
            let missing = ContentAssetName::new(format!("{id:064x}.png")).unwrap();
            assert!(
                snapshot
                    .read_content_asset(&missing)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        let failed = ContentAssetName::new(format!("{:064x}.png", 129)).unwrap();
        std::fs::create_dir(assets.join(failed.as_str())).unwrap();
        assert!(snapshot.read_content_asset(&failed).await.is_err());
        assert_eq!(snapshot.content_assets.entries.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn content_asset_reads_single_flight_concurrent_requests() {
        let reads = Arc::new(AtomicUsize::new(0));
        let snapshot = CachingArtifactSnapshot::new(Arc::new(CountingSnapshot {
            article_reads: Arc::clone(&reads),
            fail_next_article_read: Arc::new(AtomicBool::new(false)),
            cache_identity: Some("release-1"),
            last_modified: None,
        }));
        for present in [true, false] {
            reads.store(0, Ordering::SeqCst);
            let name = ContentAssetName::new(format!("{:064x}.png", u8::from(!present))).unwrap();
            let (first, second) = tokio::join!(
                snapshot.read_content_asset(&name),
                snapshot.read_content_asset(&name)
            );
            assert_eq!(first.unwrap().is_some(), present);
            assert_eq!(second.unwrap().is_some(), present);
            assert_eq!(reads.load(Ordering::SeqCst), 1);
        }
        assert_eq!(snapshot.content_assets.entries.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn cancelled_content_asset_reads_release_in_flight_entries() {
        let cache = Arc::new(ContentAssetCache::default());
        let started = Arc::new(tokio::sync::Notify::new());
        let reader = {
            let cache = Arc::clone(&cache);
            let started = Arc::clone(&started);
            tokio::spawn(async move {
                cache
                    .get_or_try_init("cancelled", || async {
                        started.notify_one();
                        std::future::pending().await
                    })
                    .await
            })
        };
        started.notified().await;
        reader.abort();
        assert!(reader.await.unwrap_err().is_cancelled());
        assert!(cache.entries.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn keyed_cache_single_flights_each_key_independently() {
        let cache = KeyedCache::new();
        let loads = AtomicUsize::new(0);

        let (first, second) = tokio::join!(
            cache.get_or_try_init("same".to_string(), || async {
                loads.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
                Ok("value".to_string())
            }),
            cache.get_or_try_init("same".to_string(), || async {
                loads.fetch_add(1, Ordering::SeqCst);
                Ok("other".to_string())
            })
        );

        assert_eq!(first.unwrap(), "value");
        assert_eq!(second.unwrap(), "value");
        assert_eq!(loads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn snapshot_expires_at_ttl_boundary() {
        let (reader, snapshot_calls, _) = counting_reader(false);
        let snapshot = reader.snapshot().await.unwrap();
        let checked_at = Instant::now();
        let cached = CachedSnapshot {
            checked_at,
            snapshot,
        };

        assert!(cached.is_fresh(checked_at, Duration::from_secs(5)));
        assert!(!cached.is_fresh(checked_at + Duration::from_secs(5), Duration::from_secs(5)));
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn same_release_identity_preserves_artifact_cache_after_refresh() {
        let snapshot_calls = Arc::new(AtomicUsize::new(0));
        let article_reads = Arc::new(AtomicUsize::new(0));
        let inner: DynArtifactReader = Arc::new(CountingReader {
            snapshot_calls: Arc::clone(&snapshot_calls),
            article_reads: Arc::clone(&article_reads),
            fail_next_article_read: Arc::new(AtomicBool::new(false)),
            cache_identity: Some("release-1"),
            last_modified: None,
        });
        let reader = CachingArtifactReader::new(inner, Duration::from_millis(1));

        reader
            .snapshot()
            .await
            .unwrap()
            .read_article_index()
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await;
        reader
            .snapshot()
            .await
            .unwrap()
            .read_article_index()
            .await
            .unwrap();

        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 2);
        assert_eq!(article_reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn refresh_failure_serves_stale_immutable_release_until_next_ttl() {
        let ttl = Duration::from_millis(10);
        let (reader, snapshot_calls, article_reads) = sequenced_reader(
            [
                SnapshotOutcome::Success(Some("release-1")),
                SnapshotOutcome::Failure,
                SnapshotOutcome::Success(Some("release-1")),
            ],
            ttl,
        );

        let first = reader.snapshot().await.unwrap();
        first.read_article_index().await.unwrap();
        tokio::time::sleep(Duration::from_millis(15)).await;
        let stale = reader.snapshot().await.unwrap();
        stale.read_article_index().await.unwrap();
        let before_next_ttl = reader.snapshot().await.unwrap();

        assert!(Arc::ptr_eq(&first, &stale));
        assert!(Arc::ptr_eq(&first, &before_next_ttl));
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 2);
        assert_eq!(article_reads.load(Ordering::SeqCst), 1);

        tokio::time::sleep(Duration::from_millis(15)).await;
        let refreshed = reader.snapshot().await.unwrap();

        assert!(Arc::ptr_eq(&first, &refreshed));
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn refresh_failure_without_immutable_release_returns_error() {
        let ttl = Duration::from_millis(10);
        let (initial_failure, _, _) =
            sequenced_reader([SnapshotOutcome::Failure], Duration::from_secs(60));

        assert!(initial_failure.snapshot().await.is_err());

        let (legacy_reader, snapshot_calls, _) = sequenced_reader(
            [SnapshotOutcome::Success(None), SnapshotOutcome::Failure],
            ttl,
        );
        legacy_reader.snapshot().await.unwrap();
        tokio::time::sleep(Duration::from_millis(15)).await;

        assert!(legacy_reader.snapshot().await.is_err());
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn immutable_release_does_not_downgrade_to_legacy_root() {
        let ttl = Duration::from_millis(10);
        let (reader, snapshot_calls, _) = sequenced_reader(
            [
                SnapshotOutcome::Success(Some("release-1")),
                SnapshotOutcome::Success(None),
                SnapshotOutcome::Success(Some("release-2")),
            ],
            ttl,
        );

        let release_1 = reader.snapshot().await.unwrap();
        tokio::time::sleep(Duration::from_millis(15)).await;
        let stale = reader.snapshot().await.unwrap();

        assert!(Arc::ptr_eq(&release_1, &stale));
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 2);

        tokio::time::sleep(Duration::from_millis(15)).await;
        let release_2 = reader.snapshot().await.unwrap();

        assert!(!Arc::ptr_eq(&release_1, &release_2));
        assert_eq!(release_2.cache_identity(), Some("release-2"));
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn zero_ttl_does_not_serve_stale_release() {
        let (reader, snapshot_calls, _) = sequenced_reader(
            [
                SnapshotOutcome::Success(Some("release-1")),
                SnapshotOutcome::Failure,
            ],
            Duration::ZERO,
        );

        reader.snapshot().await.unwrap();

        assert!(reader.snapshot().await.is_err());
        assert_eq!(snapshot_calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn missing_or_different_release_identity_does_not_reuse_cache() {
        let article_reads = Arc::new(AtomicUsize::new(0));
        let snapshot = |cache_identity| -> DynArtifactSnapshot {
            Arc::new(CountingSnapshot {
                article_reads: Arc::clone(&article_reads),
                fail_next_article_read: Arc::new(AtomicBool::new(false)),
                cache_identity,
                last_modified: None,
            })
        };
        let cached = CachedSnapshot {
            checked_at: Instant::now(),
            snapshot: snapshot(Some("release-1")),
        };

        assert!(!cached.has_same_identity(&snapshot(Some("release-2"))));

        let legacy = CachedSnapshot {
            checked_at: Instant::now(),
            snapshot: snapshot(None),
        };
        assert!(!legacy.has_same_identity(&snapshot(None)));
    }
}
