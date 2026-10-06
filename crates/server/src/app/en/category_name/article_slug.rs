use super::CategoryName;
use topcoat::{
    Result,
    context::Cx,
    router::{module_param, page, path_param},
    view::View,
};
module_param!(article_slug);

#[page]
async fn article(cx: &Cx) -> Result<impl View> {
    crate::app::category_name::article_slug::render_article(
        cx,
        domain::Locale::En,
        path_param::<CategoryName>(cx),
        path_param::<ArticleSlug>(cx),
    )
    .await
}
