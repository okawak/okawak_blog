use crate::components::{MarkdownRenderer, Sidebar, TagList};
use crate::models::article::Article;
#[cfg(feature = "ssr")]
use crate::services::s3;
use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
use stylance::import_style;

import_style!(layout_style, "layout.module.scss");
import_style!(article_style, "article.module.scss");

/// 特定の記事を取得するサーバー関数
#[server]
pub async fn get_article(category: String, slug: String) -> Result<Article, ServerFnError> {
    s3::get_article(&category, &slug)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

/// 記事ページコンポーネント
#[component]
pub fn ArticlePage(category: &'static str) -> impl IntoView {
    // URLからカテゴリーとスラッグを取得
    let params = use_params_map();
    let slug = move || params.with(|p: &ParamsMap| p.get("slug").clone().unwrap_or_default());

    // 記事データを取得
    let article_resource = Resource::new(
        move || (category.to_string(), slug()),
        move |(category_name, slug_value)| async move {
            log::info!("slug_value: {slug_value}");
            get_article(category_name, slug_value).await
        },
    );

    // レスポンシブデザインのためのメニューオープン状態
    let (menu_open, set_menu_open) = signal(false);

    view! {
        <div class=move || {
            let state = if menu_open.get() { layout_style::open } else { "" };
            format!("{} {}", layout_style::layout, state)
        }>
            // toggleボタン(モバイル用)
            <button
                class=layout_style::toggle_icon
                on:click=move |_| set_menu_open.update(|v| *v = !*v)
                aria-label="Toggle sidebar"
            ></button>
            // サイドバー
            <Sidebar class=layout_style::sidebar category=category />
            <div class=move || format!("{} {}", layout_style::content, article_style::article_page)>
                <Suspense fallback=|| {
                    view! { <div class=article_style::loading>"記事を読み込み中..."</div> }
                }>
                    <ErrorBoundary fallback=|error| {
                        view! {
                            <div class=article_style::error>
                                "記事の読み込みに失敗しました: "
                                {format!("{error:?}")}
                            </div>
                        }
                    }>
                        <Show
                            when=move || matches!(article_resource.get(), Some(Ok(_)))
                            fallback=move || {
                                view! {
                                    <div class=article_style::no_articles>
                                        {"記事がありません。"}
                                    </div>
                                }
                            }
                        >
                            {move || {
                                article_resource
                                    .get()
                                    .and_then(Result::ok)
                                    .map(|article| {
                                        view! {
                                            <article class=article_style::article>
                                                <h1 class=article_style::article_title>{article.title}</h1>

                                                <TagList tags=article.tags.clone() />

                                                <div class=article_style::article_meta>
                                                    <p>{format!("公開日: {}", article.published_at)}</p>
                                                    <p>{format!("更新日: {}", article.updated_at)}</p>
                                                </div>

                                                <MarkdownRenderer content=article.content.clone() />
                                            </article>
                                        }
                                    })
                            }}
                        </Show>
                    </ErrorBoundary>
                </Suspense>
            </div>
        </div>
    }
}

// 記事データを取得する
//#[cfg(feature = "ssr")]
//async fn fetch_article(category: &str, slug: &str) -> Result<Article, AppError> {
//    s3::get_article(category, slug).await
//}
//
//// WASM（hydrate）用 スタブ：型だけ合わせておく
//#[allow(dead_code)]
//#[cfg(not(feature = "ssr"))]
//async fn fetch_article(_category: &str, _slug: &str) -> Result<Article, AppError> {
//    // クライアントナビゲーション時に呼び出されても型エラーにならないよう、
//    // 空リスト or 適当なエラーを返す
//    Ok(Article::default())
//}
