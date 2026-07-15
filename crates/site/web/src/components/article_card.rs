use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::card::{Card, CardSize};
use crate::format::format_display_date;
use domain::{SiteArticleCard, build_article_path};
use leptos::prelude::*;
use leptos_router::components::A;

/// Site-specific article summary shared by list routes.
#[component]
pub fn ArticleCard(article: SiteArticleCard) -> impl IntoView {
    let article_href = build_article_path(&article.category, &article.slug);
    let title = article.title.as_str().to_string();
    let article_label = title.clone();
    let category = article.category_display_name;
    let description = article
        .description
        .unwrap_or_else(|| "説明はまだありません。".to_string());
    let tags = article.tags;
    let has_tags = !tags.is_empty();
    let created_at = article.created_at;
    let updated_at = article.updated_at;
    let created_at_label = format_display_date(&created_at);
    let updated_at_label = format_display_date(&updated_at);

    view! {
        <article class="min-w-0">
            <A
                href={article_href}
                {..}
                class="group block text-inherit no-underline focus-visible:rounded-xl focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-ring"
                attr:aria-label=article_label
            >
                <Card
                    size=CardSize::Sm
                    class="gap-3 border-border/80 bg-card/90 p-5 shadow-[0_10px_30px_rgb(0_0_0/0.22)] transition-[transform,box-shadow,border-color] duration-300 group-hover:-translate-y-0.5 group-hover:border-primary group-hover:shadow-[0_16px_36px_rgb(0_0_0/0.32)] group-focus-visible:border-primary"
                >
                    <div class="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground sm:text-sm">
                        <Badge
                            variant=BadgeVariant::Outline
                            class="border-primary/40 bg-background/40 text-primary"
                        >
                            {category}
                        </Badge>
                        <span class="flex flex-wrap items-center gap-x-1.5 gap-y-1">
                            <span>
                                {"公開 "} <time datetime=created_at>{created_at_label}</time>
                            </span>
                            <span aria-hidden="true">"/"</span>
                            <span>
                                {"更新 "} <time datetime=updated_at>{updated_at_label}</time>
                            </span>
                        </span>
                    </div>

                    <h3 class="m-0 text-xl leading-snug font-semibold transition-colors group-hover:text-primary group-focus-visible:text-primary">
                        {title}
                    </h3>
                    <p class="m-0 leading-7 text-muted-foreground">{description}</p>

                    <Show when=move || has_tags fallback=|| ()>
                        <ul class="m-0 flex list-none flex-wrap gap-2 p-0" aria-label="タグ">
                            {tags
                                .iter()
                                .map(|tag| {
                                    view! {
                                        <li>
                                            <Badge variant=BadgeVariant::Muted>
                                                {format!("#{tag}")}
                                            </Badge>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    </Show>
                </Card>
            </A>
        </article>
    }
}
