use crate::components::PageMetadata;
use crate::{SITE_NAME, build_site_url};
#[cfg(feature = "ssr")]
use axum::http::StatusCode;
#[cfg(feature = "ssr")]
use domain::PageKey;
#[cfg(feature = "ssr")]
use domain::build_home_page_document;
use domain::{
    HomePageDocument, SiteArticleCard, build_article_path, build_category_path,
    build_home_page_canonical_path, build_home_page_description, build_home_page_title,
};
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;
use leptos_router::components::A;
use std::sync::Arc;
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
        let snapshot = artifact_reader.snapshot().await?;
        let article_index = snapshot.read_article_index().await?;
        let site_metadata = snapshot.read_site_metadata().await?;
        let home_page_key = PageKey::new("home".to_string())
            .map_err(|error| ServerFnError::new(format!("invalid home page key: {error}")))?;
        let home_fragment = match snapshot.read_page_document(&home_page_key).await {
            Ok(fragment) => Some(fragment),
            Err(error) if error.is_not_found() => None,
            Err(error) => return Err(error.into()),
        };

        Ok(build_home_page_document(
            &article_index,
            &site_metadata,
            home_fragment.as_ref(),
        )?)
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
    let page_description: Arc<str> = build_home_page_description(&document).into();
    let home_fragment_html = document
        .fragment
        .as_ref()
        .map(|fragment| fragment.html.clone());
    let category_items = document
        .categories
        .into_iter()
        .map(|category| {
            let href = build_category_path(&category.category);
            view! {
                <li class=home_style::category_chip>
                    <A href={href} {..} class=home_style::category_link>
                        <span class=home_style::category_name>
                            {category.category_display_name}
                        </span>
                    </A>
                    <span class=home_style::category_count>
                        {format!("{}本", category.article_count)}
                    </span>
                </li>
            }
        })
        .collect_view();
    let article_items = document
        .articles
        .into_iter()
        .map(|article| view! { <ArticleCard article=article /> })
        .collect_view();

    view! {
        <div class=home_style::content_grid>
            <section class=home_style::overview_panel>
                {home_fragment_html
                    .map(|html| {
                        view! {
                            // Publisher artifacts escape raw HTML and neutralize unsafe links before persistence.
                            <div class=home_style::overview_copy inner_html=html></div>
                        }
                            .into_any()
                    })
                    .unwrap_or_else(|| {
                        view! {
                            <p class=home_style::overview_copy>
                                {"公開済みの artifact をもとに、最近の記事とカテゴリをまとめています。"}
                            </p>
                        }
                            .into_any()
                    })} <p class=home_style::overview_stats>{page_description}</p>
                <ul class=home_style::category_list>{category_items}</ul>
            </section>

            <section class=home_style::article_list>{article_items}</section>
        </div>
    }
}

#[component]
fn ArticleCard(article: SiteArticleCard) -> impl IntoView {
    let article_href = build_article_path(&article.category, &article.slug);
    let title = article.title.as_str().to_string();
    let article_label = title.clone();
    let category = article.category_display_name;
    let description = article
        .description
        .unwrap_or_else(|| "説明はまだありません。".to_string());
    let tags = article.tags;
    let has_tags = !tags.is_empty();
    let created_at = article.created_at;
    let updated_at = article.updated_at;

    view! {
        <A
            href={article_href}
            {..}
            class=home_style::article_card_link
            attr:aria-label=article_label
        >
            <article class=home_style::article_card>
                <div class=home_style::article_meta>
                    <span class=home_style::article_category>{category}</span>
                    <span class=home_style::category_count>
                        {format!("公開 {} / 更新 {}", created_at, updated_at)}
                    </span>
                </div>

                <h3 class=home_style::article_title>{title}</h3>
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
        </A>
    }
}

/// Home page component.
#[component]
pub fn HomePage() -> impl IntoView {
    let home_page = Resource::<Result<HomePageDocument, String>>::new_blocking(
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
                <h1>{SITE_NAME}</h1>
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
                    view! {
                        <PageMetadata
                            title=build_home_page_title(SITE_NAME)
                            description="公開済みの artifact をもとに、最近の記事とカテゴリをまとめています。"
                            canonical_url=build_site_url(build_home_page_canonical_path())
                        />
                        <div class=home_style::loading>"記事を読み込み中..."</div>
                    }
                }>
                    {move || match home_page.get() {
                        Some(Ok(document)) => {
                            let page_title = build_home_page_title(SITE_NAME);
                            let page_description = build_home_page_description(&document);
                            let canonical_url = build_site_url(build_home_page_canonical_path());
                            let content = if document.articles.is_empty() {
                                view! {
                                    <div class=home_style::no_articles>
                                        "記事がありません"
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! { <HomePageContent document /> }.into_any()
                            };

                            view! {
                                <PageMetadata
                                    title=page_title
                                    description=page_description
                                    canonical_url
                                />
                                {content}
                            }
                                .into_any()
                        }
                        Some(Err(error)) => {
                            mark_internal_server_error_response();
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

#[cfg(feature = "ssr")]
fn mark_internal_server_error_response() {
    if let Some(response) = use_context::<ResponseOptions>() {
        response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[cfg(not(feature = "ssr"))]
fn mark_internal_server_error_response() {}
