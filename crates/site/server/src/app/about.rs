use domain::{
    PageKey, StaticPageDocument, build_static_page_canonical_path, build_static_page_description,
    build_static_page_title,
};
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, page},
    view::{Unescaped, View, ViewExt, component, view},
};

use super::page_loader;
use crate::shell::{ShellMetadata, internal_server_error_page, not_found_page, site_shell};

const ABOUT_PAGE_KEY: &str = "about";

#[page]
async fn about(cx: &Cx) -> Result<impl View> {
    let page = PageKey::new(ABOUT_PAGE_KEY.to_string())?;

    match page_loader(cx).loader().load_static_page(&page).await {
        Ok(Some(document)) => Ok(view! { about_document(document: document) }.boxed()),
        Ok(None) => Ok(view! { not_found_page(canonical_path: "/about".to_string()) }.boxed()),
        Err(error) => {
            tracing::error!(%error, page = ABOUT_PAGE_KEY, "static page artifact read failed");
            Ok(view! {
                internal_server_error_page(
                    title: format!("About | {}", crate::SITE_NAME),
                    description: "About ページです。".to_string(),
                    canonical_path: "/about".to_string(),
                    message: "ページの読み込みに失敗しました"
                )
            }
            .boxed())
        }
    }
}

#[component]
async fn about_document(document: StaticPageDocument) -> Result<impl View> {
    let title = build_static_page_title(&document, crate::SITE_NAME);
    let description = build_static_page_description(&document);
    let canonical_url = crate::build_site_url(&build_static_page_canonical_path(&document));
    let page_title = document.title;
    // The publish pipeline escapes raw Markdown HTML and neutralizes unsafe href schemes before
    // persisting this fragment. It is therefore the trusted HTML boundary for Topcoat as well.
    let html = Unescaped::new_unchecked(document.html);

    Ok(view! {
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
    })
}
