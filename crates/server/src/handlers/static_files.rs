//! Static File Handlers - 静的ファイル配信

use axum::{Router, routing::get_service};
use tower_http::services::ServeDir;

/// 静的ファイル配信ルーターを作成
pub fn create_static_router() -> Router {
    Router::new()
        // Leptos生成のJS/WASM/CSSファイル
        .nest_service("/pkg", get_service(ServeDir::new("target/site/pkg")))
        // その他の静的アセット
        .nest_service("/assets", get_service(ServeDir::new("target/site/assets")))
}
