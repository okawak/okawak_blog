//! Storage-independent port for loading blog-domain page documents.

use std::sync::Arc;

use async_trait::async_trait;
use blog_domain::{
    ArticlePageDocument, Category, CategoryPageDocument, ContentAssetName, HomePageDocument,
    Locale, PageKey, SiteLocalesDocument, Slug, StaticPageDocument, TagLabels,
};

pub(crate) type PageLoadResult<T> = Result<T, String>;

#[async_trait]
pub(crate) trait PageLoader: Send + Sync {
    async fn load_locales(&self) -> PageLoadResult<Arc<SiteLocalesDocument>>;

    async fn load_asset(&self, name: &ContentAssetName) -> PageLoadResult<Option<bytes::Bytes>>;

    async fn load_home(
        &self,
        locale: Locale,
    ) -> PageLoadResult<Option<Presentation<HomePageDocument>>>;

    async fn load_article(
        &self,
        locale: Locale,
        category: &Category,
        slug: &Slug,
    ) -> PageLoadResult<Option<Presentation<ArticlePageDocument>>>;

    async fn load_category(
        &self,
        locale: Locale,
        category: &Category,
    ) -> PageLoadResult<Option<Presentation<CategoryPageDocument>>>;

    async fn load_static_page(
        &self,
        locale: Locale,
        page: &PageKey,
    ) -> PageLoadResult<Option<Presentation<StaticPageDocument>>>;
}

#[derive(Clone)]
pub(crate) struct Presentation<T> {
    pub(crate) document: T,
    pub(crate) labels: Arc<TagLabels>,
    pub(crate) locales: Arc<SiteLocalesDocument>,
}

pub(crate) type DynPageLoader = Arc<dyn PageLoader>;
