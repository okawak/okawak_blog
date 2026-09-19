mod article_slug;
use topcoat::{
    Result,
    context::Cx,
    router::{page, path_param},
    view::View,
};
path_param!(category_name);

#[page]
async fn category(cx: &Cx) -> Result<impl View> {
    crate::app::category_name::render_category(
        cx,
        domain::Locale::En,
        path_param::<CategoryName>(cx),
    )
    .await
}
