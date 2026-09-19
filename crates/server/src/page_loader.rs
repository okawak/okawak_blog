//! Storage-independent port for loading domain page documents.

use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    ArticlePageDocument, Category, CategoryPageDocument, ContentAssetName, HomePageDocument,
    Locale, PageKey, SiteLocalesDocument, Slug, StaticPageDocument, TagLabels,
};

pub(crate) type PageLoadResult<T> = Result<T, String>;

#[async_trait]
pub(crate) trait PageLoader: Send + Sync {
    async fn load_asset(&self, name: &ContentAssetName) -> PageLoadResult<Option<Vec<u8>>>;

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
    pub(crate) labels: TagLabels,
    pub(crate) locales: SiteLocalesDocument,
}

pub(crate) type DynPageLoader = Arc<dyn PageLoader>;

#[derive(Clone)]
pub(crate) struct PageLoaderContext(DynPageLoader);

impl PageLoaderContext {
    pub(crate) fn new(loader: DynPageLoader) -> Self {
        Self(loader)
    }

    pub(crate) fn loader(&self) -> &dyn PageLoader {
        self.0.as_ref()
    }
}
