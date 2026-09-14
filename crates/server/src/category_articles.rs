//! The category list is the only region re-rendered when its filter changes.

use std::str::FromStr;

use domain::Category;
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
pub(crate) async fn category_articles(cx: &Cx, category: String) -> Result<impl View> {
    // Shard arguments are user input even when the first render supplied them.
    let category = Category::from_str(&category).map_err(|_| bad_request("invalid category"))?;
    let query = signal(cx, String::new);
    let mut document = load_category(cx, category)
        .await
        .clone()
        .map_err(|error| {
            tracing::error!(%error, %category, "category article list read failed");
            topcoat::router::error::internal_server_error(std::io::Error::other(error))
        })?
        .ok_or_else(not_found)?;
    filter_sections(&mut document.sections, query.read());
    let count: usize = document
        .sections
        .iter()
        .map(|section| section.articles.len())
        .sum();

    Ok(view! {
        <section
            id="category-articles"
            class="grid gap-6"
            aria-label="カテゴリの記事"
        >
            <div class="grid gap-3">
                <label for="category-article-query" class="font-semibold">
                    "記事を絞り込む"
                </label>
                <input
                    id="category-article-query"
                    type="search"
                    maxlength=(MAX_QUERY_CHARS)
                    placeholder="タイトル・説明・タグ"
                    class="w-full rounded-lg border border-border bg-card px-4 py-3 text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                    :value=$(query.get())
                    @input=$(|event: Event| query.set(event.target.value))
                >
                <p role="status" class="m-0 text-sm text-muted-foreground">
                    (format!("{count}件の記事"))
                </p>
            </div>
            if count == 0 {
                <p>"該当する記事はありません。"</p>
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
                        (&section.heading)
                    </h2>
                    <div class="grid gap-4">
                        for article in &section.articles {
                            article_card(article: article)
                        }
                    </div>
                </section>
            }
        </section>
    })
}
