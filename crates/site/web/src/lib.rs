extern crate self as web;

pub mod format;
pub mod generated_content;
pub mod topcoat_pages;

use std::sync::Arc;

use async_trait::async_trait;
use domain::{
    ArticlePageDocument, Category, CategoryPageDocument, HomePageDocument, PageKey, Slug,
    StaticPageDocument,
};

pub const SITE_NAME: &str = "ぶくせんの探窟メモ";
pub const SITE_ORIGIN: &str = "https://www.okawak.net";
const SITE_ORIGIN_ENV: &str = "OKAWAK_BLOG_SITE_ORIGIN";

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
pub struct PageLoaderContext(pub DynPageLoader);

pub fn build_site_url(path: &str) -> String {
    join_site_url(&resolved_site_origin(), path)
}

fn resolved_site_origin() -> String {
    std::env::var(SITE_ORIGIN_ENV)
        .ok()
        .filter(|origin| !origin.is_empty())
        .unwrap_or_else(|| SITE_ORIGIN.to_string())
}

fn join_site_url(origin: &str, path: &str) -> String {
    let normalized_origin = origin.trim_end_matches('/');
    let normalized_path = path.trim_start_matches('/');

    if normalized_path.is_empty() {
        normalized_origin.to_string()
    } else {
        format!("{normalized_origin}/{normalized_path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_site_url_normalizes_slashes() {
        assert_eq!(
            join_site_url("https://example.com/", "/tech/intro"),
            "https://example.com/tech/intro"
        );
        assert_eq!(
            join_site_url("https://example.com", "tech"),
            "https://example.com/tech"
        );
        assert_eq!(
            join_site_url("https://example.com/", "/"),
            "https://example.com"
        );
    }
}
