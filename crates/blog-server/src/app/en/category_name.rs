mod article_slug;
use topcoat::{
    Result,
    context::Cx,
    router::{module_param, page},
    view::View,
};
module_param!(category_name: blog_domain::Category else not_found);

#[page]
async fn category(cx: &Cx) -> Result<impl View> {
    crate::app::category_name::render_category(cx, blog_domain::Locale::En, *category_name(cx)?)
        .await
}
