use crate::{ArtifactReader, ArtifactSnapshot, DynArtifactReader, DynArtifactSnapshot, Result};
use async_trait::async_trait;
use domain::{
    ArticleIndexDocument, Category, CategoryIndexDocument, PageArtifactDocument, PageKey,
    SiteMetadataDocument, Slug,
};
use std::{
    collections::HashMap,
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, OnceCell};

pub struct CachingArtifactReader {
    inner: DynArtifactReader,
    snapshot_ttl: Duration,
    cached: Mutex<Option<CachedSnapshot>>,
}

struct CachedSnapshot {
    loaded_at: Instant,
    snapshot: DynArtifactSnapshot,
}

impl CachedSnapshot {
    fn is_fresh(&self, now: Instant, ttl: Duration) -> bool {
        now.duration_since(self.loaded_at) < ttl
    }

    fn has_same_identity(&self, candidate: &DynArtifactSnapshot) -> bool {
        self.snapshot
            .cache_identity()
            .is_some_and(|identity| Some(identity) == candidate.cache_identity())
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

        let inner_snapshot = self.inner.snapshot().await?;
        if let Some(cached) = cached.as_mut()
            && cached.has_same_identity(&inner_snapshot)
        {
            cached.loaded_at = Instant::now();
            return Ok(Arc::clone(&cached.snapshot));
        }

        let snapshot: DynArtifactSnapshot = Arc::new(CachingArtifactSnapshot::new(inner_snapshot));
        *cached = Some(CachedSnapshot {
            loaded_at: Instant::now(),
            snapshot: Arc::clone(&snapshot),
        });
        Ok(snapshot)
    }
}

struct CachingArtifactSnapshot {
    inner: DynArtifactSnapshot,
    article_index: OnceCell<ArticleIndexDocument>,
    site_metadata: OnceCell<SiteMetadataDocument>,
    category_indexes: KeyedCache<CategoryIndexDocument>,
    category_html: KeyedCache<String>,
    article_html: KeyedCache<String>,
    page_documents: KeyedCache<PageArtifactDocument>,
}

impl CachingArtifactSnapshot {
    fn new(inner: DynArtifactSnapshot) -> Self {
        Self {
            inner,
            article_index: OnceCell::new(),
            site_metadata: OnceCell::new(),
            category_indexes: KeyedCache::new(),
            category_html: KeyedCache::new(),
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

    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.article_index
            .get_or_try_init(|| self.inner.read_article_index())
            .await
            .cloned()
    }

    async fn read_category_index(&self, category: &str) -> Result<CategoryIndexDocument> {
        self.category_indexes
            .get_or_try_init(category.to_string(), || {
                self.inner.read_category_index(category)
            })
            .await
    }

    async fn read_category_html(&self, category: &Category) -> Result<String> {
        self.category_html
            .get_or_try_init(category.as_str().to_string(), || {
                self.inner.read_category_html(category)
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

    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
        self.page_documents
            .get_or_try_init(page.as_str().to_string(), || {
                self.inner.read_page_document(page)
            })
            .await
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
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct CountingReader {
        snapshot_calls: Arc<AtomicUsize>,
        article_reads: Arc<AtomicUsize>,
        fail_next_article_read: Arc<AtomicBool>,
        cache_identity: Option<&'static str>,
    }

    #[async_trait]
    impl ArtifactReader for CountingReader {
        async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
            self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(CountingSnapshot {
                article_reads: Arc::clone(&self.article_reads),
                fail_next_article_read: Arc::clone(&self.fail_next_article_read),
                cache_identity: self.cache_identity,
            }))
        }
    }

    struct CountingSnapshot {
        article_reads: Arc<AtomicUsize>,
        fail_next_article_read: Arc<AtomicBool>,
        cache_identity: Option<&'static str>,
    }

    #[async_trait]
    impl ArtifactSnapshot for CountingSnapshot {
        fn cache_identity(&self) -> Option<&str> {
            self.cache_identity
        }

        async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
            self.article_reads.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            if self.fail_next_article_read.swap(false, Ordering::SeqCst) {
                return Err(InfraError::Io(std::io::Error::other("temporary failure")));
            }
            Ok(ArticleIndexDocument { articles: vec![] })
        }

        async fn read_category_index(&self, category: &str) -> Result<CategoryIndexDocument> {
            Ok(CategoryIndexDocument {
                category: category.to_string(),
                title: None,
                description: None,
                updated_at: None,
                articles: vec![],
            })
        }

        async fn read_category_html(&self, category: &Category) -> Result<String> {
            Ok(category.as_str().to_string())
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
        });
        (
            CachingArtifactReader::new(inner, Duration::from_secs(60)),
            snapshot_calls,
            article_reads,
        )
    }

    #[tokio::test]
    async fn reuses_snapshot_and_single_flights_concurrent_artifact_reads() {
        let (reader, snapshot_calls, article_reads) = counting_reader(false);
        let first = reader.snapshot().await.unwrap();
        let second = reader.snapshot().await.unwrap();

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
        let loaded_at = Instant::now();
        let cached = CachedSnapshot {
            loaded_at,
            snapshot,
        };

        assert!(cached.is_fresh(loaded_at, Duration::from_secs(5)));
        assert!(!cached.is_fresh(loaded_at + Duration::from_secs(5), Duration::from_secs(5)));
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

    #[test]
    fn missing_or_different_release_identity_does_not_reuse_cache() {
        let article_reads = Arc::new(AtomicUsize::new(0));
        let snapshot = |cache_identity| -> DynArtifactSnapshot {
            Arc::new(CountingSnapshot {
                article_reads: Arc::clone(&article_reads),
                fail_next_article_read: Arc::new(AtomicBool::new(false)),
                cache_identity,
            })
        };
        let cached = CachedSnapshot {
            loaded_at: Instant::now(),
            snapshot: snapshot(Some("release-1")),
        };

        assert!(!cached.has_same_identity(&snapshot(Some("release-2"))));

        let legacy = CachedSnapshot {
            loaded_at: Instant::now(),
            snapshot: snapshot(None),
        };
        assert!(!legacy.has_same_identity(&snapshot(None)));
    }
}
