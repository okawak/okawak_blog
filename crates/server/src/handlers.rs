//! Handlers - HTTP ハンドラーと Leptos 統合

// 新しいモジュール構造（mod.rsを使わない方式）
pub mod api;
pub mod ssr;
pub mod static_files;

// 既存ハンドラー（段階的に移行）
pub mod blog_handlers;
pub mod leptos_integration;

// Re-exports
pub use api::{AppState, create_api_router};
pub use blog_handlers::create_blog_router;
pub use leptos_integration::*;
pub use ssr::*;
pub use static_files::*;
