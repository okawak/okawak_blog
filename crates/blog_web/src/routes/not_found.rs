use leptos::prelude::*;

/// 404 Not Found ページコンポーネント
/// URLが存在しない場合に表示されるページ
#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! { <div>{"ページが見つかりませんでした。"}</div> }
}
