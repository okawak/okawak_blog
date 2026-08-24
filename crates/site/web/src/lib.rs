extern crate self as web;

mod article_card;
pub mod assets;
mod content_enhancement;
mod format;
mod page_loader;
pub mod pages;
mod shell;

pub use page_loader::{DynPageLoader, PageLoadResult, PageLoader, PageLoaderContext};

pub(crate) const SITE_NAME: &str = "ぶくせんの探窟メモ";
pub(crate) const SITE_ORIGIN: &str = "https://www.okawak.net";
const SITE_ORIGIN_ENV: &str = "OKAWAK_BLOG_SITE_ORIGIN";

pub(crate) fn build_site_url(path: &str) -> String {
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
