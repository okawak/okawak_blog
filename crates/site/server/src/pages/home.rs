use domain::{
    HomePageDocument, build_category_path, build_home_page_canonical_path,
    build_home_page_description, build_home_page_title,
};
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, route},
    view::{Unescaped, View, component, view},
};

use super::page_loader;
use crate::{
    article_card::article_card,
    shell::{ShellMetadata, internal_server_error_page, site_shell},
};

#[route(GET "/")]
pub async fn home(cx: &Cx) -> Result<View> {
    match page_loader(cx).loader().load_home().await {
        Ok(document) => view! { home_document(document: document) },
        Err(error) => {
            tracing::error!(%error, "home page artifact read failed");
            view! {
                internal_server_error_page(
                    title: build_home_page_title(crate::SITE_NAME),
                    description: "公開済みの記事を読み込めませんでした。"
                        .to_string(),
                    canonical_path: "/".to_string(),
                    message: "記事の読み込みに失敗しました"
                )
            }
        }
    }
}

#[component]
async fn home_document(document: HomePageDocument) -> Result {
    let title = build_home_page_title(crate::SITE_NAME);
    let description = build_home_page_description(&document);
    let canonical_url = crate::build_site_url(build_home_page_canonical_path());
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
                        (crate::SITE_NAME)
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
