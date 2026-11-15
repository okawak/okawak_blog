use std::{sync::Arc, time::SystemTime};

use async_trait::async_trait;
use blog_domain::{
    ArticleIndexDocument, ArticleSummaryDocument, Category, CategoryArtifactDocument,
    ContentAssetName, HomeFragmentArtifactDocument, Locale, PageArtifactDocument, PageKey,
    SiteLocalesDocument, SiteMetadataDocument, Slug,
};

use crate::Result;

pub type DynArtifactReader = Arc<dyn ArtifactReader>;
pub type DynArtifactSnapshot = Arc<dyn ArtifactSnapshot>;

#[async_trait]
pub trait ArtifactReader: Send + Sync {
    async fn snapshot(&self) -> Result<DynArtifactSnapshot>;
}

#[async_trait]
pub trait ArtifactSnapshot: Send + Sync {
    /// Returned artifacts are immutable and may share storage across requests.
    /// Cached snapshots override this lookup with a release-local index.
    async fn read_article_summary(
        &self,
        category: &Category,
        slug: &Slug,
    ) -> Result<Option<ArticleSummaryDocument>> {
        let index = self.read_article_index().await?;
        Ok(blog_domain::find_article_summary(&index, category, slug).cloned())
    }

    /// Optional locale selection, within this exact release snapshot.
    async fn localized(&self, _locale: Locale) -> Result<Option<DynArtifactSnapshot>> {
        Ok(None)
    }

    /// Older releases have no locale catalog and remain Japanese-only.
    async fn read_locales(&self) -> Result<Arc<SiteLocalesDocument>> {
        Ok(Arc::new(SiteLocalesDocument::default()))
    }

    /// Labels are part of the content release; legacy releases use original IDs.
    async fn read_tag_labels(&self) -> Result<Arc<blog_domain::TagLabels>> {
        Ok(Default::default())
    }

    async fn read_content_asset(&self, _name: &ContentAssetName) -> Result<Option<bytes::Bytes>> {
        Ok(None)
    }

    fn cache_identity(&self) -> Option<&str> {
        None
    }

    fn last_modified(&self) -> Option<SystemTime> {
        None
    }

    async fn read_article_index(&self) -> Result<Arc<ArticleIndexDocument>>;
    async fn read_category_document(
        &self,
        category: &Category,
    ) -> Result<Arc<CategoryArtifactDocument>>;
    async fn read_site_metadata(&self) -> Result<Arc<SiteMetadataDocument>>;
    async fn read_article_html(&self, category: &Category, slug: &Slug) -> Result<Arc<String>>;
    async fn read_home_fragment(&self) -> Result<Arc<HomeFragmentArtifactDocument>>;
    async fn read_page_document(&self, page: &PageKey) -> Result<Arc<PageArtifactDocument>>;
}
