use crate::components::PageMetadata;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::format::format_display_date;
use crate::routes::not_found::NotFoundPage;
use crate::{SITE_NAME, build_site_url};
#[cfg(feature = "ssr")]
use axum::http::StatusCode;
use domain::ArticlePageDocument;
#[cfg(feature = "ssr")]
use domain::{Category, Slug, build_article_page_document, find_article_summary};
use domain::{
    build_article_page_canonical_path, build_article_page_description, build_article_page_title,
};
#[cfg(feature = "ssr")]
use infra::DynArtifactReader;
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;
use leptos_router::{hooks::use_params_map, params::ParamsMap};
#[cfg(feature = "ssr")]
use std::str::FromStr;

#[server]
pub async fn get_article_page_document(
    category: String,
    slug: String,
) -> Result<Option<ArticlePageDocument>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let artifact_reader = use_context::<DynArtifactReader>()
            .ok_or_else(|| ServerFnError::new("artifact reader context is missing"))?;
        let category = match Category::from_str(&category) {
            Ok(category) => category,
            Err(_) => return Ok(None),
        };
        let slug = match Slug::new(normalize_article_slug_param(&slug).to_string()) {
            Ok(slug) => slug,
            Err(_) => return Ok(None),
        };
        let snapshot = artifact_reader.snapshot().await?;
        let article_index = snapshot.read_article_index().await?;
        let Some(summary) = find_article_summary(&article_index, &category, &slug) else {
            return Ok(None);
        };
        let html = match snapshot.read_article_html(&category, &slug).await {
            Ok(html) => html,
            Err(error) if error.is_not_found() => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        Ok(Some(build_article_page_document(summary, &html)?))
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = category;
        let _ = slug;
        Err(ServerFnError::new(
            "get_article_page_document is only available during SSR",
        ))
    }
}

#[component]
fn ArticlePageContent(document: ArticlePageDocument) -> impl IntoView {
    let title = document.article.title.as_str().to_string();
    let category = document.article.category_display_name;
    let created_at = document.article.created_at;
    let updated_at = document.article.updated_at;
    let created_at_label = format_display_date(&created_at);
    let updated_at_label = format_display_date(&updated_at);
    let description = document.article.description;
    let tags = document.article.tags;
    let has_tags = !tags.is_empty();
    let html = document.html;

    view! {
        <article class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-8 px-4 py-8 text-left sm:px-6 sm:py-12">
            <header class="grid gap-3 rounded-2xl border border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6 text-center shadow-[0_18px_42px_rgb(0_0_0/0.24)] sm:p-8">
                <p class="m-0 text-sm font-bold tracking-[0.12em] text-primary uppercase">
                    {category}
                </p>
                <h1 class="m-0 text-3xl leading-tight font-bold sm:text-4xl lg:text-5xl">
                    {title}
                </h1>
                <p class="m-0 flex flex-wrap justify-center gap-x-2 gap-y-1 leading-7 text-muted-foreground">
                    <span>{"公開 "}<time datetime=created_at>{created_at_label}</time></span>
                    <span aria-hidden="true">{"/"}</span>
                    <span>{"更新 "}<time datetime=updated_at>{updated_at_label}</time></span>
                </p>
                {description
                    .map(|description| {
                        view! {
                            <p class="mx-auto my-0 max-w-3xl leading-8 text-muted-foreground">
                                {description}
                            </p>
                        }
                    })}
                <Show when=move || has_tags fallback=|| ()>
                    <ul
                        class="m-0 flex list-none flex-wrap justify-center gap-2 p-0"
                        aria-label="タグ"
                    >
                        {tags
                            .iter()
                            .map(|tag| {
                                view! {
                                    <li>
                                        <Badge
                                            variant=BadgeVariant::Outline
                                            class="rounded-full border-border bg-background/45 px-3 py-1 text-xs font-normal text-muted-foreground"
                                        >
                                            {format!("#{tag}")}
                                        </Badge>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                </Show>
            </header>

            // Artifact HTML is generated by the private publisher pipeline, which escapes raw
            // HTML from markdown before writing the artifact.
            <div
                class="content-prose mx-auto w-full max-w-3xl rounded-xl border border-border/80 bg-card p-6 shadow-[0_12px_32px_rgb(0_0_0/0.22)] sm:p-8"
                inner_html=html
            ></div>
        </article>
    }
}

#[component]
pub fn ArticlePage() -> impl IntoView {
    let params = use_params_map();
    let article_params = move || {
        params.with(|params: &ParamsMap| {
            (
                params.get("category").unwrap_or_default(),
                params.get("slug").unwrap_or_default(),
            )
        })
    };
    let article_page = Resource::<Result<Option<ArticlePageDocument>, String>>::new_blocking(
        article_params,
        move |(category, slug)| async move {
            if category.is_empty() || slug.is_empty() {
                return Ok(None);
            }

            get_article_page_document(category, slug)
                .await
                .map_err(|error| error.to_string())
        },
    );

    view! {
        <Suspense fallback=move || {
            let (category_param, slug_param) = params
                .with(|params: &ParamsMap| {
                    let slug = params.get("slug").unwrap_or_default();
                    (
                        params.get("category").unwrap_or_default(),
                        normalize_article_slug_param(&slug).to_string(),
                    )
                });
            let canonical_url = build_site_url(&format!("/{category_param}/{slug_param}"));

            view! {
                <PageMetadata
                    title=format!("{slug_param} | {SITE_NAME}")
                    description=format!("{category_param} カテゴリの記事です。")
                    canonical_url
                    og_type="article"
                />
                <div class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] rounded-xl bg-secondary p-8 text-center text-muted-foreground">
                    "記事を読み込み中..."
                </div>
            }
        }>
            {move || match article_page.get() {
                Some(Ok(Some(document))) => {
                    let page_title = build_article_page_title(&document, SITE_NAME);
                    let page_description = build_article_page_description(&document);
                    let canonical_url = build_site_url(
                        &build_article_page_canonical_path(&document),
                    );

                    view! {
                        <PageMetadata
                            title=page_title
                            description=page_description
                            canonical_url
                            og_type="article"
                        />
                        <ArticlePageContent document />
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
                            {format!("記事の読み込みに失敗しました: {error}")}
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

fn normalize_article_slug_param(slug: &str) -> &str {
    slug.strip_suffix(".html").unwrap_or(slug)
}
