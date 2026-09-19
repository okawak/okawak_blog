mod builder;
mod path;

pub use builder::{
    build_article_page_document, build_category_page_document, build_home_page_document,
    build_static_page_document, find_article_summary,
};
pub use path::{
    build_article_page_canonical_path, build_article_path, build_category_page_canonical_path,
    build_category_path, build_home_page_canonical_path, build_static_page_canonical_path,
};

use crate::{Category, PageKey, SectionPath, Slug, Title};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteArticleCard {
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub section_path: SectionPath,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteCategorySummary {
    pub category: Category,
    pub article_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomePageDocument {
    pub total_articles: usize,
    pub categories: Vec<SiteCategorySummary>,
    pub articles: Vec<SiteArticleCard>,
    pub fragment: Option<HomeFragmentDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeFragmentDocument {
    pub title: String,
    pub description: Option<String>,
    pub html: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticlePageDocument {
    pub article: SiteArticleCard,
    pub html: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryPageDocument {
    pub category: Category,
    pub title: String,
    pub description: Option<String>,
    pub html: String,
    pub sections: Vec<CategorySectionGroup>,
    pub articles: Vec<SiteArticleCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticPageDocument {
    pub page: PageKey,
    pub title: String,
    pub description: Option<String>,
    pub html: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategorySectionGroup {
    pub section_path: SectionPath,
    pub heading: String,
    pub articles: Vec<SiteArticleCard>,
}
