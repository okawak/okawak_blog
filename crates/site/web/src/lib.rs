#![recursion_limit = "512"]

pub mod app;
pub mod components;
pub mod error;
pub mod routes;
pub mod types; // Web-specific type definitions.

pub const SITE_NAME: &str = "ぶくせんの探窟メモ";
pub const SITE_ORIGIN: &str = "https://www.okawak.net";
#[cfg(not(target_arch = "wasm32"))]
const SITE_ORIGIN_ENV: &str = "OKAWAK_BLOG_SITE_ORIGIN";

// Re-export functions and types used on the server side.
pub use app::{App, shell};
pub use error::FrontendError;

pub fn build_site_url(path: &str) -> String {
    join_site_url(&resolved_site_origin(), path)
}

fn resolved_site_origin() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|window| window.location().origin().ok())
            .filter(|origin| !origin.is_empty())
            .unwrap_or_else(|| SITE_ORIGIN.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(SITE_ORIGIN_ENV)
            .ok()
            .filter(|origin| !origin.is_empty())
            .unwrap_or_else(|| SITE_ORIGIN.to_string())
    }
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

// Client-side hydration entry point.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    // Forward panic output to the browser console.
    console_error_panic_hook::set_once();
    // Hydrate the body using the App component.
    leptos::mount::hydrate_body(App);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_site_url_normalizes_slashes() {
        assert_eq!(
            join_site_url("https://example.com/", "/articles/intro"),
            "https://example.com/articles/intro"
        );
        assert_eq!(
            join_site_url("https://example.com", "categories/tech"),
            "https://example.com/categories/tech"
        );
        assert_eq!(join_site_url("https://example.com/", "/"), "https://example.com");
    }
}
