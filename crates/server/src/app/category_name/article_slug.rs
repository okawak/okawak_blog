use std::str::FromStr;

use domain::{
    ArticlePageDocument, Category, Slug, build_article_page_canonical_path,
    build_article_page_description, build_article_page_title,
};
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, page, path_param, request},
    view::{Unescaped, View, ViewExt, component, view},
};

use super::super::page_loader;
use super::CategoryName;
use crate::shell::{ShellMetadata, article_internal_server_error_page, not_found_page, site_shell};

path_param!(article_slug);

#[page]
async fn article_page(cx: &Cx) -> Result<impl View> {
    let category_param = path_param::<CategoryName>(cx);
    let slug_param = path_param::<ArticleSlug>(cx);
    let normalized_slug = normalize_article_slug_param(slug_param);
    let requested_path = request::uri(cx).path().to_string();
    let fallback_title = format!("{normalized_slug} | {}", crate::SITE_NAME);
    let fallback_description = format!("{category_param} カテゴリの記事です。");
    let category = match Category::from_str(category_param) {
        Ok(category) => category,
        Err(_) => return Ok(view! { not_found_page(canonical_path: requested_path) }.boxed()),
    };
    let slug = match Slug::new(normalized_slug.to_string()) {
        Ok(slug) => slug,
        Err(_) => return Ok(view! { not_found_page(canonical_path: requested_path) }.boxed()),
    };

    match page_loader(cx)
        .loader()
        .load_article(&category, &slug)
        .await
    {
        Ok(Some(document)) => Ok(view! { article_document(document: document) }.boxed()),
        Ok(None) => Ok(view! { not_found_page(canonical_path: requested_path) }.boxed()),
        Err(error) => {
            tracing::error!(
                %error,
                category = category_param,
                slug = normalized_slug,
                "article page artifact read failed"
            );
            Ok(view! {
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
async fn article_document(document: ArticlePageDocument) -> Result<impl View> {
    let title = build_article_page_title(&document, crate::SITE_NAME);
    let description = build_article_page_description(&document);
    let canonical_path = build_article_page_canonical_path(&document);
    let canonical_url = crate::build_site_url(&canonical_path);
    let article = document.article;
    let page_title = article.title.as_str().to_string();
    let category = article.category_display_name;
    let created_at_label = crate::format::format_display_date(&article.created_at);
    let updated_at_label = crate::format::format_display_date(&article.updated_at);
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
            metadata: ShellMetadata::article(title, description, canonical_url),
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
                            "公開 "
                            <time datetime=(created_at.as_str())>
                                (created_at_label)
                            </time>
                        </span>
                        <span aria-hidden="true">"/"</span>
                        <span>
                            "更新 "
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
                            aria-label="タグ"
                        >
                            for tag in &tags {
                                <li>
                                    <span
                                        class="inline-flex w-fit items-center rounded-full border border-border bg-background/45 px-3 py-1 text-xs font-normal text-muted-foreground transition-colors focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2"
                                    >
                                        (format!("#{tag}"))
                                    </span>
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
