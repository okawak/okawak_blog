//! Public SSR routes and page-specific components.

mod article;
mod category;
#[path = "pages/home.rs"]
mod home_page;
mod page;

use topcoat::context::{Cx, app_context, try_request_context};

use crate::page_loader::PageLoaderContext;

pub use article::article_page;
pub use category::category_page;
pub use home_page::home;
pub use page::about;

fn page_loader(cx: &Cx) -> &PageLoaderContext {
    try_request_context::<PageLoaderContext>(cx)
        .unwrap_or_else(|| app_context::<PageLoaderContext>(cx))
}
