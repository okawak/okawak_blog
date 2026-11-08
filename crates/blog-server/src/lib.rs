#![warn(unreachable_pub)]

//! Topcoat site application, runtime composition, and HTTP boundary.

mod article_card;
mod article_filter;
mod artifact_page_loader;
mod category_articles;

// Topcoat UI vendors public component APIs into an application-private module.
#[allow(unreachable_pub)]
mod components;

pub mod app;

pub mod assets;

mod format;

mod icons;

pub mod http_cache;

mod page_loader;

mod shell;

mod i18n;
mod language;
mod metadata;
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
    fn site_origin_uses_nonempty_override_or_production_default() {
        const EXPECTED: &str = "BLOG_TEST_EXPECTED_ORIGIN";
        if let Ok(expected) = std::env::var(EXPECTED) {
            assert_eq!(resolved_site_origin(), expected);
            return;
        }
        // Isolate environment changes from concurrent router and metadata tests.
        for (origin, expected) in [
            (None, SITE_ORIGIN),
            (Some(""), SITE_ORIGIN),
            (Some("https://preview.example"), "https://preview.example"),
        ] {
            let mut child = std::process::Command::new(std::env::current_exe().unwrap());
            child
                .args([
                    "--exact",
                    "tests::site_origin_uses_nonempty_override_or_production_default",
                ])
                .env(EXPECTED, expected)
                .env_remove(SITE_ORIGIN_ENV);
            if let Some(origin) = origin {
                child.env(SITE_ORIGIN_ENV, origin);
            }
            let output = child.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }

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
