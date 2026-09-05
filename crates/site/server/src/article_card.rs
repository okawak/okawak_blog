//! Shared article card component used by listing pages.

use domain::{SiteArticleCard, build_article_path};
use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
pub(crate) async fn article_card(article: &SiteArticleCard) -> Result<impl View> {
    let article_href = build_article_path(&article.category, &article.slug);
    let description = article
        .description
        .as_deref()
        .unwrap_or("説明はまだありません。");
    let created_at_label = crate::format::format_display_date(&article.created_at);
    let updated_at_label = crate::format::format_display_date(&article.updated_at);

    Ok(view! {
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
    })
}
