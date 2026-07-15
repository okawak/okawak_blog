use crate::components::ui::card::Card;
use crate::components::{ArticleCard, PageMetadata};
use crate::routes::not_found::NotFoundPage;
use crate::{SITE_NAME, build_site_url};
#[cfg(feature = "ssr")]
use axum::http::StatusCode;
use domain::CategoryPageDocument;
#[cfg(feature = "ssr")]
use domain::{Category, build_category_page_document};
use domain::{
    build_category_page_canonical_path, build_category_page_description, build_category_page_title,
};
#[cfg(feature = "ssr")]
use infra::DynArtifactReader;
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
#[cfg(feature = "ssr")]
use std::str::FromStr;
use std::sync::Arc;

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
        let snapshot = artifact_reader.snapshot().await?;

        let category_index = match snapshot.read_category_index(category.as_str()).await {
            Ok(index) => index,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let html = match snapshot.read_category_html(&category).await {
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
                .map(|article| view! { <ArticleCard article /> })
                .collect_view();

            view! {
                <section class="grid gap-4">
                    <h2 class="m-0 text-xl font-semibold text-foreground">{section_heading}</h2>
                    <div class="grid gap-4">{article_items}</div>
                </section>
            }
        })
        .collect_view();

    view! {
        <div class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-6 px-4 py-8 text-left sm:px-6 sm:py-12">
            <Card class="gap-3 border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6 sm:p-8">
                <p class="m-0 text-sm tracking-[0.16em] text-primary uppercase">{"Category"}</p>
                <h1 class="m-0 text-3xl leading-tight font-bold sm:text-4xl">{title}</h1>
                <p class="m-0 leading-7 text-muted-foreground">{page_description}</p>
            </Card>

            // Publisher artifacts escape raw HTML and neutralize unsafe links before persistence.
            <section
                class="content-prose min-w-0 max-w-full rounded-xl border border-border/80 bg-card p-6 sm:p-8"
                inner_html=landing_html
            ></section>

            <div class="grid gap-6">{section_items}</div>
        </div>
    }
}

#[component]
pub fn CategoryPage() -> impl IntoView {
    let params = use_params_map();
    let category =
        move || params.with(|params: &ParamsMap| params.get("category").unwrap_or_default());
    let category_page = Resource::<Result<Option<CategoryPageDocument>, String>>::new_blocking(
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
        <Suspense fallback=move || {
            let category_param = params
                .with(|params: &ParamsMap| params.get("category").unwrap_or_default());
            let canonical_url = if category_param.is_empty() {
                build_site_url("/")
            } else {
                build_site_url(&format!("/{category_param}"))
            };

            view! {
                <PageMetadata
                    title=format!("{category_param} | {SITE_NAME}")
                    description=format!("{category_param} カテゴリの記事一覧です。")
                    canonical_url
                />
                <div class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] rounded-xl bg-secondary p-8 text-center text-muted-foreground">
                    "カテゴリを読み込み中..."
                </div>
            }
        }>
            {move || match category_page.get() {
                Some(Ok(Some(document))) => {
                    let page_title = build_category_page_title(&document, SITE_NAME);
                    let page_description = build_category_page_description(&document);
                    let canonical_url = build_site_url(
                        &build_category_page_canonical_path(&document),
                    );

                    view! {
                        <PageMetadata title=page_title description=page_description canonical_url />
                        <CategoryPageContent document />
                    }
                        .into_any()
                }
                Some(Ok(None)) => {
                    mark_not_found_response();
                    view! { <NotFoundPage /> }.into_any()
                }
                Some(Err(error)) => {
                    mark_internal_server_error_response();
                    view! {
                        <div class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] rounded-xl bg-secondary p-8 text-center text-muted-foreground">
                            {format!("カテゴリの読み込みに失敗しました: {error}")}
                        </div>
                    }
                        .into_any()
                }
                None => view! { <div></div> }.into_any(),
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

#[cfg(feature = "ssr")]
fn mark_internal_server_error_response() {
    if let Some(response) = use_context::<ResponseOptions>() {
        response.set_status(StatusCode::INTERNAL_SERVER_ERROR);
    }
}

#[cfg(not(feature = "ssr"))]
fn mark_internal_server_error_response() {}
