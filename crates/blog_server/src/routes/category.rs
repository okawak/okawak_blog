use crate::components::{ArticleCard, Sidebar};
use crate::models::article::ArticleSummary;
#[cfg(feature = "ssr")]
use crate::services::s3;
use leptos::prelude::*;
use reactive_stores::Store;
use stylance::import_style;

import_style!(layout_style, "layout.module.scss");
import_style!(category_style, "category.module.scss");

/// 特定のカテゴリの記事一覧を取得するサーバー関数
#[server]
pub async fn get_latest_articles(category: String) -> Result<Vec<ArticleSummary>, ServerFnError> {
    s3::fetch_latest_articles(category)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[derive(Store, Clone)]
pub struct ArticleData {
    #[store(key: String = |article: &ArticleSummary| article.id.clone())]
    rows: Vec<ArticleSummary>,
}

/// カテゴリーページコンポーネント
#[component]
pub fn CategoryPage(category: &'static str) -> impl IntoView {
    // カテゴリーに属する記事一覧を取得
    let category_articles = Resource::<Result<Vec<ArticleSummary>, String>>::new(
        || (),
        move |_| async move {
            get_latest_articles(category.to_string())
                .await
                .map_err(|e| e.to_string())
        },
    );

    let articles_store = Store::new(ArticleData { rows: vec![] });
    Effect::new(move |_| {
        if let Some(Ok(articles)) = category_articles.get() {
            let rows = articles_store.rows();
            *rows.write() = articles.clone();
        }
    });

    // カテゴリー名から表示名を取得
    let category_display_name = match category {
        "statistics" => "統計学",
        "physics" => "物理学",
        "daily" => "日常",
        "tech" => "技術",
        _ => category,
    };

    // レスポンシブデザインのためのメニューオープン状態
    let (menu_open, set_menu_open) = signal(false);

    view! {
        <div class=move || {
            let state = if menu_open.get() { layout_style::open } else { "" };
            format!("{} {}", layout_style::layout, state)
        }>
            // ハンバーガーアイコン
            <button
                class=layout_style::toggle_icon
                on:click=move |_| set_menu_open.update(|v| *v = !*v)
                aria-label="Toggle sidebar"
            ></button>
            // サイドバー
            <Sidebar class=layout_style::sidebar category=category />
            <div class=layout_style::content>
                <section class=category_style::category_header>
                    <h1>{category_display_name}</h1>
                    <p class=category_style::category_description>
                        {match category {
                            "tech" => {
                                "技術的なことについて、気になったことを残しておきます。"
                            }
                            "daily" => {
                                "一週間ごとに何かしら書けたらなと思ってます。"
                            }
                            "statistics" => {
                                "統計学について勉強したことを残しておきます。"
                            }
                            "physics" => {
                                "元物理屋なので、物理屋として気になったことメモしておきます。"
                            }
                            _ => "不明なカテゴリー",
                        }}
                    </p>
                </section>

                <section class=category_style::articles_section>
                    <h2 class=category_style::articles_title>{"最近の記事"}</h2>
                    <Suspense fallback=|| {
                        view! {
                            <div class=category_style::loading>"記事を読み込み中..."</div>
                        }
                    }>
                        <ErrorBoundary fallback=|error| {
                            view! {
                                <div class=category_style::error>
                                    "記事の読み込みに失敗しました: "
                                    {format!("{error:?}")}
                                </div>
                            }
                        }>
                            <Show
                                when=move || {
                                    matches!(
                                        category_articles.get(),
                                        Some(Ok(articles))
                                        if !articles.is_empty()
                                    )
                                }
                                fallback=|| {
                                    view! {
                                        <div class=category_style::no_articles>
                                            "記事がありません"
                                        </div>
                                    }
                                }
                            >
                                <div class=category_style::article_list>
                                    <For
                                        each=move || articles_store.rows()
                                        key=|entry| entry.read().id.clone()
                                        children=move |entry| {
                                            view! { <ArticleCard article=entry.read().clone() /> }
                                        }
                                    ></For>
                                </div>
                            </Show>
                        </ErrorBoundary>
                    </Suspense>
                </section>
            </div>
        </div>
    }
}

// タグページコンポーネント
// 特定のタグを持つ記事一覧を表示します
//#[component]
//pub fn TagPage() -> impl IntoView {
//    // URLからタグを取得: ParamsMapを使用
//    let params = use_params_map();
//    let tag = move || params.with(|p: &ParamsMap| p.get("tag").clone().unwrap_or_default());
//
//    // 残りのコードは同じ
//    let tag_articles = Resource::new(
//        move || tag(),
//        |tag| async move { fetch_tag_articles(&tag).await },
//    );
//
//    // リソースの状態を管理するシグナル (変更なし)
//    let is_loading = Signal::derive(move || tag_articles.get().is_none());
//    let has_error =
//        Signal::derive(move || tag_articles.get().is_some_and(|result| result.is_err()));
//    let error_message = Signal::derive(move || match tag_articles.get() {
//        Some(Err(e)) => e.to_string(),
//        _ => String::from("不明なエラー"),
//    });
//
//    let has_articles = Signal::derive(move || match tag_articles.get() {
//        Some(Ok(articles)) if !articles.is_empty() => true,
//        _ => false,
//    });
//    let articles_data = Signal::derive(move || match tag_articles.get() {
//        Some(Ok(articles)) => articles.clone(),
//        _ => vec![],
//    });
//
//    view! {
//        <div class="tag-page">
//            <div class="main-content">
//                <div class="tag-header">
//                    <h1 class="tag-title">タグ: #{tag}</h1>
//                    <p class="tag-description">
//                        {"このタグが付けられた記事一覧です"}
//                    </p>
//                </div>
//
//                // ローディング状態の表示
//                <Show when=move || is_loading.get() fallback=|| ()>
//                    <div class="loading">記事を読み込み中...</div>
//                </Show>
//
//                // エラー状態の表示
//                <Show when=move || has_error.get() fallback=|| ()>
//                    <div class="error">
//                        "記事の読み込みに失敗しました: " {error_message}
//                    </div>
//                </Show>
//
//                // 記事がない場合の表示
//                <Show
//                    when=move || !is_loading.get() && !has_error.get() && !has_articles.get()
//                    fallback=|| ()
//                >
//                    <div class="no-articles">このタグの記事はありません</div>
//                </Show>
//
//                // 記事一覧の表示
//                <Show when=move || has_articles.get() fallback=|| ()>
//                    <div class="article-list">
//                        <For
//                            each=move || articles_data.get()
//                            key=|article| article.id.clone()
//                            let:article
//                        >
//                            <ArticleCard article=article.clone() />
//                        </For>
//                    </div>
//                </Show>
//            </div>
//        </div>
//    }
//}

// WASM（hydrate）用 スタブ：型だけ合わせておく
#[allow(dead_code)]
#[cfg(not(feature = "ssr"))]
async fn fetch_category_articles(_category: &str) -> Result<Vec<ArticleSummary>, String> {
    // クライアントナビゲーション時に呼び出されても型エラーにならないよう、
    // 空リスト or 適当なエラーを返す
    Ok(vec![])
}

// タグで記事を検索する
//#[cfg(feature = "ssr")]
//async fn fetch_tag_articles(tag: &str) -> Result<Vec<ArticleSummary>, String> {
//    // すべてのカテゴリーから記事を取得
//    let categories = vec!["statistics", "physics", "daily", "tech"];
//    let mut all_articles = Vec::new();
//
//    for category in categories {
//        match s3::list_articles(category).await {
//            Ok(mut articles) => {
//                all_articles.append(&mut articles);
//            }
//            Err(e) => {
//                log::error!("カテゴリー{category}の記事取得に失敗: {e}");
//                // エラーがあっても他のカテゴリーは読み込む
//                continue;
//            }
//        }
//    }
//
//    // 指定されたタグを持つ記事のみをフィルタリング
//    let filtered_articles = all_articles
//        .into_iter()
//        .filter(|article| {
//            article
//                .tags
//                .iter()
//                .any(|t| t.to_lowercase() == tag.to_lowercase())
//        })
//        .collect();
//
//    Ok(filtered_articles)
//}

// WASM（hydrate）用 スタブ：型だけ合わせておく
//#[cfg(not(feature = "ssr"))]
//async fn fetch_tag_articles(_tag: &str) -> Result<Vec<ArticleSummary>, String> {
//    // クライアントナビゲーション時に呼び出されても型エラーにならないよう、
//    // 空リスト or 適当なエラーを返す
//    Ok(vec![])
//}
