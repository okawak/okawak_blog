use crate::components::{ArticleCard, Sidebar};
use crate::models::article::ArticleSummary;
use crate::services::s3;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use leptos_router::params::ParamsMap;

/// カテゴリーページコンポーネント
/// 特定のカテゴリーに属する記事一覧を表示します
#[component]
pub fn CategoryPage(category: &'static str) -> impl IntoView {
    // カテゴリーに属する記事一覧を取得
    let category_articles = Resource::new(
        move || category,
        |category| async move { fetch_category_articles(category).await },
    );

    // カテゴリー名から表示名を取得
    let category_display_name = match category {
        "statistics" => "統計学",
        "physics" => "物理学",
        "daily" => "日常",
        "tech" => "技術",
        _ => category,
    };

    // リソースの状態を管理するシグナル
    let is_loading = Signal::derive(move || category_articles.get().is_none());
    let has_error = Signal::derive(move || {
        category_articles
            .get()
            .is_some_and(|result| result.is_err())
    });
    let error_message = Signal::derive(move || match category_articles.get() {
        Some(Err(e)) => e.to_string(),
        _ => String::from("不明なエラー"),
    });
    let has_articles = Signal::derive(move || match category_articles.get() {
        Some(Ok(articles)) if !articles.is_empty() => true,
        _ => false,
    });
    let articles_data = Signal::derive(move || match category_articles.get() {
        Some(Ok(articles)) => articles.clone(),
        _ => vec![],
    });

    view! {
        <div class="category-page">
            <div class="main-content">
                <div class="category-header">
                    <h1 class="category-title">{category_display_name}</h1>
                    <p class="category-description">
                        {match category {
                            "statistics" => {
                                "統計学に関する記事です。データ分析、確率論、統計的推論などについて考察します。"
                            }
                            "physics" => {
                                "物理学に関する記事です。量子力学、相対論、物性物理学などについて考察します。"
                            }
                            "daily" => {
                                "日々の出来事や思ったことを記録します。読書記録や旅行記なども含みます。"
                            }
                            "tech" => {
                                "プログラミングやウェブ開発、サーバー構築など技術的なトピックについて書きます。"
                            }
                            _ => {
                                "様々なトピックに関する記事を掲載しています。"
                            }
                        }}
                    </p>
                </div>

                // ローディング状態の表示
                <Show when=move || is_loading.get() fallback=|| ()>
                    <div class="loading">記事を読み込み中...</div>
                </Show>

                // エラー状態の表示
                <Show when=move || has_error.get() fallback=|| ()>
                    <div class="error">
                        "記事の読み込みに失敗しました: " {error_message}
                    </div>
                </Show>

                // 記事がない場合の表示
                <Show
                    when=move || !is_loading.get() && !has_error.get() && !has_articles.get()
                    fallback=|| ()
                >
                    <div class="no-articles">
                        このカテゴリーには記事がありません
                    </div>
                </Show>

                // 記事一覧の表示
                <Show when=move || has_articles.get() fallback=|| ()>
                    <div class="article-list">
                        <For
                            each=move || articles_data.get()
                            key=|article| article.id.clone()
                            let:article
                        >
                            <ArticleCard article=article.clone() />
                        </For>
                    </div>
                </Show>
            </div>

            <Sidebar />
        </div>
    }
}

/// タグページコンポーネント
/// 特定のタグを持つ記事一覧を表示します
#[component]
pub fn TagPage() -> impl IntoView {
    // URLからタグを取得: ParamsMapを使用
    let params = use_params_map();
    let tag = move || params.with(|p: &ParamsMap| p.get("tag").clone().unwrap_or_default());

    // 残りのコードは同じ
    let tag_articles = Resource::new(
        move || tag(),
        |tag| async move { fetch_tag_articles(&tag).await },
    );

    // リソースの状態を管理するシグナル (変更なし)
    let is_loading = Signal::derive(move || tag_articles.get().is_none());
    let has_error =
        Signal::derive(move || tag_articles.get().is_some_and(|result| result.is_err()));
    let error_message = Signal::derive(move || match tag_articles.get() {
        Some(Err(e)) => e.to_string(),
        _ => String::from("不明なエラー"),
    });

    let has_articles = Signal::derive(move || match tag_articles.get() {
        Some(Ok(articles)) if !articles.is_empty() => true,
        _ => false,
    });
    let articles_data = Signal::derive(move || match tag_articles.get() {
        Some(Ok(articles)) => articles.clone(),
        _ => vec![],
    });

    view! {
        <div class="tag-page">
            <div class="main-content">
                <div class="tag-header">
                    <h1 class="tag-title">タグ: #{tag}</h1>
                    <p class="tag-description">
                        {"このタグが付けられた記事一覧です"}
                    </p>
                </div>

                // ローディング状態の表示
                <Show when=move || is_loading.get() fallback=|| ()>
                    <div class="loading">記事を読み込み中...</div>
                </Show>

                // エラー状態の表示
                <Show when=move || has_error.get() fallback=|| ()>
                    <div class="error">
                        "記事の読み込みに失敗しました: " {error_message}
                    </div>
                </Show>

                // 記事がない場合の表示
                <Show
                    when=move || !is_loading.get() && !has_error.get() && !has_articles.get()
                    fallback=|| ()
                >
                    <div class="no-articles">このタグの記事はありません</div>
                </Show>

                // 記事一覧の表示
                <Show when=move || has_articles.get() fallback=|| ()>
                    <div class="article-list">
                        <For
                            each=move || articles_data.get()
                            key=|article| article.id.clone()
                            let:article
                        >
                            <ArticleCard article=article.clone() />
                        </For>
                    </div>
                </Show>
            </div>

            <Sidebar />
        </div>
    }
}

/// カテゴリー別の記事一覧を取得する
async fn fetch_category_articles(category: &str) -> Result<Vec<ArticleSummary>, String> {
    match s3::list_articles(category).await {
        Ok(articles) => Ok(articles),
        Err(e) => {
            log::error!("カテゴリー別記事の取得に失敗: {}", e);
            Err(format!("記事の読み込みに失敗しました: {}", e))
        }
    }
}

/// タグで記事を検索する
async fn fetch_tag_articles(tag: &str) -> Result<Vec<ArticleSummary>, String> {
    // すべてのカテゴリーから記事を取得
    let categories = vec!["statistics", "physics", "daily", "tech"];
    let mut all_articles = Vec::new();

    for category in categories {
        match s3::list_articles(category).await {
            Ok(mut articles) => {
                all_articles.append(&mut articles);
            }
            Err(e) => {
                log::error!("カテゴリー{category}の記事取得に失敗: {e}");
                // エラーがあっても他のカテゴリーは読み込む
                continue;
            }
        }
    }

    // 指定されたタグを持つ記事のみをフィルタリング
    let filtered_articles = all_articles
        .into_iter()
        .filter(|article| {
            article
                .tags
                .iter()
                .any(|t| t.to_lowercase() == tag.to_lowercase())
        })
        .collect();

    Ok(filtered_articles)
}
