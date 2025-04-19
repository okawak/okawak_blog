use leptos::prelude::*;

/// タグリストを表示するコンポーネント
#[component]
pub fn TagList(#[prop(into)] tags: Vec<String>) -> impl IntoView {
    view! {
        <div class="tag-list">
            {tags
                .into_iter()
                .map(|tag| {
                    let href = format!("/tags/{}", tag);
                    let tag_text = format!("#{}", tag);
                    view! {
                        <a href=href class="tag">
                            {tag_text}
                        </a>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}
