use super::{bookmark::BookmarkEnricher, html::convert_markdown_to_html};
use crate::{error::Result, links};
use log::warn;

pub(super) async fn render(
    markdown: &str,
    link_index: &links::Index,
    enrich: &BookmarkEnricher,
) -> Result<String> {
    let markdown = links::resolve_internal_links(markdown, link_index);
    let html = convert_markdown_to_html(&markdown)?;
    let fallback = html.clone();
    Ok(enrich(html).await.unwrap_or_else(|error| {
        warn!("Warning: Failed to convert simple bookmarks to rich bookmarks: {error}");
        fallback
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PublishError;
    use std::sync::Arc;

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
