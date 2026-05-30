use crate::types::{
    ParsedArticleFile, ParsedCategoryFile, ParsedPageFile, RenderedArticle,
    RenderedCategoryLanding, RenderedPage,
};
use crate::BookmarkEnricher;
use crate::error::Result;
use artifacts::CategoryLandingMetadata;
use domain::{ArticleBody, ArticleMeta, ArticleMetaInput, Category, PageArtifactDocument, Slug, Title};
use ingest::{FileMapping, convert_markdown_to_html, convert_obsidian_links};
use log::warn;

async fn render_html(
    markdown_body: &str,
    file_mapping: &FileMapping,
    enrich: &BookmarkEnricher,
) -> Result<String> {
    let markdown_with_links = convert_obsidian_links(markdown_body, file_mapping);
    let html_body = convert_markdown_to_html(&markdown_with_links)?;
    let fallback = html_body.clone();
    let html = enrich(html_body).await.unwrap_or_else(|e| {
        warn!("Warning: Failed to convert simple bookmarks to rich bookmarks: {e}");
        fallback
    });
    Ok(html)
}

pub(crate) async fn render_article(
    parsed_file: ParsedArticleFile,
    file_mapping: &FileMapping,
    enrich: BookmarkEnricher,
) -> Result<RenderedArticle> {
    let html = render_html(&parsed_file.markdown_body, file_mapping, &enrich).await?;
    let meta = ArticleMeta::new(ArticleMetaInput {
        slug: Slug::new(parsed_file.slug)?,
        title: Title::new(parsed_file.front_matter.title)?,
        category: parsed_file.category,
        section_path: parsed_file.section_path,
        description: parsed_file.front_matter.summary,
        tags: parsed_file.front_matter.tags.unwrap_or_default(),
        priority: parsed_file.front_matter.priority,
        created_at: parsed_file.front_matter.created,
        updated_at: parsed_file.front_matter.updated,
    })?;
    let body = ArticleBody::new(html)?;
    Ok(RenderedArticle {
        meta,
        html: body.html,
    })
}

pub(crate) async fn render_page(
    parsed_file: ParsedPageFile,
    file_mapping: &FileMapping,
    enrich: BookmarkEnricher,
) -> Result<RenderedPage> {
    let html = render_html(&parsed_file.markdown_body, file_mapping, &enrich).await?;
    Ok(RenderedPage {
        document: PageArtifactDocument {
            page: parsed_file.page,
            title: parsed_file.front_matter.title,
            description: parsed_file.front_matter.summary,
            html,
            updated_at: parsed_file.front_matter.updated,
        },
    })
}

pub(crate) async fn render_category(
    parsed_file: ParsedCategoryFile,
    file_mapping: &FileMapping,
    enrich: BookmarkEnricher,
) -> Result<RenderedCategoryLanding> {
    let html = render_html(&parsed_file.markdown_body, file_mapping, &enrich).await?;
    let html = if html.trim().is_empty() {
        build_fallback_category_landing_html(
            parsed_file.category,
            &parsed_file.front_matter.title,
            parsed_file.front_matter.summary.as_deref(),
        )
    } else {
        html
    };
    Ok(RenderedCategoryLanding {
        metadata: CategoryLandingMetadata {
            category: parsed_file.category,
            title: parsed_file.front_matter.title,
            description: parsed_file.front_matter.summary,
            updated_at: parsed_file.front_matter.updated,
        },
        html,
    })
}

fn build_fallback_category_landing_html(
    category: Category,
    title: &str,
    description: Option<&str>,
) -> String {
    let heading = if title.trim().is_empty() {
        category.display_name()
    } else {
        title.trim()
    };

    let body = description
        .filter(|d| !d.trim().is_empty())
        .map(str::trim)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}カテゴリの記事一覧です。", category.display_name()));
    let heading = html_escape(heading);
    let body = html_escape(&body);
    format!("<article><h1>{heading}</h1><p>{body}</p></article>")
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_fallback_category_landing_html_uses_title_and_summary() {
        let html = build_fallback_category_landing_html(
            Category::Tech,
            "Tech",
            Some("Technology landing"),
        );

        assert!(html.contains("<h1>Tech</h1>"));
        assert!(html.contains("<p>Technology landing</p>"));
    }

    #[test]
    fn test_build_fallback_category_landing_html_falls_back_to_category_display_name() {
        let html = build_fallback_category_landing_html(Category::Physics, "   ", None);

        assert!(html.contains("<h1>物理学</h1>"));
        assert!(html.contains("物理学カテゴリの記事一覧です。"));
    }

    #[test]
    fn test_build_fallback_category_landing_html_escapes_frontmatter_text() {
        let html = build_fallback_category_landing_html(
            Category::Tech,
            "<script>alert(1)</script>",
            Some("\"quoted\" & <tag>"),
        );

        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("&quot;quoted&quot; &amp; &lt;tag&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }
}
