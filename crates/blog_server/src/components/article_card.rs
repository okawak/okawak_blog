use crate::models::article::ArticleSummary;
use leptos::prelude::*;

/// 記事のカードコンポーネント
///
/// 記事のサマリー情報を表示するためのカードUIを提供します。
/// タイトル、日付、カテゴリー、タグなどの基本情報を表示します。
#[component]
pub fn ArticleCard(article: ArticleSummary) -> impl IntoView {
    // 基本的なURLとカテゴリー情報を設定
    let article_url = format!("/{}/{}", article.category, article.slug);
    let category_url = format!("/{}", article.category);
    let category_display_name = match article.category.as_str() {
        "statistics" => "統計学".to_string(),
        "physics" => "物理学".to_string(),
        "daily" => "日常".to_string(),
        "tech" => "技術".to_string(),
        _ => article.category.clone(),
    };

    // 各クロージャで使用する変数をクローンしておく
    let title_clone = article.title.clone();
    let article_url_for_thumbnail = article_url.clone(); // サムネイル用にクローン
    let article_url_for_readmore = article_url.clone(); // 「続きを読む」リンク用にクローン

    view! {
        <article class="article-card">
            <div class="article-meta">
                <span class="article-date">{article.date_formatted}</span>
                <a href=category_url class="article-category">
                    {category_display_name}
                </a>
            </div>

            <h2 class="article-title">
                <a href=article_url>{article.title}</a>
            </h2>

            {article
                .thumbnail_url
                .map(move |url| {
                    view! {
                        <div class="article-thumbnail">
                            // クローンした変数を使用
                            <a href=article_url_for_thumbnail>
                                <img
                                    src=url
                                    alt=format!("{} のサムネイル", title_clone)
                                    loading="lazy"
                                />
                            </a>
                        </div>
                    }
                })}

            <div class="article-excerpt">
                <p>{article.excerpt}</p>
            </div>

            <div class="article-footer">
                <div class="article-tags">
                    {article
                        .tags
                        .iter()
                        .map(|tag| {
                            let tag_url = format!("/tag/{}", tag);
                            view! {
                                <a href=tag_url class="article-tag">
                                    {format!("#{}", tag)}
                                </a>
                            }
                        })
                        .collect_view()}
                </div>
                // クローンした変数を使用
                <a href=article_url_for_readmore class="read-more-link">
                    続きを読む
                </a>
            </div>
        </article>
    }
}
