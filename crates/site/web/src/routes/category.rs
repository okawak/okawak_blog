use crate::routes::not_found::NotFoundPage;
#[cfg(feature = "ssr")]
use axum::http::StatusCode;
use domain::CategoryPageDocument;
#[cfg(feature = "ssr")]
use domain::{Category, build_category_page_document};
use domain::{build_category_page_description, build_category_page_title};
#[cfg(feature = "ssr")]
use infra::DynArtifactReader;
use leptos::prelude::*;
use leptos_meta::{Meta, Title};
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
#[cfg(feature = "ssr")]
use std::str::FromStr;
use stylance::import_style;

import_style!(category_style, "category.module.scss");

const SITE_NAME: &str = "ぶくせんの探窟メモ";

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

        Ok(Some(build_category_page_document(&category_index)?))
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
    let page_title = build_category_page_title(&document, SITE_NAME);
    let page_description = build_category_page_description(&document);
    let title = document.category_display_name;
    let article_items = document
        .articles
        .into_iter()
        .map(|article| {
            let href = format!("/articles/{}", article.slug.as_str());
            let title = article.title.as_str().to_string();
            let description = article
                .description
                .unwrap_or_else(|| "説明はまだありません。".to_string());
            let updated_at = article.updated_at;

            view! {
                <article class=category_style::article_card>
                    <h2 class=category_style::article_title>
                        <a class=category_style::article_link href=href>
                            {title}
                        </a>
                    </h2>
                    <p class=category_style::article_description>{description}</p>
                    <p class=category_style::article_meta>
                        {format!("更新 {}", updated_at)}
                    </p>
                </article>
            }
        })
        .collect_view();

    view! {
        <Title text=page_title />
        <Meta name="description" content=page_description.clone() />

        <div class=category_style::category_page>
            <header class=category_style::category_header>
                <p class=category_style::eyebrow>{"Category"}</p>
                <h1 class=category_style::category_title>{title}</h1>
                <p class=category_style::category_description>{page_description}</p>
            </header>

            <section class=category_style::article_list>
                {article_items}
            </section>
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
        <Suspense fallback=|| {
            view! { <div class=category_style::loading>"カテゴリを読み込み中..."</div> }
        }>
            {move || match category_page.get() {
                Some(Ok(Some(document))) => view! { <CategoryPageContent document /> }.into_any(),
                Some(Ok(None)) => {
                    mark_not_found_response();
                    view! { <NotFoundPage /> }.into_any()
                }
                Some(Err(error)) => view! {
                    <div class=category_style::error>
                        {format!("カテゴリの読み込みに失敗しました: {error}")}
                    </div>
                }
                .into_any(),
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
