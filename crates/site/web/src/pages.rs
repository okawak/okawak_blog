//! Public SSR routes and page-specific components.

use std::str::FromStr;

use domain::{
    ArticlePageDocument, Category as DomainCategory, CategoryPageDocument, HomePageDocument,
    PageKey, Slug, StaticPageDocument, build_article_page_canonical_path,
    build_article_page_description, build_article_page_title, build_category_page_canonical_path,
    build_category_page_description, build_category_page_title, build_category_path,
    build_home_page_canonical_path, build_home_page_description, build_home_page_title,
    build_static_page_canonical_path, build_static_page_description, build_static_page_title,
};
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{StatusCode, path_param, raw_path_params, request, route},
    view::{Unescaped, View, component, view},
};

use crate::{
    PageLoaderContext,
    article_card::article_card,
    shell::{
        ShellMetadata, article_internal_server_error_page, internal_server_error_page,
        not_found_page, site_shell,
    },
};

const ABOUT_PAGE_KEY: &str = "about";
path_param!(category_name);

#[route(GET "/")]
pub async fn home(cx: &Cx) -> Result<View> {
    match page_loader(cx).loader().load_home().await {
        Ok(document) => view! { home_document(document: document) },
        Err(error) => {
            eprintln!("Home page artifact read failed: {error}");
            view! {
                internal_server_error_page(
                    title: build_home_page_title(web::SITE_NAME),
                    description: "公開済みの記事を読み込めませんでした。"
                        .to_string(),
                    canonical_path: "/".to_string(),
                    message: "記事の読み込みに失敗しました"
                )
            }
        }
    }
}

fn page_loader(cx: &Cx) -> &PageLoaderContext {
    try_request_context::<PageLoaderContext>(cx)
        .unwrap_or_else(|| app_context::<PageLoaderContext>(cx))
}

#[component]
async fn home_document(document: HomePageDocument) -> Result {
    let title = build_home_page_title(web::SITE_NAME);
    let description = build_home_page_description(&document);
    let canonical_url = web::build_site_url(build_home_page_canonical_path());
    let is_empty = document.articles.is_empty();

    view! {
        site_shell(
            status: StatusCode::OK,
            metadata: ShellMetadata::website(title, description, canonical_url),
            current_path: "/".to_string(),
            <div
                class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-12 px-4 py-8 text-left sm:px-6 sm:py-12"
            >
                <section
                    class="rounded-2xl border border-border/70 bg-gradient-to-br from-card via-card to-secondary/70 px-6 py-10 text-center shadow-[0_18px_42px_rgb(0_0_0/0.28)] sm:px-10"
                >
                    <p class="m-0 text-sm tracking-[0.16em] text-primary uppercase">
                        "Artifact-Driven Blog"
                    </p>
                    <h1
                        class="m-0 mt-4 text-3xl leading-tight font-bold after:mx-auto after:mt-3 after:block after:h-1 after:w-12 after:rounded-full after:bg-primary sm:text-4xl"
                    >
                        (web::SITE_NAME)
                    </h1>
                    <div class="mx-auto mt-5 max-w-3xl">
                        <p class="m-0 leading-8 text-muted-foreground">
                            "気になったことをメモしておくブログです。Obsidian から生成した成果物をもとに、Topcoat で公開ページを組み立てています。"
                        </p>
                    </div>
                </section>

                <section>
                    <div class="mb-6 grid gap-2">
                        <h2
                            class="m-0 text-2xl font-semibold after:mt-2 after:block after:h-1 after:w-12 after:rounded-full after:bg-primary"
                        >
                            "最近の記事"
                        </h2>
                        <p class="m-0 text-muted-foreground">
                            "新しい順に、公開済みの記事を紹介します。"
                        </p>
                    </div>

                    if is_empty {
                        <div
                            class="rounded-xl bg-secondary p-8 text-center text-muted-foreground"
                        >
                            "記事がありません"
                        </div>
                    } else {
                        home_page_content(document: document)
                    }
                </section>
            </div>
        )
    }
}

#[component]
async fn home_page_content(document: HomePageDocument) -> Result {
    let page_description = build_home_page_description(&document);

    view! {
        <div class="grid gap-6 lg:grid-cols-[minmax(18rem,22rem)_minmax(0,1fr)]">
            <div
                class="flex flex-col gap-4 rounded-xl border border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6 text-card-foreground shadow-sm"
            >
                match document.fragment.as_ref() {
                    Some(fragment) => {
                        <div class="content-prose text-muted-foreground">
                            (Unescaped::new_unchecked(
                                fragment.html.clone(),
                            ))
                        </div>
                    }
                    None => {
                        <p class="m-0 leading-8 text-muted-foreground">
                            "公開済みの artifact をもとに、最近の記事とカテゴリをまとめています。"
                        </p>
                    }
                }
                <p class="m-0 text-lg leading-8">(page_description)</p>
                <ul class="m-0 flex list-none flex-wrap gap-3 p-0">
                    for category in &document.categories {
                        <li>
                            <span
                                class="inline-flex w-fit items-center gap-2 rounded-full border border-border bg-background/45 px-3 py-1.5 text-sm font-semibold text-foreground transition-colors focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2"
                            >
                                <a
                                    href=(build_category_path(&category.category))
                                    class="font-semibold text-foreground no-underline transition-colors hover:text-primary focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                                >
                                    (&category.category_display_name)
                                </a>
                                <span class="text-xs font-normal text-muted-foreground">
                                    (format!("{}本", category.article_count))
                                </span>
                            </span>
                        </li>
                    }
                </ul>
            </div>

            <section class="grid content-start gap-4" aria-label="最近の記事">
                for article in &document.articles {
                    article_card(article: article)
                }
            </section>
        </div>
    }
}

#[route(GET "/{category_name}/{article_slug}")]
pub async fn article_page(cx: &Cx) -> Result<View> {
    let mut params = raw_path_params(cx);
    let category_param = params
        .next()
        .expect("article route should provide a category path parameter")
        .1
        .as_str();
    let slug_param = params
        .next()
        .expect("article route should provide a slug path parameter")
        .1
        .as_str();
    let normalized_slug = normalize_article_slug_param(slug_param);
    let requested_path = request::uri(cx).path().to_string();
    let fallback_title = format!("{normalized_slug} | {}", web::SITE_NAME);
    let fallback_description = format!("{category_param} カテゴリの記事です。");
    let category = match DomainCategory::from_str(category_param) {
        Ok(category) => category,
        Err(_) => return view! { not_found_page(canonical_path: requested_path) },
    };
    let slug = match Slug::new(normalized_slug.to_string()) {
        Ok(slug) => slug,
        Err(_) => return view! { not_found_page(canonical_path: requested_path) },
    };

    match page_loader(cx)
        .loader()
        .load_article(&category, &slug)
        .await
    {
        Ok(Some(document)) => view! { article_document(document: document) },
        Ok(None) => view! { not_found_page(canonical_path: requested_path) },
        Err(error) => {
            eprintln!(
                "Article page artifact read failed for {category_param}/{normalized_slug}: {error}"
            );
            view! {
                article_internal_server_error_page(
                    title: fallback_title,
                    description: fallback_description,
                    canonical_path: requested_path
                )
            }
        }
    }
}

#[component]
async fn article_document(document: ArticlePageDocument) -> Result {
    let title = build_article_page_title(&document, web::SITE_NAME);
    let description = build_article_page_description(&document);
    let canonical_path = build_article_page_canonical_path(&document);
    let canonical_url = web::build_site_url(&canonical_path);
    let article = document.article;
    let page_title = article.title.as_str().to_string();
    let category = article.category_display_name;
    let created_at_label = web::format::format_display_date(&article.created_at);
    let updated_at_label = web::format::format_display_date(&article.updated_at);
    let created_at = article.created_at;
    let updated_at = article.updated_at;
    let article_description = article.description;
    let tags = article.tags;
    // The publish pipeline escapes raw Markdown HTML and neutralizes unsafe href schemes before
    // persisting this fragment. It is therefore the trusted HTML boundary for Topcoat as well.
    let html = Unescaped::new_unchecked(document.html);

    view! {
        site_shell(
            status: StatusCode::OK,
            metadata: ShellMetadata::article(title, description, canonical_url),
            current_path: canonical_path,
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
    }
}

fn normalize_article_slug_param(slug: &str) -> &str {
    slug.strip_suffix(".html").unwrap_or(slug)
}

#[route(GET "/{category_name}")]
pub async fn category_page(cx: &Cx) -> Result<View> {
    let category_param = path_param::<CategoryName>(cx);
    let requested_path = request::uri(cx).path().to_string();
    let category = match DomainCategory::from_str(category_param) {
        Ok(category) => category,
        Err(_) => {
            return view! { not_found_page(canonical_path: requested_path) };
        }
    };

    match page_loader(cx).loader().load_category(&category).await {
        Ok(Some(document)) => view! { category_document(document: document) },
        Ok(None) => {
            view! { not_found_page(canonical_path: requested_path) }
        }
        Err(error) => {
            eprintln!("Category page artifact read failed for {category_param}: {error}");
            view! {
                internal_server_error_page(
                    title: format!("{category_param} | {}", web::SITE_NAME),
                    description: format!("{category_param} カテゴリの記事一覧です。"),
                    canonical_path: requested_path,
                    message: "カテゴリの読み込みに失敗しました"
                )
            }
        }
    }
}

#[component]
async fn category_document(document: CategoryPageDocument) -> Result {
    let title = build_category_page_title(&document, web::SITE_NAME);
    let description = build_category_page_description(&document);
    let canonical_path = build_category_page_canonical_path(&document);
    let canonical_url = web::build_site_url(&canonical_path);
    let page_title = document.title;
    // The publish pipeline escapes raw Markdown HTML and neutralizes unsafe href schemes before
    // persisting this fragment. It is therefore the trusted HTML boundary for Topcoat as well.
    let landing_html = Unescaped::new_unchecked(document.html);

    view! {
        site_shell(
            status: StatusCode::OK,
            metadata: ShellMetadata::website(
                title,
                description.clone(),
                canonical_url,
            ),
            current_path: canonical_path,
            <div
                class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-6 px-4 py-8 text-left sm:px-6 sm:py-12"
            >
                <div
                    class="flex flex-col gap-3 rounded-xl border border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6 text-card-foreground shadow-sm sm:p-8"
                >
                    <p class="m-0 text-sm tracking-[0.16em] text-primary uppercase">
                        "Category"
                    </p>
                    <h1 class="m-0 text-3xl leading-tight font-bold sm:text-4xl">
                        (page_title)
                    </h1>
                    <p class="m-0 leading-7 text-muted-foreground">(description)</p>
                </div>

                <section
                    class="content-prose min-w-0 max-w-full rounded-xl border border-border/80 bg-card p-6 sm:p-8"
                >
                    (landing_html)
                </section>

                <div class="grid gap-6">
                    for section in &document.sections {
                        <section class="grid gap-4">
                            <h2 class="m-0 text-xl font-semibold text-foreground">
                                (&section.heading)
                            </h2>
                            <div class="grid gap-4">
                                for article in &section.articles {
                                    article_card(article: article)
                                }
                            </div>
                        </section>
                    }
                </div>
            </div>
        )
    }
}

#[route(GET "/about")]
pub async fn about(cx: &Cx) -> Result<View> {
    let page = PageKey::new(ABOUT_PAGE_KEY.to_string())?;

    match page_loader(cx).loader().load_static_page(&page).await {
        Ok(Some(document)) => view! { about_document(document: document) },
        Ok(None) => {
            view! { not_found_page(canonical_path: "/about".to_string()) }
        }
        Err(error) => {
            eprintln!("About page artifact read failed: {error}");
            view! {
                internal_server_error_page(
                    title: format!("About | {}", web::SITE_NAME),
                    description: "About ページです。".to_string(),
                    canonical_path: "/about".to_string(),
                    message: "ページの読み込みに失敗しました"
                )
            }
        }
    }
}

#[component]
async fn about_document(document: StaticPageDocument) -> Result {
    let title = build_static_page_title(&document, web::SITE_NAME);
    let description = build_static_page_description(&document);
    let canonical_url = web::build_site_url(&build_static_page_canonical_path(&document));
    let page_title = document.title;
    // The publish pipeline escapes raw Markdown HTML and neutralizes unsafe href schemes before
    // persisting this fragment. It is therefore the trusted HTML boundary for Topcoat as well.
    let html = Unescaped::new_unchecked(document.html);

    view! {
        site_shell(
            status: StatusCode::OK,
            metadata: ShellMetadata::website(title, description, canonical_url),
            current_path: "/about".to_string(),
            <div
                class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-8 px-4 py-8 text-left sm:px-6 sm:py-12"
            >
                <section
                    class="flex min-h-64 items-center justify-center rounded-2xl border border-border/70 bg-gradient-to-br from-card via-card to-secondary/70 p-8 text-center shadow-[0_18px_42px_rgb(0_0_0/0.24)] sm:min-h-80"
                >
                    <div
                        class="grid max-w-xl gap-3 rounded-xl border border-border/60 bg-background/20 p-6 shadow-[0_10px_28px_rgb(0_0_0/0.22)] sm:p-8"
                    >
                        <p
                            class="m-0 text-sm tracking-[0.16em] text-muted-foreground uppercase"
                        >
                            "Page"
                        </p>
                        <h1
                            class="m-0 text-3xl leading-tight font-bold text-primary after:mx-auto after:mt-3 after:block after:h-1 after:w-12 after:rounded-full after:bg-primary sm:text-4xl"
                        >
                            (page_title)
                        </h1>
                    </div>
                </section>
                <article
                    class="content-prose w-full rounded-xl border border-border/80 bg-card p-6 shadow-[0_12px_32px_rgb(0_0_0/0.22)] sm:p-8"
                >
                    (html)
                </article>
            </div>
        )
    }
}
