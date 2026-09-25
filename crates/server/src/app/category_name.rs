use crate::i18n::{Message, t};
use crate::metadata::*;
use domain::Locale;
pub(crate) mod article_slug;

use std::str::FromStr;

use domain::{Category, CategoryPageDocument, build_category_page_canonical_path};
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, page, path_param, request},
    view::{Unescaped, View, ViewExt, component, view},
};

use super::load_category;
use crate::{
    category_articles::category_articles,
    shell::{ShellMetadata, internal_server_error_page, not_found_page, site_shell},
};

path_param!(category_name);

#[page]
async fn category_page(cx: &Cx) -> Result<impl View> {
    render_category(cx, Locale::Ja, path_param::<CategoryName>(cx)).await
}

pub(crate) async fn render_category(
    cx: &Cx,
    locale: Locale,
    category_param: &str,
) -> Result<impl View> {
    let requested_path = request::uri(cx).path().to_string();
    let category = match Category::from_str(category_param) {
        Ok(category) => category,
        Err(_) => {
            return Ok(view! { cx => not_found_page(canonical_path: requested_path) }.boxed());
        }
    };

    match load_category(cx, category, locale).await.clone() {
        Ok(Some(presentation)) => Ok(
            view! { cx => category_document(presentation: presentation, locale: locale) }.boxed(),
        ),
        Ok(None) => Ok(view! { cx => not_found_page(canonical_path: requested_path) }.boxed()),
        Err(error) => {
            tracing::error!(
                %error,
                category = category_param,
                "category page artifact read failed"
            );
            Ok(view! {
                cx =>
                internal_server_error_page(
                    title: format!("{category_param} | {}", t(locale, Message::SiteName)),
                    description: t(locale, Message::ErrorCategory).to_string(),
                    canonical_path: requested_path,
                    message: Message::ErrorCategory
                )
            }
            .boxed())
        }
    }
}

#[component]
async fn category_document(
    presentation: crate::page_loader::Presentation<CategoryPageDocument>,
    locale: Locale,
) -> Result<impl View> {
    let crate::page_loader::Presentation {
        document, locales, ..
    } = presentation;
    let title = build_category_page_title(&document, locale);
    let description = build_category_page_description(&document, locale);
    let canonical_path = locale.path(&build_category_page_canonical_path(&document));
    let category = document.category.to_string();
    let locale_arg = locale.to_string();
    let page_title = document.title;
    // The publish pipeline escapes raw Markdown HTML and neutralizes unsafe href schemes before
    // persisting this fragment. It is therefore the trusted HTML boundary for Topcoat as well.
    let landing_html = Unescaped::new_unchecked(document.html);

    Ok(view! {
        site_shell(
            status: StatusCode::OK,
            metadata: ShellMetadata::website(
                locale,
                title,
                description.clone(),
                canonical_path,
            ).with_locales(locales),
            <div
                class="mx-auto grid min-h-full w-full max-w-[var(--site-content-width)] gap-6 px-4 py-8 text-left sm:px-6 sm:py-12"
            >
                <div
                    class="flex flex-col gap-3 rounded-xl border border-border/80 bg-gradient-to-b from-card to-secondary/70 p-6 text-card-foreground shadow-sm sm:p-8"
                >
                    <p class="m-0 text-sm tracking-[0.16em] text-primary uppercase">
                        (t(locale, Message::CategoryLabel))
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

                category_articles(category: category, locale: locale_arg)
            </div>
        )
    })
}
