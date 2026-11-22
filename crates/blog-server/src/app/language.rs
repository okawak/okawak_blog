mod about;
mod category_name;

use topcoat::{Result, context::Cx, router::page, view::View};

topcoat::router::module_param!(language: blog_domain::Locale else not_found);

#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    super::render_home(cx, *language(cx)?).await
}
