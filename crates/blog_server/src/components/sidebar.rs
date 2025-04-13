use crate::models::article::ArticleSummary;
use leptos::prelude::*;

/// サイドバーコンポーネント
///
/// 人気記事、カテゴリー、タグなどの補助的なナビゲーション要素を表示します
#[component]
pub fn Sidebar(#[prop(optional)] recent_articles: Option<Vec<ArticleSummary>>) -> impl IntoView {
    view! {
        <aside class="sidebar">
            <div class="sidebar-section">
                <h3 class="sidebar-title">カテゴリー</h3>
            // ...（カテゴリーリストの部分は変更なし）...
            </div>

            <div class="sidebar-section">
                <h3 class="sidebar-title">最近の記事</h3>
                <RecentArticlesList articles=recent_articles />
            </div>

        // <div class="sidebar-section">
        // <h3 class="sidebar-title">プロフィール</h3>
        // <div class="author-profile">
        // <img src="/images/profile.jpg" alt="プロフィール画像" class="profile-image" />
        // <p>
        // しがないエンジニア。<br />
        // </p>
        // <div class="social-links">
        // <a href="https://github.com/okawak" target="_blank" rel="noopener noreferrer">
        // <i class="github-icon"></i> GitHub
        // </a>
        // <a href="_" target="_blank" rel="noopener noreferrer">
        // <i class="twitter-icon"></i> Twitter
        // </a>
        // </div>
        // </div>
        // </div>
        // コメントアウトされたプロフィールセクション...
        </aside>
    }
}

/// 最近の記事リストを表示するコンポーネント
#[component]
fn RecentArticlesList(articles: Option<Vec<ArticleSummary>>) -> impl IntoView {
    // 記事の配列を保持するシグナル
    // これにより値がリアクティブコンテキスト内で安全に使用できる
    let articles_resource = RwSignal::new(articles);

    // 記事があるかどうかをチェックする関数（Fnトレイトを実装）
    let has_articles =
        move || articles_resource.with(|arts| arts.as_ref().map_or(false, |a| !a.is_empty()));

    view! {
        <ul class="recent-posts">
            <Show
                when=has_articles
                fallback=|| {
                    view! { <li class="no-recent-posts">最近の投稿はありません</li> }
                }
            >
                <For
                    // シグナルから値を取得するが、消費はしない
                    each=move || {
                        articles_resource
                            .with(|arts| {
                                arts.as_ref()
                                    .map(|a| a.iter().take(5).cloned().collect::<Vec<_>>())
                                    .unwrap_or_default()
                            })
                    }
                    key=|article| article.id.clone()
                    let:article
                >
                    <li>
                        <a href=format!(
                            "/{}/{}",
                            article.category,
                            article.slug,
                        )>{article.title.clone()}</a>
                        <span class="post-date">{article.date_formatted.clone()}</span>
                    </li>
                </For>
            </Show>
        </ul>
    }
}
