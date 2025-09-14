use domain::ArticleSummary;
use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
use stylance::import_style;

import_style!(sidebar_style, "sidebar.module.scss");

/// 特定のカテゴリの記事全てを取得するサーバー関数
#[server]
pub async fn get_all_articles(category: String) -> Result<Vec<ArticleSummary>, ServerFnError> {
    // TODO: coreクレートのユースケースを使用して記事を取得
    let _ = category; // 未使用警告を回避
    Ok(vec![])
}

/// 記事リストを生成する関数
fn render_sidebar(articles: Vec<ArticleSummary>) -> impl IntoView {
    let params = use_params_map();
    let slug = move || params.with(|p: &ParamsMap| p.get("slug").clone().unwrap_or_default());

    view! {
        <div class=sidebar_style::sidebar_group>
            <h3 class=sidebar_style::group_title>記事一覧</h3>
            <ul class=sidebar_style::group_articles>
                {articles
                    .into_iter()
                    .map(|article| {
                        let link = format!("/{}/{}", article.category, article.slug);
                        let link_style = if slug() == article.slug.to_string() {
                            sidebar_style::article_link_active
                        } else {
                            sidebar_style::article_link
                        };
                        view! {
                            <li class=link_style>
                                <a href=link>{article.title.to_string()}</a>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        </div>
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
