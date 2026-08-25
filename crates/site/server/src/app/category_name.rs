mod article_slug;

use std::str::FromStr;

use domain::{
    Category, CategoryPageDocument, build_category_page_canonical_path,
    build_category_page_description, build_category_page_title,
};
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, page, path_param, request},
    view::{Unescaped, View, component, view},
};

use super::page_loader;
use crate::{
    article_card::article_card,
    shell::{ShellMetadata, internal_server_error_page, not_found_page, site_shell},
};

path_param!(category_name);

#[page]
async fn category_page(cx: &Cx) -> Result<View> {
    let category_param = path_param::<CategoryName>(cx);
    let requested_path = request::uri(cx).path().to_string();
    let category = match Category::from_str(category_param) {
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
            tracing::error!(
                %error,
                category = category_param,
                "category page artifact read failed"
            );
            view! {
                internal_server_error_page(
                    title: format!("{category_param} | {}", crate::SITE_NAME),
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
    let title = build_category_page_title(&document, crate::SITE_NAME);
    let description = build_category_page_description(&document);
    let canonical_path = build_category_page_canonical_path(&document);
    let canonical_url = crate::build_site_url(&canonical_path);
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
