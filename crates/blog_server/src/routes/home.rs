use crate::components::{ArticleCard, Sidebar};
use crate::models::article::ArticleSummary;
use crate::services::s3;
use leptos::prelude::*;

/// ホームページコンポーネント
#[component]
pub fn HomePage() -> impl IntoView {
    // 最新記事一覧を取得
    let latest_articles = Resource::new(
        || (),
        |_| async move { fetch_latest_articles().await }
    );

    // 手動でローディング状態を管理
    // 初期状態: get() が None を返す場合はロード中
    // エラー状態: get() が Some(Err(_)) を返す場合はエラー
    // 完了状態: get() が Some(Ok(_)) を返す場合は読み込み完了
    let is_loading = Signal::derive(move || latest_articles.get().is_none());
    let has_error = Signal::derive(move || latest_articles.get().is_some_and(|result| result.is_err()));
    let error_message = Signal::derive(move || {
        match latest_articles.get() {
            Some(Err(e)) => e.to_string(),
            _ => String::from("不明なエラー")
        }
    });
    let has_articles = Signal::derive(move || {
        match latest_articles.get() {
            Some(Ok(articles)) if !articles.is_empty() => true,
            _ => false
        }
    });
    let articles_data = Signal::derive(move || {
        match latest_articles.get() {
            Some(Ok(articles)) => articles.clone(),
            _ => vec![]
        }
    });

    view! {
        <div class="home-page">
            <div class="main-content">
                <section class="profile-section">
                    <h1>ぶくせんの探窟メモ</h1>
                    <div class="profile-content">
                        <div class="profile-image">
                            <img src="/assets/images/profile.jpg" alt="プロフィール画像" />
                        </div>
                        <div class="profile-text">
                            <p>
                                "こんにちは、岡わかです。このブログでは統計学、物理学、技術、そして日常の事柄について"
                                "考えたことを記録しています。"
                            </p>
                            <p>
                                "専門は物理学ですが、データ分析や数学的なトピックについても興味を持っています。"
                                "このブログが何か新しい発見や学びのきっかけになれば嬉しいです。"
                            </p>
                        </div>
                    </div>
                </section>

                <section class="latest-articles">
                    <h2>最新の記事</h2>

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
                        <div class="no-articles">記事がありません</div>
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
                </section>
            </div>

            <Sidebar />
        </div>
    }
}

/// S3バケットから最新記事一覧を取得
async fn fetch_latest_articles() -> Result<Vec<ArticleSummary>, String> {
    // 全カテゴリーから最新記事を集める
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

    // 投稿日時の降順でソート
    all_articles.sort_by(|a, b| b.published_at.cmp(&a.published_at));

    // 最大10件に制限
    let latest = all_articles.into_iter().take(10).collect();

    Ok(latest)
}
