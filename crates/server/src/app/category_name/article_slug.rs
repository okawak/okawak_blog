use crate::i18n::{Message, t};
use crate::metadata::*;
use domain::Locale;
use std::str::FromStr;

use domain::{ArticlePageDocument, Category, Slug, build_article_page_canonical_path};
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, page, path_param, request},
    view::{Unescaped, View, ViewExt, component, view},
};

use super::super::page_loader;
use super::CategoryName;
use crate::components::badge::{BadgeVariant, badge};
use crate::shell::{ShellMetadata, article_internal_server_error_page, not_found_page, site_shell};

path_param!(article_slug);

#[page]
async fn article_page(cx: &Cx) -> Result<impl View> {
    render_article(
        cx,
        Locale::Ja,
        path_param::<CategoryName>(cx),
        path_param::<ArticleSlug>(cx),
    )
    .await
}

pub(crate) async fn render_article(
    cx: &Cx,
    locale: Locale,
    category_param: &str,
    slug_param: &str,
) -> Result<impl View> {
    let normalized_slug = normalize_article_slug_param(slug_param);
    let requested_path = request::uri(cx).path().to_string();
    let fallback_title = format!("{normalized_slug} | {}", t(locale, Message::SiteName));
    let fallback_description = t(locale, Message::ErrorArticle).to_string();
    let category = match Category::from_str(category_param) {
        Ok(category) => category,
        Err(_) => {
            return Ok(view! { cx => not_found_page(canonical_path: requested_path) }.boxed());
        }
    };
    let slug = match Slug::new(normalized_slug.to_string()) {
        Ok(slug) => slug,
        Err(_) => {
            return Ok(view! { cx => not_found_page(canonical_path: requested_path) }.boxed());
        }
    };

    match page_loader(cx)
        .loader()
        .load_article(locale, &category, &slug)
        .await
    {
        Ok(Some(presentation)) => Ok(
            view! { cx => article_document(presentation: presentation, locale: locale) }.boxed(),
        ),
        Ok(None) => Ok(view! { cx => not_found_page(canonical_path: requested_path) }.boxed()),
        Err(error) => {
            tracing::error!(
                %error,
                category = category_param,
                slug = normalized_slug,
                "article page artifact read failed"
            );
            Ok(view! {
                cx =>
                article_internal_server_error_page(
                    title: fallback_title,
                    description: fallback_description,
                    canonical_path: requested_path
                )
            }
            .boxed())
        }
    }
}

#[component]
async fn article_document(
    presentation: crate::page_loader::Presentation<ArticlePageDocument>,
    locale: Locale,
) -> Result<impl View> {
    let crate::page_loader::Presentation {
        document,
        labels,
        locales,
    } = presentation;
    let title = build_article_page_title(&document, locale);
    let description = build_article_page_description(&document, locale);
    let canonical_path = locale.path(&build_article_page_canonical_path(&document));
    let article = document.article;
    let page_title = article.title.as_str().to_string();
    let category = crate::i18n::category_name(locale, article.category);
    let created_at_label = crate::format::format_display_date(&article.created_at, locale);
    let updated_at_label = crate::format::format_display_date(&article.updated_at, locale);
    let created_at = article.created_at;
    let updated_at = article.updated_at;
    let article_description = article.description;
    let tags = article.tags;
    // The publish pipeline escapes raw Markdown HTML and neutralizes unsafe href schemes before
    // persisting this fragment. It is therefore the trusted HTML boundary for Topcoat as well.
    let html = Unescaped::new_unchecked(document.html);

    Ok(view! {
        site_shell(
            status: StatusCode::OK,
            metadata: ShellMetadata::article(locale, title, description, canonical_path).with_locales(
                locales,
            ),
            <article
                class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-8 px-4 py-8 text-left sm:px-6 sm:py-12"
            >
                <header
                    class="grid gap-3 rounded-2xl border border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6 text-center shadow-[0_18px_42px_rgb(0_0_0/0.24)] sm:p-8"
                >
                    <p
                        class="m-0 text-sm font-bold tracking-[0.12em] text-primary uppercase"
                    >
                        (category)
                    </p>
                    <h1
                        class="m-0 text-3xl leading-tight font-bold sm:text-4xl lg:text-5xl"
                    >
                        (page_title)
                    </h1>
                    <p
                        class="m-0 flex flex-wrap justify-center gap-x-2 gap-y-1 leading-7 text-muted-foreground"
                    >
                        <span>
                            (t(locale, Message::ArticlePublished))
                            <time datetime=(created_at.as_str())>
                                (created_at_label)
                            </time>
                        </span>
                        <span aria-hidden="true">"/"</span>
                        <span>
                            (t(locale, Message::ArticleUpdated))
                            <time datetime=(updated_at.as_str())>
                                (updated_at_label)
                            </time>
                        </span>
                    </p>
                    if let Some(article_description) = article_description {
                        <p
                            class="mx-auto my-0 max-w-3xl leading-8 text-muted-foreground"
                        >
                            (article_description)
                        </p>
                    }
                    if !tags.is_empty() {
                        <ul
                            class="m-0 flex list-none flex-wrap justify-center gap-2 p-0"
                            aria-label=(t(locale, Message::ArticleTags))
                        >
                            #[key(tag)]
                            for tag in &tags {
                                <li>
                                    badge(
                                        variant: BadgeVariant::Secondary,
                                        (format!("#{}", labels.get(tag).unwrap_or(tag)))
                                    )
                                </li>
                            }
                        </ul>
                    }
                </header>

                <div
                    class="content-prose w-full rounded-xl border border-border/80 bg-card p-6 shadow-[0_12px_32px_rgb(0_0_0/0.22)] sm:p-8"
                >
                    (html)
                </div>
            </article>
        )
    })
}

fn normalize_article_slug_param(slug: &str) -> &str {
    slug.strip_suffix(".html").unwrap_or(slug)
}
