use crate::models::article::ArticleSummary;
#[cfg(feature = "ssr")]
use crate::services::s3;

use leptos::prelude::*;
use std::collections::HashMap;
use stylance::import_style;

import_style!(sidebar_style, "sidebar.module.scss");

/// 特定のカテゴリの記事全てを取得するサーバー関数
#[server]
pub async fn get_all_articles(category: String) -> Result<Vec<ArticleSummary>, ServerFnError> {
    s3::list_articles(category)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

/// groupごとに整理してViewを生成する関数
fn render_sidebar(articles: Vec<ArticleSummary>) -> impl IntoView {
    let mut grouped: HashMap<String, Vec<ArticleSummary>> = HashMap::new();
    for article in articles {
        grouped
            .entry(article.group.clone())
            .or_default()
            .push(article);
    }

    view! {
        <>
            {grouped
                .into_iter()
                .map(|(group_name, mut articles)| {
                    articles.sort_by_key(|a| a.priority_level);

                    view! {
                        <div class=sidebar_style::sidebar_group>
                            <h3 class=sidebar_style::group_title>{group_name}</h3>
                            <ul class=sidebar_style::group_articles>
                                {articles
                                    .into_iter()
                                    .map(|article| {
                                        let link = format!(
                                            "/{}/{}",
                                            article.category,
                                            article.slug,
                                        );
                                        view! {
                                            <li class=sidebar_style::article_link>
                                                <a href=link>{article.title}</a>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        </div>
                    }
                })
                .collect_view()}
        </>
    }
}

/// サイドバーコンポーネント
#[component]
pub fn Sidebar(category: &'static str, class: &'static str) -> impl IntoView {
    let article_summaries = Resource::<Result<Vec<ArticleSummary>, String>>::new(
        || (),
        move |_| async move {
            get_all_articles(category.to_string())
                .await
                .map_err(|e| e.to_string())
        },
    );

    view! {
        <aside class=move || { format!("{} {}", class, sidebar_style::sidebar) }>
            <div class=sidebar_style::sidebar_section>
                <Show
                    when=move || matches!(article_summaries.get(), Some(Ok(_)))
                    fallback=move || {
                        view! {
                            <p class=sidebar_style::error>
                                {"記事の取得に失敗しました。"}
                            </p>
                        }
                    }
                >
                    {move || { article_summaries.get().and_then(Result::ok).map(render_sidebar) }}
                </Show>
            </div>
        </aside>
    }
}
