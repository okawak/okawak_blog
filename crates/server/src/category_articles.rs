//! The category list is the only region re-rendered when its filter changes.

use std::str::FromStr;

use crate::i18n::{Message, t};
use domain::{Category, Locale};
use topcoat::{
    Result,
    context::Cx,
    router::error::{bad_request, not_found},
    runtime::{Event, shard, signal},
    view::{View, view},
};

use crate::{
    app::load_category,
    article_card::article_card,
    article_filter::{MAX_QUERY_CHARS, filter_sections},
};

#[shard]
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
                <label for="category-article-query" class="font-semibold">
                    (t(locale, Message::FilterLabel))
                </label>
                <input
                    id="category-article-query"
                    type="search"
                    maxlength=(MAX_QUERY_CHARS)
                    placeholder=(t(locale, Message::FilterPlaceholder))
                    class="w-full rounded-lg border border-border bg-card px-4 py-3 text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                    :value=$(query.get())
                    @input=$(|event: Event| query.set(event.target.value))
                >
                <p role="status" class="m-0 text-sm text-muted-foreground">
                    (crate::i18n::article_count(locale, count))
                </p>
            </div>
            if count == 0 {
                <p>(t(locale, Message::EmptyMatches))</p>
            }
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
