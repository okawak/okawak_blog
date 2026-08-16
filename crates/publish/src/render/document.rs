use super::{bookmark::BookmarkEnricher, html::convert_markdown_to_html};
use crate::{
    classify::{ParsedArticleFile, ParsedCategoryFile, ParsedHomeFile, ParsedPageFile},
    error::Result,
    links,
};
use domain::{
    ArticleBody, ArticleMeta, ArticleMetaInput, Category, CategoryLandingMeta,
    CategoryLandingMetaInput, HomeFragmentArtifactDocument, PageArtifactDocument, Title,
};
use html_escape::encode_text;
use tracing::warn;

pub(crate) struct RenderedArticle {
    pub(crate) meta: ArticleMeta,
    pub(crate) html: String,
}

pub(crate) struct RenderedCategoryLanding {
    pub(crate) meta: CategoryLandingMeta,
    pub(crate) html: String,
}

async fn render_markdown(
    markdown: &str,
    link_index: &links::Index,
    enrich: &BookmarkEnricher,
) -> String {
    let html = convert_markdown_to_html(markdown, link_index);
    let fallback = html.clone();
    enrich(html).await.unwrap_or_else(|error| {
        warn!(%error, "failed to enrich bookmarks");
        fallback
    })
}

pub(crate) async fn render_article(
    parsed_file: ParsedArticleFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
) -> Result<RenderedArticle> {
    let html = render_markdown(&parsed_file.markdown_body, link_index, &enrich).await;
    let meta = ArticleMeta::new(ArticleMetaInput {
        slug: parsed_file.slug,
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

pub(crate) async fn render_category(
    parsed_file: ParsedCategoryFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
) -> Result<RenderedCategoryLanding> {
    let html = render_markdown(&parsed_file.markdown_body, link_index, &enrich).await;
    let title = normalize_category_title(parsed_file.category, &parsed_file.front_matter.title);
    let description = normalize_category_description(parsed_file.front_matter.summary.as_deref());
    let html = if html.trim().is_empty() {
        build_fallback_category_html(parsed_file.category, &title, description.as_deref())
    } else {
        html
    };
    let meta = CategoryLandingMeta::new(CategoryLandingMetaInput {
        category: parsed_file.category,
        title: Title::new(title)?,
        description,
        updated_at: parsed_file.front_matter.updated,
    })?;
    Ok(RenderedCategoryLanding { meta, html })
}

pub(crate) async fn render_home(
    parsed_file: ParsedHomeFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
) -> HomeFragmentArtifactDocument {
    let html = render_markdown(&parsed_file.markdown_body, link_index, &enrich).await;
    HomeFragmentArtifactDocument {
        title: parsed_file.front_matter.title,
        description: parsed_file.front_matter.summary,
        html,
        updated_at: parsed_file.front_matter.updated,
    }
}

pub(crate) async fn render_page(
    parsed_file: ParsedPageFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
) -> PageArtifactDocument {
    let html = render_markdown(&parsed_file.markdown_body, link_index, &enrich).await;
    PageArtifactDocument {
        page: parsed_file.page,
        title: parsed_file.front_matter.title,
        description: parsed_file.front_matter.summary,
        html,
        updated_at: parsed_file.front_matter.updated,
    }
}

fn normalize_category_title(category: Category, title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        category.display_name().to_owned()
    } else {
        title.to_owned()
    }
}

fn normalize_category_description(description: Option<&str>) -> Option<String> {
    description
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(str::to_owned)
}

fn build_fallback_category_html(
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
        .filter(|description| !description.trim().is_empty())
        .map(str::trim)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}カテゴリの記事一覧です。", category.display_name()));
    let heading = encode_text(heading);
    let body = encode_text(&body);
    format!("<article><h1>{heading}</h1><p>{body}</p></article>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        classify::ClassifiedFiles,
        error::PublishError,
        vault::{ContentKind, ObsidianFrontMatter},
    };
    use domain::{SectionPath, Slug};
    use indoc::indoc;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_render_markdown_converts_internal_links_to_html() {
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

        let html = render_markdown(markdown, &link_index, &enrich).await;

        assert!(html.contains("<h1>My Article</h1>"));
        assert!(html.contains("<a href=\"/tech/def456\">link</a>"));
        assert!(html.contains("<a href=\"/daily/ghi789\">reference</a>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<ul>"));
    }

    #[tokio::test]
    async fn test_render_markdown_converts_internal_embeds_to_images() {
        let files = ClassifiedFiles {
            articles: vec![parsed_article("notes/article", Category::Tech, "def456")],
            ..Default::default()
        };
        let link_index = links::Index::from_classified_files(&files);
        let enrich: BookmarkEnricher = Arc::new(|html| Box::pin(async move { Ok(html) }));

        let html = render_markdown(
            "Embed ![[article]] and ![[article|Alt text]].",
            &link_index,
            &enrich,
        )
        .await;

        assert!(html.contains(r#"<img src="/tech/def456" alt="article" />"#));
        assert!(html.contains(r#"<img src="/tech/def456" alt="Alt text" />"#));
    }

    #[tokio::test]
    async fn test_render_markdown_preserves_piped_wikilinks_inside_table_cells() {
        let files = ClassifiedFiles {
            articles: vec![parsed_article("notes/article", Category::Tech, "def456")],
            ..Default::default()
        };
        let link_index = links::Index::from_classified_files(&files);
        let enrich: BookmarkEnricher = Arc::new(|html| Box::pin(async move { Ok(html) }));
        let markdown = indoc! {r#"
            | [[article|Header link]] | ![[article|Header embed]] |
            | --- | --- |
            | [[article|Cell link]] | ![[article|Cell embed]] |
        "#};

        let html = render_markdown(markdown, &link_index, &enrich).await;

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
    async fn test_render_markdown_escapes_wikilink_text_and_destination() {
        let files = ClassifiedFiles {
            articles: vec![parsed_article("notes/article", Category::Tech, "def456")],
            ..Default::default()
        };
        let link_index = links::Index::from_classified_files(&files);
        let enrich: BookmarkEnricher = Arc::new(|html| Box::pin(async move { Ok(html) }));

        let html = render_markdown(
            "[[article|Display & <script>]] and [[File \"quoted\"|missing]]",
            &link_index,
            &enrich,
        )
        .await;

        assert!(html.contains(r#"<a href="/tech/def456">Display &amp; &lt;script&gt;</a>"#));
        assert!(html.contains(r#"<a href="/File%20%22quoted%22">missing</a>"#));
    }

    #[tokio::test]
    async fn test_render_markdown_falls_back_to_html_when_bookmark_enrichment_fails() {
        let link_index = links::Index::default();
        let enrich: BookmarkEnricher = Arc::new(|_html| {
            Box::pin(async { Err(PublishError::Parse("enrichment failed".to_string())) })
        });

        let html = render_markdown("# Hello", &link_index, &enrich).await;

        assert_eq!(html, "<h1>Hello</h1>\n");
    }

    #[test]
    fn test_build_fallback_category_html_uses_title_and_summary() {
        let html = build_fallback_category_html(Category::Tech, "Tech", Some("Technology landing"));

        assert!(html.contains("<h1>Tech</h1>"));
        assert!(html.contains("<p>Technology landing</p>"));
    }

    #[test]
    fn test_build_fallback_category_html_falls_back_to_category_display_name() {
        let html = build_fallback_category_html(Category::Physics, "   ", None);

        assert!(html.contains("<h1>物理学</h1>"));
        assert!(html.contains("物理学カテゴリの記事一覧です。"));
    }

    #[test]
    fn test_build_fallback_category_html_escapes_frontmatter_text() {
        let html = build_fallback_category_html(
            Category::Tech,
            "<script>alert(1)</script>",
            Some("\"quoted\" & <tag>"),
        );

        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("\"quoted\" &amp; &lt;tag&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn test_normalize_category_metadata_trims_and_falls_back() {
        assert_eq!(normalize_category_title(Category::Physics, "   "), "物理学");
        assert_eq!(normalize_category_title(Category::Tech, "  Tech  "), "Tech");
        assert_eq!(normalize_category_description(Some("   ")), None);
        assert_eq!(
            normalize_category_description(Some("  Technology landing  ")),
            Some("Technology landing".to_string())
        );
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
