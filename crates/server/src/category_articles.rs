//! The category list is the only region re-rendered when its filter changes.

use std::str::FromStr;

use crate::i18n::{Message, t};
use domain::{Category, Locale};
use topcoat::{
    Result,
    context::Cx,
    router::error::{bad_request, not_found},
    runtime::{Event, shard, signal},
    view::{View, attributes, view},
};

use crate::{
    app::load_category,
    article_card::article_card,
    article_filter::{MAX_QUERY_CHARS, filter_sections},
    components::{
        alert::{alert, alert_description},
        badge::{BadgeVariant, badge},
        field::{field, field_description, field_label},
        input::input,
    },
};

#[shard("/_topcoat/shards/category-articles")]
pub(crate) async fn category_articles(
    cx: &Cx,
    category: String,
    locale: String,
) -> Result<impl View> {
    // Shard arguments are user input even when the first render supplied them.
    let category = Category::from_str(&category).map_err(|_| bad_request("invalid category"))?;
    let locale = Locale::from_str(&locale).map_err(|_| bad_request("invalid locale"))?;
    let query = signal(cx, String::new);
    let presentation = load_category(cx, category, locale)
        .await
        .clone()
        .map_err(|error| {
            tracing::error!(%error, %category, "category article list read failed");
            topcoat::router::error::internal_server_error(std::io::Error::other(error))
        })?
        .ok_or_else(not_found)?;
    let crate::page_loader::Presentation {
        mut document,
        labels,
        ..
    } = presentation;
    filter_sections(&mut document.sections, query.read(), &labels);
    let count: usize = document
        .sections
        .iter()
        .map(|section| section.articles.len())
        .sum();

    Ok(view! {
        <section
            id="category-articles"
            class="grid gap-6"
            aria-label=(t(locale, Message::CategoryArticles))
        >
            <div class="grid gap-3">
                field(
                    field_label(
                        attrs: attributes! { for="category-article-query" },
                        (t(locale, Message::FilterLabel))
                    )
                    input(
                        attrs: attributes! {
                            id="category-article-query"
                            aria-describedby="category-filter-help"
                            type="search"
                            maxlength=(MAX_QUERY_CHARS)
                            placeholder=(t(locale, Message::FilterPlaceholder))
                            :value=$(query.get())
                            @input=$(|event: Event| query.set(event.target.value))
                        }
                    )
                    field_description(
                        attrs: attributes! { id="category-filter-help" },
                        (t(locale, Message::FilterPlaceholder))
                    )
                )
                badge(
                    variant: BadgeVariant::Secondary,
                    attrs: attributes! { role="status" },
                    (crate::i18n::article_count(locale, count))
                )
            </div>
            if count == 0 {
                alert(alert_description((t(locale, Message::EmptyMatches))))
            }
            #[key(section.section_path.segments().join("/"))]
            for section in &document.sections {
                <section
                    id=(format!(
                        "category-section-{}",
                        section.section_path.segments().join("/"),
                    ))
                    class="grid gap-4"
                >
                    <h2 class="m-0 text-xl font-semibold text-foreground">
                        (if section.section_path.is_empty() {
                            t(locale, Message::CategoryGeneral)
                        } else {
                            &section.heading
                        })
                    </h2>
                    <div class="grid gap-4">
                        #[key(article.slug.as_str())]
                        for article in &section.articles {
                            article_card(
                                article: article,
                                locale: locale,
                                labels: &labels
                            )
                        }
                    </div>
                </section>
            }
        </section>
    })
}
