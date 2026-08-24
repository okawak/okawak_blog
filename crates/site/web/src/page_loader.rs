//! Storage-independent port for loading domain page documents.

use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    ArticlePageDocument, Category, CategoryPageDocument, HomePageDocument, PageKey, Slug,
    StaticPageDocument,
};

pub type PageLoadResult<T> = Result<T, String>;

#[async_trait]
pub trait PageLoader: Send + Sync {
    async fn load_home(&self) -> PageLoadResult<HomePageDocument>;

    async fn load_article(
        &self,
        category: &Category,
        slug: &Slug,
    ) -> PageLoadResult<Option<ArticlePageDocument>>;

    async fn load_category(
        &self,
        category: &Category,
    ) -> PageLoadResult<Option<CategoryPageDocument>>;

    async fn load_static_page(&self, page: &PageKey) -> PageLoadResult<Option<StaticPageDocument>>;
}

pub type DynPageLoader = Arc<dyn PageLoader>;

#[derive(Clone)]
pub struct PageLoaderContext(DynPageLoader);

impl PageLoaderContext {
    pub fn new(loader: DynPageLoader) -> Self {
        Self(loader)
    }

    pub fn loader(&self) -> &dyn PageLoader {
        self.0.as_ref()
    }
}
