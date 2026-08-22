use super::{bookmark::BookmarkEnricher, html::convert_markdown_to_html};
use crate::links;

pub(super) async fn render(
    markdown: &str,
    link_index: &links::Index,
    enrich: &BookmarkEnricher,
) -> String {
    let html = convert_markdown_to_html(markdown, link_index);
    enrich(html).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        classify::{ClassifiedFiles, ParsedArticleFile},
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
        let enrich = passthrough_bookmark_enricher();
        let markdown = indoc! {r#"
            # My Article

            This is a test with [[article|link]] and **bold** text.

            ## Section Two

            - Item with [[reference]]
            - Regular item
        "#};

        let html = render(markdown, &link_index, &enrich).await;

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/tech/def456\">link</a>"));
        assert!(html.contains("<a href=\"/daily/ghi789\">reference</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }

    #[tokio::test]
    async fn test_render_converts_internal_embeds_to_images() {
        let files = ClassifiedFiles {
            articles: vec![parsed_article("notes/article", Category::Tech, "def456")],
            ..Default::default()
        };
        let link_index = links::Index::from_classified_files(&files);
        let enrich = passthrough_bookmark_enricher();

        let html = render(
            "Embed ![[article]] and ![[article|Alt text]].",
            &link_index,
            &enrich,
        )
        .await;

        assert!(html.contains(r#"<img src="/tech/def456" alt="article" />"#));
        assert!(html.contains(r#"<img src="/tech/def456" alt="Alt text" />"#));
    }

    #[tokio::test]
    async fn test_render_resolves_escaped_piped_wikilinks_inside_tables() {
        let files = ClassifiedFiles {
            articles: vec![parsed_article("notes/article", Category::Tech, "def456")],
            ..Default::default()
        };
        let link_index = links::Index::from_classified_files(&files);
        let enrich = passthrough_bookmark_enricher();
        let markdown = indoc! {r#"
            | [[article\|Header link]] | ![[article\|Header embed]] |
            | --- | --- |
            | [[article\|Cell link]] | ![[article\|Cell embed]] |
        "#};

        let html = render(markdown, &link_index, &enrich).await;

        assert!(html.starts_with("<table>"), "unexpected html:\n{html}");
        assert!(
            html.contains(r#"<th><a href="/tech/def456">Header link</a></th>"#),
            "unexpected html:\n{html}"
        );
        assert!(
            html.contains(r#"<th><img src="/tech/def456" alt="Header embed" /></th>"#),
            "unexpected html:\n{html}"
        );
        assert!(
            html.contains(r#"<td><a href="/tech/def456">Cell link</a></td>"#),
            "unexpected html:\n{html}"
        );
        assert!(
            html.contains(r#"<td><img src="/tech/def456" alt="Cell embed" /></td>"#),
            "unexpected html:\n{html}"
        );
    }

    #[tokio::test]
    async fn test_render_escapes_wikilink_text_and_destination() {
        let files = ClassifiedFiles {
            articles: vec![parsed_article("notes/article", Category::Tech, "def456")],
            ..Default::default()
        };
        let link_index = links::Index::from_classified_files(&files);
        let enrich = passthrough_bookmark_enricher();

        let html = render(
            "[[article|Display & <script>]] and [[File \"quoted\"|missing]]",
            &link_index,
            &enrich,
        )
        .await;

        assert!(html.contains(r#"<a href="/tech/def456">Display &amp; &lt;script&gt;</a>"#));
        assert!(html.contains(r#"<a href="/File%20%22quoted%22">missing</a>"#));
    }

    fn passthrough_bookmark_enricher() -> BookmarkEnricher {
        Arc::new(|html| Box::pin(async move { html }))
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
