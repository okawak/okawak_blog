use crate::models::article::ArticleSummary;
#[cfg(feature = "ssr")]
use crate::services::s3;

use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
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

    let params = use_params_map();
    let slug = move || params.with(|p: &ParamsMap| p.get("slug").clone().unwrap_or_default());

    view! {
        <>
            {grouped
                .into_iter()
                .map(|(group_name, mut articles)| {
                    articles.sort_by(|a, b| b.priority_level.cmp(&a.priority_level));

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
                                        let link_style = if slug() == article.slug {
                                            sidebar_style::article_link_active
                                        } else {
                                            sidebar_style::article_link
                                        };
                                        view! {
                                            <li class=link_style>
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
                <Suspense fallback=|| {
                    view! { <div class=sidebar_style::loading>"記事を読み込み中..."</div> }
                }>
                    <ErrorBoundary fallback=|error| {
                        view! {
                            <div class=sidebar_style::error>
                                "記事の読み込みに失敗しました: "
                                {format!("{error:?}")}
                            </div>
                        }
                    }>
                        <Show
                            when=move || matches!(article_summaries.get(), Some(Ok(_)))
                            fallback=move || {
                                view! {
                                    <p class=sidebar_style::no_articles>
                                        {"記事がありません。"}
                                    </p>
                                }
                            }
                        >
                            {move || {
                                article_summaries.get().and_then(Result::ok).map(render_sidebar)
                            }}
                        </Show>
                    </ErrorBoundary>
                </Suspense>
            </div>
        </aside>
    }
}
