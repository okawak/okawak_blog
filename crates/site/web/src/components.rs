//! Shared blog UI components.
//!
//! Common UI elements such as the header, footer, and sidebar live here.

// Public submodules.
pub mod footer;
pub mod header;
pub mod sidebar;

// Re-export frequently used components.
pub use footer::Footer;
pub use header::Header;
pub use sidebar::Sidebar;

// Shared types and constants used across components.
#[derive(Clone, Debug, PartialEq)]
pub struct NavigationItem {
    pub title: String,
    pub href: String,
    pub is_active: bool,
}

/// Main navigation items.
pub fn get_main_nav_items(current_path: &str) -> Vec<NavigationItem> {
    vec![
        NavigationItem {
            title: "ホーム".into(),
            href: "/".into(),
            is_active: current_path == "/",
        },
        NavigationItem {
            title: "About".into(),
            href: "/about".into(),
            is_active: current_path == "/about",
        },
    ]
}

/// Social links.
pub fn get_social_links() -> Vec<NavigationItem> {
    vec![NavigationItem {
        title: "GitHub".into(),
        href: "https://github.com/okawak".into(),
        is_active: false,
    }]
}
