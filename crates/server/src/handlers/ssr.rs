//! SSR Handlers - Leptos Server-Side Rendering統合

use axum::Router;

/// SSR ルーターを作成
pub fn create_ssr_router() -> Router {
    // 将来実装: Leptos SSR統合
    // Router::new()
    //     .leptos_routes(&leptos_options, web::app_routes, web::App)
    //     .fallback(web::file_and_error_handler)

    // 一時的なプレースホルダー
    Router::new()
}