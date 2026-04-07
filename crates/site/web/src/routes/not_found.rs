use leptos::prelude::*;

/// 404 page component.
/// Rendered when the requested URL does not exist.
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! { <div>{"ページが見つかりませんでした。"}</div> }
}
