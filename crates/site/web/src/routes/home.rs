use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::card::Card;
use crate::components::{ArticleCard, PageMetadata};
use crate::{SITE_NAME, build_site_url};
#[cfg(feature = "ssr")]
use axum::http::StatusCode;
#[cfg(feature = "ssr")]
use domain::PageKey;
#[cfg(feature = "ssr")]
use domain::build_home_page_document;
use domain::{
    HomePageDocument, build_category_path, build_home_page_canonical_path,
    build_home_page_description, build_home_page_title,
};
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_axum::ResponseOptions;
use leptos_router::components::A;
use std::sync::Arc;

#[cfg(feature = "ssr")]
use infra::DynArtifactReader;

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
                <li>
                    <Badge
                        variant=BadgeVariant::Outline
                        class="gap-2 rounded-full border-border bg-background/45 px-3 py-1.5 text-sm"
                    >
                        <A
                            href={href}
                            {..}
                            class="font-semibold text-foreground no-underline transition-colors hover:text-primary focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                        >
                            {category.category_display_name}
                        </A>
                        <span class="text-xs font-normal text-muted-foreground">
                            {format!("{}本", category.article_count)}
                        </span>
                    </Badge>
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
        <div class="grid gap-6 lg:grid-cols-[minmax(18rem,22rem)_minmax(0,1fr)]">
            <Card class="gap-4 border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6">
                {home_fragment_html
                    .map(|html| {
                        view! {
                            // Publisher artifacts escape raw HTML and neutralize unsafe links before persistence.
                            <div
                                class="leading-8 text-muted-foreground [&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
                                inner_html=html
                            ></div>
                        }
                            .into_any()
                    })
                    .unwrap_or_else(|| {
                        view! {
                            <p class="m-0 leading-8 text-muted-foreground">
                                {"公開済みの artifact をもとに、最近の記事とカテゴリをまとめています。"}
                            </p>
                        }
                            .into_any()
                    })} <p class="m-0 text-lg leading-8">{page_description}</p>
                <ul class="m-0 flex list-none flex-wrap gap-3 p-0">{category_items}</ul>
            </Card>

            <section class="grid content-start gap-4" aria-label="最近の記事">
                {article_items}
            </section>
        </div>
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
        <div class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-12 px-4 py-8 text-left sm:px-6 sm:py-12">
            <section class="rounded-2xl border border-border/70 bg-gradient-to-br from-card via-card to-secondary/70 px-6 py-10 text-center shadow-[0_18px_42px_rgb(0_0_0/0.28)] sm:px-10">
                <p class="m-0 text-sm tracking-[0.16em] text-primary uppercase">
                    {"Artifact-Driven Blog"}
                </p>
                <h1 class="m-0 mt-4 text-3xl leading-tight font-bold after:mx-auto after:mt-3 after:block after:h-1 after:w-12 after:rounded-full after:bg-primary sm:text-4xl">
                    {SITE_NAME}
                </h1>
                <div class="mx-auto mt-5 max-w-3xl">
                    <p class="m-0 leading-8 text-muted-foreground">
                        {"気になったことをメモしておくブログです。Obsidian から生成した成果物をもとに、Leptos で公開ページを組み立てています。"}
                    </p>
                </div>
            </section>

            <section>
                <div class="mb-6 grid gap-2">
                    <h2 class="m-0 text-2xl font-semibold after:mt-2 after:block after:h-1 after:w-12 after:rounded-full after:bg-primary">
                        {"最近の記事"}
                    </h2>
                    <p class="m-0 text-muted-foreground">
                        {"新しい順に、公開済みの記事を紹介します。"}
                    </p>
                </div>

                <Suspense fallback=|| {
                    view! {
                        <PageMetadata
                            title=build_home_page_title(SITE_NAME)
                            description="公開済みの artifact をもとに、最近の記事とカテゴリをまとめています。"
                            canonical_url=build_site_url(build_home_page_canonical_path())
                        />
                        <div class="rounded-xl bg-secondary p-8 text-center text-muted-foreground">
                            "記事を読み込み中..."
                        </div>
                    }
                }>
                    {move || match home_page.get() {
                        Some(Ok(document)) => {
                            let page_title = build_home_page_title(SITE_NAME);
                            let page_description = build_home_page_description(&document);
                            let canonical_url = build_site_url(build_home_page_canonical_path());
                            let content = if document.articles.is_empty() {
                                view! {
                                    <div class="rounded-xl bg-secondary p-8 text-center text-muted-foreground">
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
                                <div class="rounded-xl bg-secondary p-8 text-center text-muted-foreground">
                                    {format!("記事の読み込みに失敗しました: {error}")}
                                </div>
                            }
                                .into_any()
                        }
                        None => view! { <div></div> }.into_any(),
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
