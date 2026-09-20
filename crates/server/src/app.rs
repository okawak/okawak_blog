//! Topcoat route tree root and application router composition.

use crate::i18n::{Message, t};
use crate::metadata::*;
use domain::Locale;

mod about;
mod api;
mod category_name;
mod content_assets;
mod en;

use std::sync::Arc;

use domain::{
    Category, CategoryPageDocument, HomePageDocument, build_category_path,
    build_home_page_canonical_path,
};
use infra::DynArtifactReader;
use topcoat::{
    Result,
    asset::{AssetConfig, RouterBuilderAssetExt},
    context::{Cx, app_context, memoize, try_request_context},
    router::{
        Body, LayerFn, LayerFuture, Next, Path, Router, StatusCode, TrailingSlash,
        error::NotFoundError,
        page, request,
        response::{IntoResponse, Response},
    },
    runtime::{RouterBuilderRuntimeExt, RouterBuilderShardExt},
    view::{Unescaped, View, ViewExt, attributes, class, component, view},
};

use crate::{
    article_card::article_card,
    artifact_page_loader::ArtifactPageLoader,
    components::{
        alert::{alert, alert_description},
        badge::{BadgeVariant, badge_variants},
    },
    http_cache::{ArtifactConditionalGetDecision, ArtifactHttpCacheState},
    page_loader::{PageLoadResult, PageLoaderContext},
    shell::{ShellMetadata, internal_server_error_page, not_found_page, site_shell},
};

#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    render_home(cx, Locale::Ja).await
}

pub(crate) async fn render_home(cx: &Cx, locale: Locale) -> Result<impl View> {
    match page_loader(cx).loader().load_home(locale).await {
        Ok(Some(presentation)) => {
            Ok(view! { cx => home_document(presentation: presentation, locale: locale) }.boxed())
        }
        Ok(None) => Ok(view! { cx => not_found_page(canonical_path: locale.path("/")) }.boxed()),
        Err(error) => {
            tracing::error!(%error, "home page artifact read failed");
            let description = t(locale, Message::ErrorHome).to_string();
            Ok(view! {
                cx =>
                internal_server_error_page(
                    title: build_home_page_title(locale),
                    description: description,
                    canonical_path: locale.path("/"),
                    message: Message::ErrorHome
                )
            }
            .boxed())
        }
    }
}

pub(crate) fn page_loader(cx: &Cx) -> &PageLoaderContext {
    try_request_context::<PageLoaderContext>(cx)
        .unwrap_or_else(|| app_context::<PageLoaderContext>(cx))
}

// The page and its inline shard share the same published document and snapshot.
#[memoize]
pub(crate) async fn load_category(
    cx: &Cx,
    category: Category,
    locale: Locale,
) -> PageLoadResult<Option<crate::page_loader::Presentation<CategoryPageDocument>>> {
    page_loader(cx)
        .loader()
        .load_category(locale, &category)
        .await
}

fn is_under_path(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn is_site_page_path(path: &str) -> bool {
    !is_under_path(path, "/api")
        && !is_under_path(path, "/_topcoat")
        && !is_under_path(path, "/content-assets")
}

fn render_unmatched_path<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
    Box::pin(async move {
        match next.run(cx, body).await {
            Err(error)
                if error.downcast_ref::<NotFoundError>().is_some()
                    && is_site_page_path(request::uri(cx).path()) =>
            {
                let canonical_path = request::uri(cx).path().to_string();
                let page = view! { cx => not_found_page(canonical_path: canonical_path) }
                    .single()
                    .await?;
                page.into_response(cx)
            }
            response => response,
        }
    })
}

fn not_modified_response(conditional_get: &ArtifactConditionalGetDecision) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    conditional_get.insert_headers(response.headers_mut());
    response
}

fn artifact_conditional_get<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
    Box::pin(async move {
        let state = app_context::<ArtifactHttpCacheState>(cx);
        let Some(conditional_get) = state
            // Runtime page re-runs rewrite POST into GET, but their signal-dependent
            // HTML must not share validators with the ordinary published page.
            .conditional_get(
                request::original_method(cx),
                request::uri(cx),
                request::headers(cx),
            )
            .await
        else {
            return next.run(cx, body).await;
        };

        let negotiate = is_site_page_path(request::uri(cx).path())
            && crate::language::is_negotiated_request(request::uri(cx));
        let mut snapshot = conditional_get.snapshot();
        if negotiate && snapshot.is_none() {
            snapshot = app_context::<api::ArtifactReaderContext>(cx)
                .0
                .snapshot()
                .await
                .ok();
        }
        let scoped = snapshot.map(|snapshot| {
            let loader = PageLoaderContext::new(Arc::new(ArtifactPageLoader::from_snapshot(
                snapshot.clone(),
            )));
            cx.with(snapshot).with(loader)
        });
        let cx = scoped.as_ref().unwrap_or(cx);
        // A Japanese home ETag must never suppress a redirect selected by an English browser.
        if negotiate && let Some(response) = crate::language::redirect(cx).await {
            return Ok(response);
        }
        let mut response = if conditional_get.should_short_circuit() {
            not_modified_response(&conditional_get)
        } else {
            let mut response = next.run(cx, body).await?;
            if conditional_get.should_return_not_modified_after_response(response.status()) {
                not_modified_response(&conditional_get)
            } else {
                if conditional_get.should_attach_validators(response.status()) {
                    conditional_get.insert_headers(response.headers_mut());
                }
                response
            }
        };
        if request::uri(cx).path() == "/" {
            crate::language::vary_home(&mut response);
        }
        Ok(response)
    })
}

pub fn create_router(
    artifact_reader: DynArtifactReader,
    validators_enabled: bool,
    assets: AssetConfig,
) -> Router {
    topcoat::router::module_router!()
        .runtime()
        .discover_shards()
        .trailing_slash(TrailingSlash::Redirect)
        // The framework-neutral decision filters APIs, static assets, and unsuccessful responses.
        // One global layer also avoids nested prefix layers acquiring more than one snapshot.
        .layer(LayerFn::new(None::<&Path>, artifact_conditional_get))
        .layer(LayerFn::new(None::<&Path>, render_unmatched_path))
        .app_context(ArtifactHttpCacheState::new(
            artifact_reader.clone(),
            validators_enabled,
        ))
        .app_context(PageLoaderContext::new(Arc::new(
            ArtifactPageLoader::from_reader(artifact_reader.clone()),
        )))
        .app_context(api::ArtifactReaderContext(artifact_reader))
        .assets(assets)
        .build()
}

#[component]
async fn home_document(
    presentation: crate::page_loader::Presentation<HomePageDocument>,
    locale: Locale,
) -> Result<impl View> {
    let crate::page_loader::Presentation {
        document,
        labels,
        locales,
    } = presentation;
    let title = build_home_page_title(locale);
    let description = build_home_page_description(&document, locale);
    let canonical_path = locale.path(build_home_page_canonical_path());
    let is_empty = document.articles.is_empty();

    Ok(view! {
        site_shell(
            status: StatusCode::OK,
            metadata: ShellMetadata::website(locale, title, description, canonical_path).with_locales(
                locales,
            ),
            <div
                class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-12 px-4 py-8 text-left sm:px-6 sm:py-12"
            >
                <section
                    class="rounded-2xl border border-border/70 bg-gradient-to-br from-card via-card to-secondary/70 px-6 py-10 text-center shadow-[0_18px_42px_rgb(0_0_0/0.28)] sm:px-10"
                >
                    <p class="m-0 text-sm tracking-[0.16em] text-primary uppercase">
                        (t(locale, Message::HomeEyebrow))
                    </p>
                    <h1
                        class="m-0 mt-4 text-3xl leading-tight font-bold after:mx-auto after:mt-3 after:block after:h-1 after:w-12 after:rounded-full after:bg-primary sm:text-4xl"
                    >
                        (t(locale, Message::SiteName))
                    </h1>
                    <div class="mx-auto mt-5 max-w-3xl">
                        <p class="m-0 leading-8 text-muted-foreground">
                            (t(locale, Message::HomeIntro))
                        </p>
                    </div>
                </section>

                <section>
                    <div class="mb-6 grid gap-2">
                        <h2
                            class="m-0 text-2xl font-semibold after:mt-2 after:block after:h-1 after:w-12 after:rounded-full after:bg-primary"
                        >
                            (t(locale, Message::HomeRecent))
                        </h2>
                        <p class="m-0 text-muted-foreground">
                            (t(locale, Message::HomeDescription))
                        </p>
                    </div>

                    if is_empty {
                        alert(
                            attrs: attributes! { class="p-8 text-center" },
                            alert_description((t(locale, Message::EmptyArticles)))
                        )
                    } else {
                        home_page_content(
                            document: document,
                            locale: locale,
                            labels: labels
                        )
                    }
                </section>
            </div>
        )
    })
}

#[component]
async fn home_page_content(
    document: HomePageDocument,
    locale: Locale,
    labels: domain::TagLabels,
) -> Result<impl View> {
    let page_description = build_home_page_description(&document, locale);

    Ok(view! {
        <div class="grid gap-6 lg:grid-cols-[minmax(18rem,22rem)_minmax(0,1fr)]">
            <div
                class="flex flex-col gap-4 rounded-xl border border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6 text-card-foreground shadow-sm"
            >
                match document.fragment.as_ref() {
                    Some(fragment) => {
                        <div class="content-prose text-muted-foreground">
                            (Unescaped::new_unchecked(fragment.html.clone()))
                        </div>
                    }
                    None => {
                        <p class="m-0 leading-8 text-muted-foreground">
                            (t(locale, Message::HomeFallback))
                        </p>
                    }
                }
                <p class="m-0 text-lg leading-8">(page_description)</p>
                <ul class="m-0 flex list-none flex-wrap gap-3 p-0">
                    for category in &document.categories {
                        <li>
                            <a
                                href=(locale.path(&build_category_path(&category.category)))
                                class=(class!(
                                    badge_variants(BadgeVariant::Outline),
                                    "gap-2 rounded-full px-3 py-1.5 text-sm no-underline transition-colors hover:border-primary hover:text-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                                ))
                            >
                                (crate::i18n::category_name(locale, category.category))
                                <span class="text-xs font-normal text-muted-foreground">
                                    (crate::i18n::article_count(locale, category.article_count))
                                </span>
                            </a>
                        </li>
                    }
                </ul>
            </div>

            <section
                class="grid content-start gap-4"
                aria-label=(t(locale, Message::HomeRecent))
            >
                for article in &document.articles {
                    article_card(article: article, locale: locale, labels: &labels)
                }
            </section>
        </div>
    })
}
