use leptos::prelude::*;
use stylance::import_style;

import_style!(tag_list_style, "tag_list.module.scss");

/// タグリストを表示するコンポーネント
#[component]
pub fn TagList(#[prop(into)] tags: Vec<String>) -> impl IntoView {
    view! {
        <div class=tag_list_style::tag_list>
            {tags
                .into_iter()
                .map(|tag| {
                    let href = "/".to_string();
                    let tag_text = format!("#{tag}");
                    // let href = format!("/tags/{}", tag);
                    view! {
                        <a href=href class=tag_list_style::tag>
                            {tag_text}
                        </a>
                    }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}
