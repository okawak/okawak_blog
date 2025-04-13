//! ブログUIのコンポーネントモジュール
//!
//! このモジュールには、ブログの様々な部分で使用されるUIコンポーネントが含まれています。
//! ヘッダー、フッター、サイドバーなどの共通要素はここで定義されています。

// サブモジュールを公開
pub mod article_card;
pub mod footer;
pub mod header;
pub mod markdown_renderer;
pub mod sidebar;

// 頻繁に使われるコンポーネントを再エクスポート
pub use article_card::ArticleCard;
pub use footer::Footer;
pub use header::Header;
pub use markdown_renderer::MarkdownRenderer;
pub use sidebar::Sidebar;

// コンポーネント間で共有される共通の型や定数
#[derive(Clone, Debug, PartialEq)]
pub struct NavigationItem {
    pub title: String,
    pub href: String,
    pub is_active: bool,
}

/// メインナビゲーションリンク
pub fn get_main_nav_items(current_path: &str) -> Vec<NavigationItem> {
    vec![
        NavigationItem {
            title: "ホーム".into(),
            href: "/".into(),
            is_active: current_path == "/",
        },
        NavigationItem {
            title: "統計学".into(),
            href: "/statistics".into(),
            is_active: current_path == "/statistics" || current_path.starts_with("/statistics/"),
        },
        NavigationItem {
            title: "物理学".into(),
            href: "/physics".into(),
            is_active: current_path == "/physics" || current_path.starts_with("/physics/"),
        },
        NavigationItem {
            title: "日常".into(),
            href: "/daily".into(),
            is_active: current_path == "/daily" || current_path.starts_with("/daily/"),
        },
        NavigationItem {
            title: "技術".into(),
            href: "/tech".into(),
            is_active: current_path == "/tech" || current_path.starts_with("/tech/"),
        },
    ]
}

/// SNSリンク
pub fn get_social_links() -> Vec<NavigationItem> {
    vec![
        NavigationItem {
            title: "GitHub".into(),
            href: "https://github.com/okawak".into(),
            is_active: false,
        },
        NavigationItem {
            title: "Twitter".into(),
            href: "https://twitter.com/okawak_".into(),
            is_active: false,
        },
    ]
}
