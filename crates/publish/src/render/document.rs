use super::{body, bookmark::BookmarkEnricher};
use crate::{
    classify::{ParsedArticleFile, ParsedCategoryFile, ParsedHomeFile, ParsedPageFile},
    error::Result,
    links,
};
use domain::{
    ArticleBody, ArticleMeta, CategoryLandingBody, CategoryLandingMeta,
    HomeFragmentArtifactDocument, PageArtifactDocument, PublishableArticle,
    PublishableCategoryLanding, Timestamp, Title,
};

pub(crate) async fn render_article(
    parsed_file: ParsedArticleFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
) -> Result<PublishableArticle> {
    let html = body::render(&parsed_file.markdown_body, link_index, &enrich).await;
    let meta = ArticleMeta {
        slug: parsed_file.slug,
        title: Title::new(parsed_file.front_matter.title)?,
        category: parsed_file.category,
        section_path: parsed_file.section_path,
        description: parsed_file.front_matter.summary,
        tags: parsed_file.front_matter.tags.unwrap_or_default(),
        priority: parsed_file.front_matter.priority,
        created_at: Timestamp::new(parsed_file.front_matter.created)?,
        updated_at: Timestamp::new(parsed_file.front_matter.updated)?,
    };
    let body = ArticleBody::new(html)?;
    Ok(PublishableArticle::new(meta, body))
}

pub(crate) async fn render_category(
    parsed_file: ParsedCategoryFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
) -> Result<PublishableCategoryLanding> {
    let html = body::render(&parsed_file.markdown_body, link_index, &enrich).await;
    let meta = CategoryLandingMeta {
        category: parsed_file.category,
        title: Title::new(parsed_file.front_matter.title)?,
        description: parsed_file.front_matter.summary,
        updated_at: Timestamp::new(parsed_file.front_matter.updated)?,
    };
    let body = CategoryLandingBody::new(html)?;
    Ok(PublishableCategoryLanding::new(meta, body))
}

pub(crate) async fn render_home(
    parsed_file: ParsedHomeFile,
    link_index: &links::Index,
    enrich: BookmarkEnricher,
) -> HomeFragmentArtifactDocument {
    let html = body::render(&parsed_file.markdown_body, link_index, &enrich).await;
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
    let html = body::render(&parsed_file.markdown_body, link_index, &enrich).await;
    PageArtifactDocument {
        page: parsed_file.page,
        title: parsed_file.front_matter.title,
        description: parsed_file.front_matter.summary,
        html,
        updated_at: parsed_file.front_matter.updated,
    }
}
