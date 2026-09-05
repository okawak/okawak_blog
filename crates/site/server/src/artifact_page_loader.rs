//! Artifact-backed implementation of the storage-independent page loader contract.

use async_trait::async_trait;
use domain::{
    ArticlePageDocument, Category, CategoryPageDocument, HomePageDocument, PageKey, Slug,
    StaticPageDocument, build_article_page_document, build_category_page_document,
    build_home_page_document, build_static_page_document, find_article_summary,
};
use infra::{DynArtifactReader, DynArtifactSnapshot};

use crate::page_loader::{PageLoadResult, PageLoader};

#[derive(Clone)]
enum ArtifactPageSource {
    Reader(DynArtifactReader),
    Snapshot(DynArtifactSnapshot),
}

#[derive(Clone)]
pub(crate) struct ArtifactPageLoader {
    source: ArtifactPageSource,
}

impl ArtifactPageLoader {
    pub(crate) fn from_reader(reader: DynArtifactReader) -> Self {
        Self {
            source: ArtifactPageSource::Reader(reader),
        }
    }

    pub(crate) fn from_snapshot(snapshot: DynArtifactSnapshot) -> Self {
        Self {
            source: ArtifactPageSource::Snapshot(snapshot),
        }
    }

    async fn snapshot(&self) -> PageLoadResult<DynArtifactSnapshot> {
        match &self.source {
            ArtifactPageSource::Reader(reader) => {
                reader.snapshot().await.map_err(|error| error.to_string())
            }
            ArtifactPageSource::Snapshot(snapshot) => Ok(snapshot.clone()),
        }
    }
}

#[async_trait]
impl PageLoader for ArtifactPageLoader {
    async fn load_home(&self) -> PageLoadResult<HomePageDocument> {
        let snapshot = self.snapshot().await?;
        let (article_index, site_metadata, home_fragment) = tokio::try_join!(
            snapshot.read_article_index(),
            snapshot.read_site_metadata(),
            async {
                match snapshot.read_home_fragment().await {
                    Ok(fragment) => Ok(Some(fragment)),
                    Err(error) if error.is_not_found() => Ok(None),
                    Err(error) => Err(error),
                }
            },
        )
        .map_err(|error| error.to_string())?;

        build_home_page_document(&article_index, &site_metadata, home_fragment.as_ref())
            .map_err(|error| error.to_string())
    }

    async fn load_article(
        &self,
        category: &Category,
        slug: &Slug,
    ) -> PageLoadResult<Option<ArticlePageDocument>> {
        let snapshot = self.snapshot().await?;
        let article_index = snapshot
            .read_article_index()
            .await
            .map_err(|error| error.to_string())?;
        let Some(summary) = find_article_summary(&article_index, category, slug) else {
            return Ok(None);
        };
        let html = match snapshot.read_article_html(category, slug).await {
            Ok(html) => html,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };

        build_article_page_document(summary, &html)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    async fn load_category(
        &self,
        category: &Category,
    ) -> PageLoadResult<Option<CategoryPageDocument>> {
        let snapshot = self.snapshot().await?;
        let artifact = match snapshot.read_category_document(category).await {
            Ok(artifact) => artifact,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };

        build_category_page_document(&artifact)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    async fn load_static_page(&self, page: &PageKey) -> PageLoadResult<Option<StaticPageDocument>> {
        let snapshot = self.snapshot().await?;
        let artifact = match snapshot.read_page_document(page).await {
            Ok(artifact) => artifact,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };

        build_static_page_document(&artifact)
            .map(Some)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::pending,
        sync::{
            Arc,
            atomic::{AtomicU8, Ordering},
        },
        task::{Context, Waker},
    };

    use domain::{
        ArticleIndexDocument, CategoryArtifactDocument, HomeFragmentArtifactDocument,
        PageArtifactDocument, SiteMetadataDocument,
    };
    use infra::{ArtifactSnapshot, Result};

    use super::*;

    /// Each home read records that it started, then stays pending. This makes
    /// sequential reads distinguishable without sleeps or a real S3 connection.
    struct PendingHomeSnapshot(Arc<AtomicU8>);

    #[async_trait]
    impl ArtifactSnapshot for PendingHomeSnapshot {
        async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
            self.0.fetch_or(1, Ordering::SeqCst);
            pending().await
        }

        async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
            self.0.fetch_or(2, Ordering::SeqCst);
            pending().await
        }

        async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
            self.0.fetch_or(4, Ordering::SeqCst);
            pending().await
        }

        async fn read_category_document(&self, _: &Category) -> Result<CategoryArtifactDocument> {
            panic!("home must not read a category document")
        }

        async fn read_article_html(&self, _: &Category, _: &Slug) -> Result<String> {
            panic!("home must not read article HTML")
        }

        async fn read_page_document(&self, _: &PageKey) -> Result<PageArtifactDocument> {
            panic!("home must not read a static page")
        }
    }

    #[test]
    fn home_starts_all_independent_reads_before_waiting_for_results() {
        let started = Arc::new(AtomicU8::new(0));
        let loader =
            ArtifactPageLoader::from_snapshot(Arc::new(PendingHomeSnapshot(started.clone())));
        let mut load = loader.load_home();
        let mut cx = Context::from_waker(Waker::noop());

        assert!(load.as_mut().poll(&mut cx).is_pending());
        assert_eq!(
            started.load(Ordering::SeqCst),
            7,
            "all three home reads must start"
        );
    }
}
