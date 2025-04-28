use crate::components::{MarkdownRenderer, Sidebar, TagList};
use crate::error::AppError;
use crate::models::article::Article;
#[cfg(feature = "ssr")]
use crate::services::s3;
use leptos::prelude::*;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
use stylance::import_style;

import_style!(layout_style, "layout.module.scss");

/// 記事詳細ページコンポーネント（シンプル版）
#[component]
pub fn ArticlePage(cat: &'static str) -> impl IntoView {
    // URLからカテゴリーとスラッグを取得
    let params = use_params_map();
    let category =
        move || params.with(|p: &ParamsMap| p.get("category").clone().unwrap_or_default());
    let slug = move || params.with(|p: &ParamsMap| p.get("slug").clone().unwrap_or_default());

    // 記事データを取得
    let article_resource = Resource::new(
        move || (category(), slug()),
        |(category, slug)| async move { fetch_article(&category, &slug).await },
    );

    // リソースの状態を管理するシグナル
    let is_loading = Memo::new(move |_| article_resource.get().is_none());
    let has_error =
        Memo::new(move |_| article_resource.get().is_some_and(|result| result.is_err()));
    let error_message = Memo::new(move |_| match article_resource.get() {
        Some(Err(e)) => e.to_string(),
        _ => String::from("記事の読み込みに失敗しました"),
    });

    // 記事データを個別のシグナルに分解
    let article_title = Memo::new(move |_| {
        article_resource
            .get()
            .and_then(|result| result.ok())
            .map(|article| article.title.clone())
            .unwrap_or_default()
    });

    let article_content = Memo::new(move |_| {
        article_resource
            .get()
            .and_then(|result| result.ok())
            .map(|article| article.content.clone())
            .unwrap_or_default()
    });

    let article_category = Memo::new(move |_| {
        article_resource
            .get()
            .and_then(|result| result.ok())
            .map(|article| article.category.clone())
            .unwrap_or_default()
    });

    let article_tags = Memo::new(move |_| {
        article_resource
            .get()
            .and_then(|result| result.ok())
            .map(|article| article.tags.clone())
            .unwrap_or_default()
    });

    let article_date = Memo::new(move |_| {
        article_resource
            .get()
            .and_then(|result| result.ok())
            .map(|article| article.date_formatted())
            .unwrap_or_default()
    });

    let has_article =
        Memo::new(move |_| article_resource.get().is_some_and(|result| result.is_ok()));

    view! {
        <div class="article-page">
            <div class="main-content">
                // ローディング状態の表示
                <Show when=move || is_loading.get() fallback=|| ()>
                    <div class="loading">記事を読み込み中...</div>
                </Show>

                // エラー状態の表示
                <Show when=move || has_error.get() fallback=|| ()>
                    <div class="error">
                        <h2>エラーが発生しました</h2>
                        <p>{move || error_message.get()}</p>
                        <a href="/" class="back-link">
                            ホームに戻る
                        </a>
                    </div>
                </Show>

                // 記事内容の表示（シンプル化）
                <Show when=move || has_article.get() fallback=|| ()>
                    <article class="article-content">
                        <header class="article-header">
                            <div class="article-meta">
                                <a
                                    href=move || format!("/{}", article_category.get())
                                    class="category-link"
                                >
                                    {move || get_category_display_name(article_category.get())}
                                </a>
                                <time>{move || article_date.get()}</time>
                            </div>
                            <h1>{move || article_title.get()}</h1>
                            // タグリストのプロパティ修正: tagsをSignal::derivedを使って変換
                            <TagList tags=Signal::derive(move || article_tags.get()).get() />
                        </header>

                        // シンプル化されたMarkdown表示
                        <div class="article-body">
                            <MarkdownRenderer
                                content=article_content.get()
                                enable_toc=true
                                enable_mathjax=true
                            />
                        </div>
                    </article>
                </Show>
            </div>

            <Sidebar category=cat class="" />
        </div>
    }
}

/// カテゴリーコードから表示名を取得
/// 引数のライフタイムと戻り値の静的ライフタイムの不一致を修正
fn get_category_display_name(category: String) -> &'static str {
    match category.as_str() {
        "statistics" => "統計学",
        "physics" => "物理学",
        "daily" => "日常",
        "tech" => "技術",
        _ => "その他", // 未知のカテゴリーの場合は静的文字列を返す
    }
}

/// 記事データを取得する
#[cfg(feature = "ssr")]
async fn fetch_article(category: &str, slug: &str) -> Result<Article, AppError> {
    s3::get_article(category, slug).await
}

// WASM（hydrate）用 スタブ：型だけ合わせておく
#[cfg(not(feature = "ssr"))]
async fn fetch_article(_category: &str, _slug: &str) -> Result<Article, AppError> {
    // クライアントナビゲーション時に呼び出されても型エラーにならないよう、
    // 空リスト or 適当なエラーを返す
    Ok(Article::default())
}
