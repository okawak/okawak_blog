use topcoat::{Result, context::Cx, router::page, view::View};

#[page]
async fn about(cx: &Cx) -> Result<impl View> {
    crate::app::about::render_about(cx, domain::Locale::En).await
}
