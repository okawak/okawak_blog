use super::{bookmark::BookmarkEnricher, html::convert_markdown_to_html};
use crate::{error::Result, links};
use tracing::warn;

pub(super) async fn render(
    markdown: &str,
    link_index: &links::Index,
    enrich: &BookmarkEnricher,
) -> Result<String> {
    let markdown = links::resolve_internal_links(markdown, link_index);
    let html = convert_markdown_to_html(&markdown)?;
    let fallback = html.clone();
    Ok(enrich(html).await.unwrap_or_else(|error| {
        warn!(%error, "failed to enrich bookmarks");
        fallback
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        classify::{ClassifiedFiles, ParsedArticleFile},
        error::PublishError,
        vault::{ContentKind, ObsidianFrontMatter},
    };
    use domain::{Category, SectionPath, Slug};
    use indoc::indoc;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_render_converts_internal_links_to_html() {
        let files = ClassifiedFiles {
            articles: vec![
                parsed_article("notes/article", Category::Tech, "def456"),
                parsed_article("notes/reference", Category::Daily, "ghi789"),
            ],
            ..Default::default()
        };
        let link_index = links::Index::from_classified_files(&files);
        let enrich: BookmarkEnricher = Arc::new(|html| Box::pin(async move { Ok(html) }));
        let markdown = indoc! {r#"
            # My Article

            This is a test with [[article|link]] and **bold** text.

            ## Section Two

            - Item with [[reference]]
            - Regular item
        "#};

        let html = render(markdown, &link_index, &enrich).await.unwrap();

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/tech/def456\">link</a>"));
        assert!(html.contains("<a href=\"/daily/ghi789\">reference</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }

    #[tokio::test]
    async fn test_render_falls_back_to_html_when_bookmark_enrichment_fails() {
        let link_index = links::Index::default();
        let enrich: BookmarkEnricher = Arc::new(|_html| {
            Box::pin(async { Err(PublishError::Parse("enrichment failed".to_string())) })
        });

        let html = render("# Hello", &link_index, &enrich).await.unwrap();

        assert_eq!(html, "<h1>Hello</h1>\n");
    }

    fn parsed_article(source_key: &str, category: Category, slug: &str) -> ParsedArticleFile {
        ParsedArticleFile {
            category,
            slug: Slug::new(slug.to_string()).unwrap(),
            source_key: source_key.to_string(),
            section_path: SectionPath::default(),
            markdown_body: String::new(),
            front_matter: ObsidianFrontMatter {
                title: "Article".to_string(),
                kind: ContentKind::Article,
                tags: None,
                summary: None,
                is_completed: true,
                priority: None,
                created: "2025-01-01T00:00:00+09:00".to_string(),
                updated: "2025-01-01T00:00:00+09:00".to_string(),
                category: Some(category.as_str().to_string()),
                page: None,
            },
        }
    }
}
