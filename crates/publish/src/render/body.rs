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
    use std::sync::Arc;
    #[tokio::test]
    async fn renders_public_links_tables_and_safe_display_text() {
        let index = links::Index::fixture();
        let enrich: BookmarkEnricher = Arc::new(|html| Box::pin(async move { html }));
        let html = render("# Article\n\n[Display & \\<script\\>](content:article) **bold**\n\n| Header | Image |\n| --- | --- |\n| [Reference](content:reference) | ![Alt](/content-assets/image.png) |", &index, &enrich).await;
        assert!(html.contains(r#"href="/tech/def456""#));
        assert!(html.contains("Display &amp; &lt;script&gt;"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<table>"));
        assert!(html.contains(r#"href="/daily/ghi789""#));
        assert!(html.contains(r#"src="/content-assets/image.png" alt="Alt""#));
    }
}
