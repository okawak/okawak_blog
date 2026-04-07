//! SSR handlers.

use axum::Router;

/// Builds the SSR router.
pub fn create_ssr_router() -> Router {
    // Future implementation: full Leptos SSR integration.
    // Router::new()
    //     .leptos_routes(&leptos_options, web::app_routes, web::App)
    //     .fallback(web::file_and_error_handler)

    // Temporary placeholder.
    Router::new()
}
