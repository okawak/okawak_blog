use crate::components::PageMetadata;
use crate::routes::not_found::NotFoundPage;
use crate::{SITE_NAME, build_site_url};
#[cfg(feature = "ssr")]
use axum::http::StatusCode;
use domain::CategoryPageDocument;
#[cfg(feature = "ssr")]
use domain::{Category, build_category_page_document};
use domain::{
    build_article_path, build_category_page_canonical_path, build_category_page_description,
    build_category_page_title,
};
#[cfg(feature = "ssr")]
use infra::DynArtifactReader;
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;
use leptos_router::components::A;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
#[cfg(feature = "ssr")]
use std::str::FromStr;
use std::sync::Arc;
use stylance::import_style;

import_style!(category_style, "category.module.scss");

#[server]
pub async fn get_category_page_document(
    category: String,
) -> Result<Option<CategoryPageDocument>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let artifact_reader = use_context::<DynArtifactReader>()
            .ok_or_else(|| ServerFnError::new("artifact reader context is missing"))?;
        let category = match Category::from_str(&category) {
            Ok(category) => category,
            Err(_) => return Ok(None),
        };

        let category_index = match artifact_reader.read_category_index(category.as_str()).await {
            Ok(index) => index,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let html = match artifact_reader.read_category_html(&category).await {
            Ok(html) => html,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        Ok(Some(build_category_page_document(&category_index, &html)?))
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = category;
        Err(ServerFnError::new(
            "get_category_page_document is only available during SSR",
        ))
    }
}

#[component]
fn CategoryPageContent(document: CategoryPageDocument) -> impl IntoView {
    let page_description: Arc<str> = build_category_page_description(&document).into();
    let CategoryPageDocument {
        title,
        html: landing_html,
        sections,
        ..
    } = document;
    let section_items = sections
        .into_iter()
        .map(|section| {
            let section_heading = section.heading;
            let article_items = section
                .articles
                .into_iter()
                .map(|article| {
                    let href = build_article_path(&article.category, &article.slug);
                    let title = article.title.as_str().to_string();
                    let description = article
                        .description
                        .unwrap_or_else(|| "説明はまだありません。".to_string());
                    let updated_at = article.updated_at;

                    view! {
                        <article class=category_style::article_card>
                            <h3 class=category_style::article_title>
                                <A href={href} {..} class=category_style::article_link>
                                    {title}
                                </A>
                            </h3>
                            <p class=category_style::article_description>{description}</p>
                            <p class=category_style::article_meta>
                                {format!("更新 {}", updated_at)}
                            </p>
                        </article>
                    }
                })
                .collect_view();

            view! {
                <section class=category_style::section_group>
                    <h2 class=category_style::section_heading>{section_heading}</h2>
                    <div class=category_style::article_list>{article_items}</div>
                </section>
            }
        })
        .collect_view();

    view! {
        <div class=category_style::category_page>
            <header class=category_style::category_header>
                <p class=category_style::eyebrow>{"Category"}</p>
                <p class=category_style::category_title>{title}</p>
                <p class=category_style::category_description>{page_description}</p>
            </header>

            // Publisher artifacts escape raw HTML and neutralize unsafe links before persistence.
            <section class=category_style::landing_content inner_html=landing_html></section>

            <div class=category_style::section_list>{section_items}</div>
        </div>
    }
}

#[component]
pub fn CategoryPage() -> impl IntoView {
    let params = use_params_map();
    let category =
        move || params.with(|params: &ParamsMap| params.get("category").unwrap_or_default());
    let category_page = Resource::<Result<Option<CategoryPageDocument>, String>>::new(
        category,
        move |category| async move {
            if category.is_empty() {
                return Ok(None);
            }

            get_category_page_document(category)
                .await
                .map_err(|error| error.to_string())
        },
    );

    view! {
        {move || {
            let (page_title, page_description, canonical_url) = match category_page.get() {
                Some(Ok(Some(document))) => (
                    build_category_page_title(&document, SITE_NAME),
                    build_category_page_description(&document),
                    build_site_url(&build_category_page_canonical_path(&document)),
                ),
                _ => {
                    let category_param =
                        params.with(|params: &ParamsMap| params.get("category").unwrap_or_default());
                    let page_title = if category_param.is_empty() {
                        SITE_NAME.to_string()
                    } else {
                        format!("{category_param} | {SITE_NAME}")
                    };
                    let page_description = if category_param.is_empty() {
                        "カテゴリページです。".to_string()
                    } else {
                        format!("{category_param} カテゴリの記事一覧です。")
                    };
                    let canonical_url = if category_param.is_empty() {
                        build_site_url("/")
                    } else {
                        build_site_url(&format!("/{category_param}"))
                    };

                    (page_title, page_description, canonical_url)
                }
            };

            view! { <PageMetadata title=page_title description=page_description canonical_url /> }
        }}

        <Suspense fallback=|| {
            view! { <div class=category_style::loading>"カテゴリを読み込み中..."</div> }
        }>
            {move || match category_page.get() {
                Some(Ok(Some(document))) => view! { <CategoryPageContent document /> }.into_any(),
                Some(Ok(None)) => {
                    mark_not_found_response();
                    view! { <NotFoundPage /> }.into_any()
                }
                Some(Err(error)) => {
                    view! {
                        <div class=category_style::error>
                            {format!("カテゴリの読み込みに失敗しました: {error}")}
                        </div>
                    }
                        .into_any()
                }
                None => view! { <div class=category_style::loading></div> }.into_any(),
            }}
        </Suspense>
    }
}

#[cfg(feature = "ssr")]
fn mark_not_found_response() {
    if let Some(response) = use_context::<ResponseOptions>() {
        response.set_status(StatusCode::NOT_FOUND);
    }
}

#[cfg(not(feature = "ssr"))]
fn mark_not_found_response() {}
