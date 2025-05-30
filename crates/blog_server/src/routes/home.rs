use crate::components::ArticleCard;
use crate::models::article::ArticleSummary;
#[cfg(feature = "ssr")]
use crate::services::s3;
use leptos::prelude::*;
use reactive_stores::Store;
use stylance::import_style;

import_style!(home_style, "home.module.scss");

/// 全てのカテゴリの記事一覧を取得するサーバー関数
#[server]
pub async fn get_latest_articles() -> Result<Vec<ArticleSummary>, ServerFnError> {
    s3::fetch_latest_articles(String::new(), 10)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[derive(Store, Clone)]
pub struct ArticlesData {
    #[store(key: String = |article: &ArticleSummary| article.id.clone())]
    rows: Vec<ArticleSummary>,
}

/// ホームページコンポーネント
#[component]
pub fn HomePage() -> impl IntoView {
    // 最新記事一覧を取得
    let latest_articles = Resource::<Result<Vec<ArticleSummary>, String>>::new(
        || (),
        move |_| {
            async move {
                // SSR/CSR 両方で get_latest_articles を呼ぶ
                get_latest_articles().await.map_err(|e| e.to_string())
            }
        },
    );

    let articles_store = Store::new(ArticlesData { rows: vec![] });
    Effect::new(move |_| {
        if let Some(Ok(articles)) = latest_articles.get() {
            let rows = articles_store.rows();
            *rows.write() = articles.clone();
        }
    });

    view! {
        <div class=home_style::home_page>
            <section class=home_style::profile_section>
                <h1>{"ホーム"}</h1>
                <div class=home_style::profile_text>
                    <p>{"気になったことをメモしておくブログです。"}</p>
                </div>
            </section>

            <section class=home_style::latest_articles>
                <h2>{"最近の記事"}</h2>
                <Suspense fallback=|| {
                    view! { <div class=home_style::loading>"記事を読み込み中..."</div> }
                }>
                    <ErrorBoundary fallback=|error| {
                        view! {
                            <div class=home_style::error>
                                "記事の読み込みに失敗しました: "
                                {format!("{error:?}")}
                            </div>
                        }
                    }>
                        <Show
                            when=move || {
                                matches!(
                                    latest_articles.get(),
                                    Some(Ok(articles))
                                    if !articles.is_empty()
                                )
                            }
                            fallback=|| {
                                view! {
                                    <div class=home_style::no_articles>
                                        "記事がありません"
                                    </div>
                                }
                            }
                        >
                            <div class=home_style::article_list>
                                <For
                                    each=move || articles_store.rows()
                                    key=|entry| entry.read().id.clone()
                                    children=move |entry| {
                                        view! { <ArticleCard article=entry.read().clone() /> }
                                    }
                                />

                            </div>
                        </Show>
                    </ErrorBoundary>
                </Suspense>
            </section>
        </div>
    }
}

// WASM（hydrate）用 スタブ：型だけ合わせておく
#[cfg(not(feature = "ssr"))]
#[allow(dead_code)]
async fn fetch_latest_articles() -> Result<Vec<ArticleSummary>, String> {
    // クライアントナビゲーション時に呼び出されても型エラーにならないよう、
    // 空リスト or 適当なエラーを返す
    Ok(vec![])
}
