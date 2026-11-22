use crate::i18n::{Message, t};
use crate::metadata::*;
use blog_domain::Locale;
pub(crate) mod article_slug;

use blog_domain::{Category, CategoryPageDocument, build_category_page_canonical_path};
use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{StatusCode, error::not_found, module_param, page, raw_path_params, request},
    view::{Unescaped, View, ViewExt, component, view},
};

use crate::app::load_category;
use crate::{
    category_articles::category_articles,
    shell::{internal_server_error_page, site_shell},
};

module_param!(category_name: Category else not_found);

#[page]
async fn category_page(cx: &Cx) -> Result<impl View> {
    let locale = *super::language(cx)?;
    let category = *category_name(cx)?;
    match load_category(cx, category, locale).await.clone() {
        Ok(Some(presentation)) => {
            Ok(view! { category_document(presentation: presentation, locale: locale) }.boxed())
        }
        Ok(None) => Err(not_found().into()),
        Err(error) => {
            let requested_path = request::uri(cx).path().to_string();
            // Preserve the requested spelling in the error title (e.g. /TECH).
            let category_param = raw_path_params(cx)
                .find_map(|(name, value)| (name == "category_name").then_some(value.as_str()))
                .unwrap_or(category.as_str());
            tracing::error!(
                %error,
                category = category_param,
                "category page artifact read failed"
            );
            let title = format!("{category_param} | {}", t(locale, Message::SiteName));
            Ok(view! {
                internal_server_error_page(
                    title: title,
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
    cx: &Cx,
    presentation: std::sync::Arc<crate::page_loader::Presentation<CategoryPageDocument>>,
    locale: Locale,
) -> Result<impl View> {
    let document = &presentation.document;
    let locales = std::sync::Arc::clone(&presentation.locales);
    let title = build_category_page_title(document, locale);
    let description = build_category_page_description(document, locale);
    let canonical_path = locale.path(&build_category_page_canonical_path(document));
    let category = document.category.to_string();
    let locale_arg = locale.to_string();
    let page_title = document.title.clone();
    // The publish pipeline escapes raw Markdown HTML and neutralizes unsafe href schemes before
    // persisting this fragment. It is therefore the trusted HTML boundary for Topcoat as well.
    let landing_html = Unescaped::new_unchecked(document.html.clone());

    Ok(view! {
        site_shell(
            status: StatusCode::OK,
            metadata: Metadata::new().title(title).description(description.clone()),
            locale: locale,
            canonical_path: canonical_path,
            locales: locales,
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
