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
    use crate::error::PublishError;
    use indoc::indoc;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_render_converts_internal_links_to_html() {
        let link_index = links::Index::default();
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
        assert!(html.contains("<a href=\"/article\">link</a>"));
        assert!(html.contains("<a href=\"/reference\">reference</a>"));
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
}
