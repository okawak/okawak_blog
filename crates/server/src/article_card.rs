//! Shared article card component used by listing pages.

use crate::i18n::{Message, t};
use domain::{Locale, SiteArticleCard, TagLabels, build_article_path};
use topcoat::{
    Result,
    view::{View, component, view},
};

use crate::components::badge::{BadgeVariant, badge};

#[component]
pub(crate) async fn article_card(
    article: &SiteArticleCard,
    locale: Locale,
    labels: &TagLabels,
) -> Result<impl View> {
    let article_href = locale.path(&build_article_path(&article.category, &article.slug));
    let description = article
        .description
        .as_deref()
        .unwrap_or(t(locale, Message::ArticleNoDescription));
    let created_at_label = crate::format::format_display_date(&article.created_at, locale);
    let updated_at_label = crate::format::format_display_date(&article.updated_at, locale);

    Ok(view! {
        <article
            id=(format!("article-{}-{}", article.category, article.slug))
            class="min-w-0"
        >
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
                        badge(
                            variant: BadgeVariant::Outline,
                            (crate::i18n::category_name(locale, article.category))
                        )
                        <span class="flex flex-wrap items-center gap-x-1.5 gap-y-1">
                            <span>
                                (t(locale, Message::ArticlePublished))
                                <time datetime=(article.created_at.as_str())>
                                    (created_at_label)
                                </time>
                            </span>
                            <span aria-hidden="true">"/"</span>
                            <span>
                                (t(locale, Message::ArticleUpdated))
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
                            aria-label=(t(locale, Message::ArticleTags))
                        >
                            for tag in &article.tags {
                                <li>
                                    badge(
                                        variant: BadgeVariant::Secondary,
                                        (format!("#{}", labels.get(tag).unwrap_or(tag)))
                                    )
                                </li>
                            }
                        </ul>
                    }
                </div>
            </a>
        </article>
    })
}
