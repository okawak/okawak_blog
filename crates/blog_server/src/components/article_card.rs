use crate::models::article::ArticleSummary;
use leptos::prelude::*;
use stylance::import_style;

import_style!(article_card_style, "article_card.module.scss");

/// 記事のカードコンポーネント
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
        <article class=article_card_style::article_card>
            <div class=article_card_style::article_meta>
                <span class=article_card_style::article_date>{article.date_formatted}</span>
                <a href=category_url class=article_card_style::article_category>
                    {category_display_name}
                </a>
            </div>

            <h2 class=article_card_style::article_title>
                <a href=article_url>{article.title}</a>
            </h2>

            {article
                .thumbnail_url
                .map(move |url| {
                    view! {
                        <div class=article_card_style::article_thumbnail>
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

            <div class=article_card_style::article_excerpt>
                <p>{article.excerpt}</p>
            </div>

            <div class=article_card_style::article_footer>
                <div class=article_card_style::article_tags>
                    {article
                        .tags
                        .iter()
                        .map(|tag| {
                            let tag_url = format!("/tag/{tag}");
                            view! {
                                <a href=tag_url class=article_card_style::article_tag>
                                    {format!("#{tag}")}
                                </a>
                            }
                        })
                        .collect_view()}
                </div>
                // クローンした変数を使用
                <a href=article_url_for_readmore class=article_card_style::read_more_link>
                    続きを読む
                </a>
            </div>
        </article>
    }
}
