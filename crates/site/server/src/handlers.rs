//! Handlers - HTTP ハンドラーと Leptos 統合

pub mod api;
pub mod ssr;
pub mod static_files;

pub use api::create_api_router;
pub use ssr::*;
pub use static_files::*;
