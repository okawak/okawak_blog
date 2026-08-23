//! Topcoat SSR pages introduced route by route during the migration.

use chrono::Datelike;
use domain::{
    PageKey, StaticPageDocument, build_static_page_canonical_path, build_static_page_description,
    build_static_page_document, build_static_page_title,
};
use infra::DynArtifactSnapshot;
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{StatusCode, route},
    view::{Unescaped, View, component, view},
};

use crate::topcoat_runtime::ArtifactReaderContext;

const ABOUT_PAGE_KEY: &str = "about";
const NOT_FOUND_TITLE: &str = "ページが見つかりません";
const NOT_FOUND_DESCRIPTION: &str = "お探しのページは見つかりませんでした。";
const STYLESHEET_PATH: &str = "/pkg/web.css";

#[route(GET "/about")]
pub(crate) async fn about(cx: &Cx) -> Result<View> {
    let snapshot = match try_request_context::<DynArtifactSnapshot>(cx) {
        Some(snapshot) => snapshot.clone(),
        None => {
            app_context::<ArtifactReaderContext>(cx)
                .0
                .snapshot()
                .await?
        }
    };
    let page = PageKey::new(ABOUT_PAGE_KEY.to_string())?;

    match snapshot.read_page_document(&page).await {
        Ok(artifact) => match build_static_page_document(&artifact) {
            Ok(document) => view! { about_document(document: document) },
            Err(error) => {
                eprintln!("About page artifact is invalid: {error}");
                view! { internal_server_error_page() }
            }
        },
        Err(error) if error.is_not_found() => view! { not_found_page() },
        Err(error) => {
            eprintln!("About page artifact read failed: {error}");
            view! { internal_server_error_page() }
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
async fn not_found_page() -> Result {
    view! {
        site_shell(
            status: StatusCode::NOT_FOUND,
            title: format!("{NOT_FOUND_TITLE} | {}", web::SITE_NAME),
            description: NOT_FOUND_DESCRIPTION.to_string(),
            canonical_url: web::build_site_url("/about"),
            <div>"ページが見つかりませんでした。"</div>
        )
    }
}

#[component]
async fn internal_server_error_page() -> Result {
    view! {
        site_shell(
            status: StatusCode::INTERNAL_SERVER_ERROR,
            title: format!("About | {}", web::SITE_NAME),
            description: "About ページです。".to_string(),
            canonical_url: web::build_site_url("/about"),
            <div
                class="mx-auto my-8 w-[calc(100%-2rem)] max-w-[var(--site-content-width)] rounded-xl bg-secondary p-8 text-center text-muted-foreground"
            >
                "ページの読み込みに失敗しました"
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
    child: View,
) -> Result {
    let year = chrono::Local::now().year();
    let stylesheet_href = stylesheet_href();

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
                                    <li><a href="/">"ホーム"</a></li>
                                    <li><a href="/about" aria-current="page">"About"</a></li>
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
