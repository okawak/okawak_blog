//! Domain models and pure functions for publishable site artifacts.

use crate::{Category, DomainError, Result, Slug, Title};
use std::cmp::Ordering;

/// Metadata for a publishable article.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleMeta {
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub section_path: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleMetaInput {
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub section_path: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl ArticleMeta {
    pub fn new(input: ArticleMetaInput) -> Result<Self> {
        if input.created_at.trim().is_empty() {
            return Err(DomainError::validation("created_at"));
        }
        if input.updated_at.trim().is_empty() {
            return Err(DomainError::validation("updated_at"));
        }

        Ok(Self {
            slug: input.slug,
            title: input.title,
            category: input.category,
            section_path: input.section_path,
            description: input.description,
            tags: input.tags,
            priority: input.priority,
            created_at: input.created_at,
            updated_at: input.updated_at,
        })
    }
}

/// Rendered HTML body for a publishable article.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleBody {
    pub html: String,
}

impl ArticleBody {
    pub fn new(html: String) -> Result<Self> {
        if html.trim().is_empty() {
            return Err(DomainError::validation("html"));
        }

        Ok(Self { html })
    }
}

/// Fully publishable article used by the artifact pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishableArticle {
    pub meta: ArticleMeta,
    pub body: ArticleBody,
}

impl PublishableArticle {
    pub fn new(meta: ArticleMeta, body: ArticleBody) -> Self {
        Self { meta, body }
    }
}

/// Lightweight summary entry stored in article and category indexes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedArticleSummary {
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub section_path: Vec<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Category-specific index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryIndex {
    pub category: Category,
    pub articles: Vec<PublishedArticleSummary>,
}

/// Per-category metadata for the whole site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryMetadata {
    pub category: Category,
    pub article_count: usize,
}

/// Site-wide metadata for generated artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteMetadata {
    pub total_articles: usize,
    pub categories: Vec<CategoryMetadata>,
}

/// Build a summary entry from a publishable article.
pub fn build_article_summary(article: &PublishableArticle) -> PublishedArticleSummary {
    build_article_summary_from_meta(&article.meta)
}

/// Build a summary entry from article metadata.
pub fn build_article_summary_from_meta(meta: &ArticleMeta) -> PublishedArticleSummary {
    PublishedArticleSummary {
        slug: meta.slug.clone(),
        title: meta.title.clone(),
        category: meta.category,
        section_path: meta.section_path.clone(),
        description: meta.description.clone(),
        tags: meta.tags.clone(),
        priority: meta.priority,
        created_at: meta.created_at.clone(),
        updated_at: meta.updated_at.clone(),
    }
}

/// Build the site-wide article index.
pub fn build_article_index(article_metas: &[ArticleMeta]) -> Vec<PublishedArticleSummary> {
    let mut summaries: Vec<_> = article_metas
        .iter()
        .map(build_article_summary_from_meta)
        .collect();
    summaries.sort_by(compare_summaries);
    summaries
}

/// Build per-category indexes.
pub fn build_category_indexes(article_metas: &[ArticleMeta]) -> Vec<CategoryIndex> {
    use std::collections::HashMap;

    let mut grouped: HashMap<Category, Vec<PublishedArticleSummary>> = HashMap::new();
    for article_meta in article_metas {
        grouped
            .entry(article_meta.category)
            .or_default()
            .push(build_article_summary_from_meta(article_meta));
    }

    let mut indexes: Vec<_> = grouped
        .into_iter()
        .map(|(category, mut articles)| {
            articles.sort_by(compare_summaries);
            CategoryIndex { category, articles }
        })
        .collect();
    indexes.sort_by(|a, b| a.category.as_str().cmp(b.category.as_str()));
    indexes
}

/// Build metadata for the generated site.
pub fn build_site_metadata(article_metas: &[ArticleMeta]) -> SiteMetadata {
    let category_indexes = build_category_indexes(article_metas);
    let categories = category_indexes
        .into_iter()
        .map(|index| CategoryMetadata {
            category: index.category,
            article_count: index.articles.len(),
        })
        .collect();

    SiteMetadata {
        total_articles: article_metas.len(),
        categories,
    }
}

fn compare_summaries(a: &PublishedArticleSummary, b: &PublishedArticleSummary) -> Ordering {
    b.priority
        .unwrap_or(i32::MIN)
        .cmp(&a.priority.unwrap_or(i32::MIN))
        .then_with(|| b.updated_at.cmp(&a.updated_at))
        .then_with(|| b.created_at.cmp(&a.created_at))
        .then_with(|| a.slug.as_str().cmp(b.slug.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_article(
        title: &str,
        slug: &str,
        category: Category,
        priority: Option<i32>,
        created_at: &str,
    ) -> PublishableArticle {
        let meta = ArticleMeta::new(ArticleMetaInput {
            slug: Slug::new(slug.to_string()).unwrap(),
            title: Title::new(title.to_string()).unwrap(),
            category,
            section_path: vec![],
            description: Some(format!("{title} summary")),
            tags: vec!["tag".to_string()],
            priority,
            created_at: created_at.to_string(),
            updated_at: created_at.to_string(),
        })
        .unwrap();
        let body = ArticleBody::new(format!("<p>{title}</p>")).unwrap();
        PublishableArticle::new(meta, body)
    }

    #[test]
    fn test_article_body_validation() {
        assert!(ArticleBody::new("   ".to_string()).is_err());
        assert!(ArticleBody::new("<p>body</p>".to_string()).is_ok());
    }

    #[test]
    fn test_article_meta_validation() {
        let valid_input = ArticleMetaInput {
            slug: Slug::new("valid00000001".to_string()).unwrap(),
            title: Title::new("Valid".to_string()).unwrap(),
            category: Category::Tech,
            section_path: vec!["block".to_string()],
            description: Some("summary".to_string()),
            tags: vec!["tag".to_string()],
            priority: Some(1),
            created_at: "2025-01-01T00:00:00+09:00".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
        };

        assert!(ArticleMeta::new(valid_input.clone()).is_ok());

        let missing_created_at = ArticleMetaInput {
            created_at: "   ".to_string(),
            ..valid_input.clone()
        };
        assert!(ArticleMeta::new(missing_created_at).is_err());

        let missing_updated_at = ArticleMetaInput {
            updated_at: "".to_string(),
            ..valid_input
        };
        assert!(ArticleMeta::new(missing_updated_at).is_err());
    }

    #[test]
    fn test_build_article_index_orders_by_priority_desc() {
        let articles = vec![
            build_article(
                "Low",
                "low000000001",
                Category::Tech,
                Some(1),
                "2025-01-01T00:00:00+09:00",
            ),
            build_article(
                "High",
                "high00000002",
                Category::Tech,
                Some(10),
                "2025-01-02T00:00:00+09:00",
            ),
        ];

        let metas: Vec<_> = articles.into_iter().map(|article| article.meta).collect();
        let index = build_article_index(&metas);
        assert_eq!(index[0].title.as_str(), "High");
        assert_eq!(index[1].title.as_str(), "Low");
    }

    #[test]
    fn test_build_category_indexes_groups_articles() {
        let articles = vec![
            build_article(
                "Tech",
                "tech00000001",
                Category::Tech,
                Some(1),
                "2025-01-01T00:00:00+09:00",
            ),
            build_article(
                "Daily",
                "daily0000001",
                Category::Daily,
                Some(1),
                "2025-01-02T00:00:00+09:00",
            ),
        ];

        let metas: Vec<_> = articles.into_iter().map(|article| article.meta).collect();
        let indexes = build_category_indexes(&metas);
        assert_eq!(indexes.len(), 2);
        assert_eq!(indexes[0].articles.len(), 1);
        assert_eq!(indexes[1].articles.len(), 1);
    }
}
