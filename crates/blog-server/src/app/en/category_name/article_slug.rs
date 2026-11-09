use super::category_name;
use topcoat::{
    Result,
    context::Cx,
    router::{module_param, page},
    view::View,
};
module_param!(article_slug);

#[page]
async fn article(cx: &Cx) -> Result<impl View> {
    crate::app::category_name::article_slug::render_article(
        cx,
        blog_domain::Locale::En,
        category_name(cx),
        article_slug(cx),
    )
    .await
}
