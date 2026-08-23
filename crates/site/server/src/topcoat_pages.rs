//! Topcoat SSR pages introduced route by route during the migration.

use std::str::FromStr;

use chrono::Datelike;
use domain::{
    Category as DomainCategory, CategoryPageDocument, HomePageDocument, PageKey, SiteArticleCard,
    StaticPageDocument, build_article_path, build_category_page_canonical_path,
    build_category_page_description, build_category_page_document, build_category_page_title,
    build_category_path, build_home_page_canonical_path, build_home_page_description,
    build_home_page_document, build_home_page_title, build_static_page_canonical_path,
    build_static_page_description, build_static_page_document, build_static_page_title,
};
use infra::DynArtifactSnapshot;
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{StatusCode, path_param, request, route},
    view::{Unescaped, View, component, view},
};

use crate::topcoat_runtime::ArtifactReaderContext;
use web::generated_content::{
    CODE_HIGHLIGHT_SCRIPT, HIGHLIGHT_SCRIPT_URL, HIGHLIGHT_STYLESHEET_URL, KATEX_SCRIPT_INTEGRITY,
    KATEX_SCRIPT_URL, KATEX_STYLESHEET_INTEGRITY, KATEX_STYLESHEET_URL, MATH_RENDER_SCRIPT,
};

const ABOUT_PAGE_KEY: &str = "about";
const NOT_FOUND_TITLE: &str = "ページが見つかりません";
const NOT_FOUND_DESCRIPTION: &str = "お探しのページは見つかりませんでした。";
const STYLESHEET_PATH: &str = "/pkg/web.css";

path_param!(category_name);

#[route(GET "/")]
pub(crate) async fn home(cx: &Cx) -> Result<View> {
    let snapshot = match request_snapshot(cx).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("Home page artifact snapshot failed: {error}");
            return view! {
                internal_server_error_page(
                    title: build_home_page_title(web::SITE_NAME),
                    description: "公開済みの記事を読み込めませんでした。"
                        .to_string(),
                    canonical_path: "/".to_string(),
                    message: "記事の読み込みに失敗しました"
                )
            };
        }
    };
    let document = async {
        let article_index = snapshot.read_article_index().await?;
        let site_metadata = snapshot.read_site_metadata().await?;
        let home_fragment = match snapshot.read_home_fragment().await {
            Ok(fragment) => Some(fragment),
            Err(error) if error.is_not_found() => None,
            Err(error) => return Err(error),
        };

        build_home_page_document(&article_index, &site_metadata, home_fragment.as_ref())
            .map_err(Into::into)
    }
    .await;

    match document {
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

async fn request_snapshot(cx: &Cx) -> Result<DynArtifactSnapshot> {
    match try_request_context::<DynArtifactSnapshot>(cx) {
        Some(snapshot) => Ok(snapshot.clone()),
        None => Ok(app_context::<ArtifactReaderContext>(cx)
            .0
            .snapshot()
            .await?),
    }
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
            title: title,
            description: description,
            canonical_url: canonical_url,
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

#[component]
async fn article_card(article: &SiteArticleCard) -> Result {
    let article_href = build_article_path(&article.category, &article.slug);
    let description = article
        .description
        .as_deref()
        .unwrap_or("説明はまだありません。");
    let created_at_label = web::format::format_display_date(&article.created_at);
    let updated_at_label = web::format::format_display_date(&article.updated_at);

    view! {
        <article class="min-w-0">
            <a
                href=(article_href)
                class="group block text-inherit no-underline focus-visible:rounded-xl focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
                aria-label=(article.title.as_str())
            >
                <div
                    class="flex flex-col gap-3 rounded-xl border border-border/80 bg-card/90 p-5 text-card-foreground shadow-[0_10px_30px_rgb(0_0_0/0.22)] transition-[transform,box-shadow,border-color] duration-300 group-hover:-translate-y-0.5 group-hover:border-primary group-hover:shadow-[0_16px_36px_rgb(0_0_0/0.32)] group-focus-visible:border-primary"
                >
                    <div
                        class="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground sm:text-sm"
                    >
                        <span
                            class="inline-flex w-fit items-center rounded-md border border-primary/40 bg-background/40 px-2.5 py-0.5 text-xs font-semibold text-primary transition-colors focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2"
                        >
                            (&article.category_display_name)
                        </span>
                        <span class="flex flex-wrap items-center gap-x-1.5 gap-y-1">
                            <span>
                                "公開 "
                                <time datetime=(article.created_at.as_str())>
                                    (created_at_label)
                                </time>
                            </span>
                            <span aria-hidden="true">"/"</span>
                            <span>
                                "更新 "
                                <time datetime=(article.updated_at.as_str())>
                                    (updated_at_label)
                                </time>
                            </span>
                        </span>
                    </div>

                    <h3
                        class="m-0 text-xl leading-snug font-semibold transition-colors group-hover:text-primary group-focus-visible:text-primary"
                    >
                        (article.title.as_str())
                    </h3>
                    <p class="m-0 leading-7 text-muted-foreground">(description)</p>

                    if !article.tags.is_empty() {
                        <ul
                            class="m-0 flex list-none flex-wrap gap-2 p-0"
                            aria-label="タグ"
                        >
                            for tag in &article.tags {
                                <li>
                                    <span
                                        class="inline-flex w-fit items-center rounded-md border border-transparent bg-muted px-2.5 py-0.5 text-xs font-semibold text-muted-foreground transition-colors hover:bg-muted/80 focus:outline-hidden focus:ring-2 focus:ring-ring focus:ring-offset-2"
                                    >
                                        (format!("#{tag}"))
                                    </span>
                                </li>
                            }
                        </ul>
                    }
                </div>
            </a>
        </article>
    }
}

#[route(GET "/{category_name}")]
pub(crate) async fn category_page(cx: &Cx) -> Result<View> {
    let category_param = path_param::<CategoryName>(cx);
    let requested_path = request::uri(cx).path().to_string();
    let category = match DomainCategory::from_str(category_param) {
        Ok(category) => category,
        Err(_) => {
            return view! { not_found_page(canonical_path: requested_path) };
        }
    };

    let snapshot = match request_snapshot(cx).await {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("Category page artifact snapshot failed for {category_param}: {error}");
            return view! {
                internal_server_error_page(
                    title: format!("{category_param} | {}", web::SITE_NAME),
                    description: format!("{category_param} カテゴリの記事一覧です。"),
                    canonical_path: requested_path,
                    message: "カテゴリの読み込みに失敗しました"
                )
            };
        }
    };

    match snapshot.read_category_document(&category).await {
        Ok(artifact) => match build_category_page_document(&artifact) {
            Ok(document) => view! { category_document(document: document) },
            Err(error) => {
                eprintln!("Category page artifact is invalid for {category_param}: {error}");
                view! {
                    internal_server_error_page(
                        title: format!("{category_param} | {}", web::SITE_NAME),
                        description: format!("{category_param} カテゴリの記事一覧です。"),
                        canonical_path: requested_path,
                        message: "カテゴリの読み込みに失敗しました"
                    )
                }
            }
        },
        Err(error) if error.is_not_found() => {
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
            title: title,
            description: description.clone(),
            canonical_url: canonical_url,
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
pub(crate) async fn about(cx: &Cx) -> Result<View> {
    let snapshot = request_snapshot(cx).await?;
    let page = PageKey::new(ABOUT_PAGE_KEY.to_string())?;

    match snapshot.read_page_document(&page).await {
        Ok(artifact) => match build_static_page_document(&artifact) {
            Ok(document) => view! { about_document(document: document) },
            Err(error) => {
                eprintln!("About page artifact is invalid: {error}");
                view! {
                    internal_server_error_page(
                        title: format!("About | {}", web::SITE_NAME),
                        description: "About ページです。".to_string(),
                        canonical_path: "/about".to_string(),
                        message: "ページの読み込みに失敗しました"
                    )
                }
            }
        },
        Err(error) if error.is_not_found() => {
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
            title: title,
            description: description,
            canonical_url: canonical_url,
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

#[component]
async fn not_found_page(canonical_path: String) -> Result {
    let canonical_url = web::build_site_url(&canonical_path);

    view! {
        site_shell(
            status: StatusCode::NOT_FOUND,
            title: format!("{NOT_FOUND_TITLE} | {}", web::SITE_NAME),
            description: NOT_FOUND_DESCRIPTION.to_string(),
            canonical_url: canonical_url,
            current_path: canonical_path,
            <div>"ページが見つかりませんでした。"</div>
        )
    }
}

#[component]
async fn internal_server_error_page(
    title: String,
    description: String,
    canonical_path: String,
    message: &'static str,
) -> Result {
    let canonical_url = web::build_site_url(&canonical_path);

    view! {
        site_shell(
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: title,
            description: description,
            canonical_url: canonical_url,
            current_path: canonical_path,
            <div
                class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] rounded-xl bg-secondary p-8 text-center text-muted-foreground"
            >
                (message)
            </div>
        )
    }
}

#[component]
async fn site_shell(
    status: StatusCode,
    title: String,
    description: String,
    canonical_url: String,
    current_path: String,
    child: View,
) -> Result {
    let year = chrono::Local::now().year();
    let stylesheet_href = stylesheet_href();
    let math_render_script = Unescaped::new_unchecked(MATH_RENDER_SCRIPT);
    let code_highlight_script = Unescaped::new_unchecked(CODE_HIGHLIGHT_SCRIPT);

    view! {
        (status)
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(title.clone())</title>
                <meta name="description" content=(description.clone())>
                <link rel="canonical" href=(canonical_url.clone())>
                <meta property="og:title" content=(title)>
                <meta property="og:description" content=(description)>
                <meta property="og:url" content=(canonical_url)>
                <meta property="og:type" content="website">
                <link rel="stylesheet" href=(stylesheet_href)>
                <link
                    rel="icon"
                    href="/favicon.ico?v=f544a69c"
                    type="image/x-icon"
                    sizes="16x16 32x32 48x48"
                >
                <link
                    rel="stylesheet"
                    href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.7.2/css/all.min.css"
                >
                <link
                    rel="stylesheet"
                    href=(KATEX_STYLESHEET_URL)
                    integrity=(KATEX_STYLESHEET_INTEGRITY)
                    crossorigin="anonymous"
                >
                <script
                    defer=""
                    src=(KATEX_SCRIPT_URL)
                    integrity=(KATEX_SCRIPT_INTEGRITY)
                    crossorigin="anonymous"
                    onload="window.okawakScheduleMathRender && window.okawakScheduleMathRender();"
                ></script>
                <script>(math_render_script)</script>
                <link rel="stylesheet" href=(HIGHLIGHT_STYLESHEET_URL)>
                <script
                    defer=""
                    src=(HIGHLIGHT_SCRIPT_URL)
                    onload="window.okawakScheduleCodeHighlight && window.okawakScheduleCodeHighlight();"
                ></script>
                <script>(code_highlight_script)</script>
            </head>
            <body>
                <div class="flex min-h-dvh flex-col text-foreground">
                    <header
                        class="sticky top-0 z-50 h-[var(--site-header-height)] border-b border-border/60 bg-[image:var(--site-header-background)] shadow-[0_8px_24px_rgb(0_0_0/0.45)] backdrop-blur-sm"
                    >
                        <div
                            class="relative mx-auto flex h-full max-w-[var(--site-content-width)] items-center justify-between gap-3 px-4 sm:px-6"
                        >
                            <a
                                href="/"
                                class="min-w-0 text-foreground no-underline transition-colors hover:text-primary focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
                            >
                                <h1
                                    class="m-0 truncate text-xl leading-tight font-bold sm:text-2xl"
                                >
                                    (web::SITE_NAME)
                                </h1>
                            </a>
                            <nav aria-label="メインナビゲーション">
                                <ul class="m-0 flex list-none items-center gap-2 p-0">
                                    <li>
                                        <a
                                            href="/"
                                            aria-current=(if current_path == "/" {
                                                Some("page")
                                            } else {
                                                None
                                            })
                                        >
                                            "ホーム"
                                        </a>
                                    </li>
                                    <li>
                                        <a
                                            href="/about"
                                            aria-current=(if current_path == "/about" {
                                                Some("page")
                                            } else {
                                                None
                                            })
                                        >
                                            "About"
                                        </a>
                                    </li>
                                </ul>
                            </nav>
                        </div>
                    </header>
                    <main class="content-container flex-1">(child)</main>
                    <footer
                        class="border-t border-border bg-gradient-to-r from-card to-background px-4 py-8 text-center text-sm text-muted-foreground"
                    >
                        <div class="mx-auto max-w-[var(--site-content-width)]">
                            <p class="my-2 leading-relaxed">
                                (format!("© {year} okawak. All Rights Reserved."))
                            </p>
                            <p class="my-2 leading-relaxed">
                                <small>
                                    "Powered by "
                                    <a
                                        href="https://github.com/tokio-rs/topcoat"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    >
                                        "Topcoat"
                                    </a>
                                </small>
                            </p>
                        </div>
                    </footer>
                </div>
            </body>
        </html>
    }
}

fn stylesheet_href() -> String {
    let hash = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(std::path::Path::to_owned))
        .and_then(|directory| {
            [directory.join("hash.txt"), directory.join("../hash.txt")]
                .into_iter()
                .find_map(|path| std::fs::read_to_string(path).ok())
        })
        .and_then(|hashes| {
            hashes.lines().find_map(|line| {
                let (name, hash) = line.trim().split_once(':')?;
                (name == "css")
                    .then(|| hash.trim().to_string())
                    .filter(|hash| !hash.is_empty())
            })
        });

    match hash {
        Some(hash) => format!("/pkg/web.{hash}.css"),
        None => STYLESHEET_PATH.to_string(),
    }
}
