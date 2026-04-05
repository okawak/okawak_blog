#[cfg(feature = "ssr")]
use domain::build_home_page_document;
use domain::{HomePageDocument, SiteArticleCard};
use leptos::prelude::*;
use stylance::import_style;

#[cfg(feature = "ssr")]
use infra::DynArtifactReader;

import_style!(home_style, "home.module.scss");

#[server]
pub async fn get_home_page_document() -> Result<HomePageDocument, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let artifact_reader = use_context::<DynArtifactReader>()
            .ok_or_else(|| ServerFnError::new("artifact reader context is missing"))?;
        let article_index = artifact_reader.read_article_index().await?;
        let site_metadata = artifact_reader.read_site_metadata().await?;

        Ok(build_home_page_document(&article_index, &site_metadata)?)
    }

    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new(
            "get_home_page_document is only available during SSR",
        ))
    }
}

#[component]
fn HomePageContent(document: HomePageDocument) -> impl IntoView {
    let categories = document.categories.clone();
    let articles = document.articles.clone();

    view! {
        <div class=home_style::content_grid>
            <section class=home_style::overview_panel>
                <p class=home_style::overview_copy>
                    {"公開済みの artifact をもとに、最近の記事とカテゴリをまとめています。"}
                </p>
                <p class=home_style::overview_stats>
                    {format!("{}本の記事を {}カテゴリで公開中です。", document.total_articles, categories.len())}
                </p>
                <ul class=home_style::category_list>
                    <For
                        each=move || categories.clone()
                        key=|category| category.category.as_str().to_string()
                        children=move |category| {
                            view! {
                                <li class=home_style::category_chip>
                                    <span class=home_style::category_name>
                                        {category.category_display_name}
                                    </span>
                                    <span class=home_style::category_count>
                                        {format!("{}本", category.article_count)}
                                    </span>
                                </li>
                            }
                        }
                    />
                </ul>
            </section>

            <section class=home_style::article_list>
                <For
                    each=move || articles.clone()
                    key=|article| article.slug.as_str().to_string()
                    children=move |article| {
                        view! { <ArticleCard article=article /> }
                    }
                />
            </section>
        </div>
    }
}

#[component]
fn ArticleCard(article: SiteArticleCard) -> impl IntoView {
    let slug = article.slug.as_str().to_string();
    let title = article.title.as_str().to_string();
    let category = article.category_display_name;
    let description = article
        .description
        .unwrap_or_else(|| "説明はまだありません。".to_string());
    let tags = article.tags;
    let has_tags = !tags.is_empty();
    let created_at = article.created_at;
    let updated_at = article.updated_at;

    view! {
        <article class=home_style::article_card>
            <div class=home_style::article_meta>
                <span class=home_style::article_category>{category}</span>
                <span class=home_style::category_count>
                    {format!("公開 {} / 更新 {}", created_at, updated_at)}
                </span>
            </div>

            <h3 class=home_style::article_title>
                <a class=home_style::article_link href=format!("/articles/{slug}")>
                    {title}
                </a>
            </h3>
            <p class=home_style::article_description>{description}</p>

            <Show when=move || has_tags fallback=|| ()>
                <ul class=home_style::tag_list>
                    {tags
                        .iter()
                        .map(|tag| {
                            view! { <li class=home_style::tag>{format!("#{tag}")}</li> }
                        })
                        .collect_view()}
                </ul>
            </Show>
        </article>
    }
}

/// ホームページコンポーネント
#[component]
pub fn HomePage() -> impl IntoView {
    let home_page = Resource::<Result<HomePageDocument, String>>::new(
        || (),
        move |_| async move {
            get_home_page_document()
                .await
                .map_err(|error| error.to_string())
        },
    );

    view! {
        <div class=home_style::home_page>
            <section class=home_style::profile_section>
                <p class=home_style::eyebrow>{"Artifact-Driven Blog"}</p>
                <h1>{"ぶくせんの探窟メモ"}</h1>
                <div class=home_style::profile_text>
                    <p>
                        {"気になったことをメモしておくブログです。Obsidian から生成した成果物をもとに、Leptos で公開ページを組み立てています。"}
                    </p>
                </div>
            </section>

            <section class=home_style::latest_articles>
                <div class=home_style::section_header>
                    <h2>{"最近の記事"}</h2>
                    <p>{"まずは home を artifact 駆動に置き換えています。"}</p>
                </div>

                <Suspense fallback=|| {
                    view! { <div class=home_style::loading>"記事を読み込み中..."</div> }
                }>
                    {move || match home_page.get() {
                        Some(Ok(document)) if document.articles.is_empty() => {
                            view! {
                                <div class=home_style::no_articles>
                                    "記事がありません"
                                </div>
                            }
                                .into_any()
                        }
                        Some(Ok(document)) => {
                            view! { <HomePageContent document /> }.into_any()
                        }
                        Some(Err(error)) => {
                            view! {
                                <div class=home_style::error>
                                    {format!("記事の読み込みに失敗しました: {error}")}
                                </div>
                            }
                                .into_any()
                        }
                        None => view! { <div class=home_style::loading></div> }.into_any(),
                    }}
                </Suspense>
            </section>
        </div>
    }
}
