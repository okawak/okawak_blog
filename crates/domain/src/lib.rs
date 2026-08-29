#![warn(unreachable_pub)]

mod artifact;
mod entities;
mod error;
mod page;
mod publication;

pub use artifact::{
    ARTIFACT_RELEASE_SCHEMA_VERSION, ArticleIndexDocument, ArticleSummaryDocument,
    ArtifactReleasePointerDocument, CategoryArtifactDocument, CategoryMetadataDocument,
    HomeFragmentArtifactDocument, PageArtifactDocument, SiteMetadataDocument,
};
pub use entities::{Category, PageKey, SectionPath, Slug, Timestamp, Title};
pub use error::{DomainError, Result};
pub use page::{
    ArticlePageDocument, CategoryPageDocument, CategorySectionGroup, HomeFragmentDocument,
    HomePageDocument, SiteArticleCard, SiteCategorySummary, StaticPageDocument,
    build_article_page_canonical_path, build_article_page_description, build_article_page_document,
    build_article_page_title, build_article_path, build_category_page_canonical_path,
    build_category_page_description, build_category_page_document, build_category_page_title,
    build_category_path, build_home_page_canonical_path, build_home_page_description,
    build_home_page_document, build_home_page_title, build_static_page_canonical_path,
    build_static_page_description, build_static_page_document, build_static_page_title,
    find_article_summary,
};
pub use publication::{
    ArticleBody, ArticleMeta, CategoryIndex, CategoryLandingBody, CategoryLandingMeta,
    CategoryMetadata, PublishableArticle, PublishableCategoryLanding, PublishedArticleSummary,
    SiteMetadata, build_article_index, build_category_indexes, build_site_metadata,
};
