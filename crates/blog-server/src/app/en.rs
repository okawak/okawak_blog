mod about;
mod category_name;

use topcoat::{Result, context::Cx, router::page, view::View};

#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    super::render_home(cx, blog_domain::Locale::En).await
}
