//! Domain models and pure functions for publishable site artifacts.

use crate::{Category, DomainError, Result, SectionPath, Slug, Timestamp, Title};
use std::cmp::Ordering;

/// Metadata for a publishable article.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleMeta {
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub section_path: SectionPath,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Rendered HTML body for a publishable article.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleBody(String);

impl ArticleBody {
    pub fn new(html: String) -> Result<Self> {
        if html.trim().is_empty() {
            return Err(DomainError::validation("html"));
        }

        Ok(Self(html))
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
    pub section_path: SectionPath,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Metadata for a rendered category landing page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLandingMeta {
    pub category: Category,
    pub title: Title,
    pub description: Option<String>,
    pub updated_at: Timestamp,
}

/// Category-specific index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryIndex {
    pub category: Category,
    /// Landing metadata included in the category index artifact when available.
    pub landing: Option<CategoryLandingMeta>,
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

/// Build per-category indexes, including categories represented only by a landing page.
pub fn build_category_indexes(
    article_metas: &[ArticleMeta],
    category_landings: Vec<CategoryLandingMeta>,
) -> Vec<CategoryIndex> {
    use std::collections::HashMap;

    let mut grouped: HashMap<Category, Vec<PublishedArticleSummary>> = HashMap::new();
    for article_meta in article_metas {
        grouped
            .entry(article_meta.category)
            .or_default()
            .push(build_article_summary_from_meta(article_meta));
    }

    let mut landings_by_category: HashMap<_, _> = category_landings
        .into_iter()
        .map(|landing| (landing.category, landing))
        .collect();
    for category in landings_by_category.keys() {
        grouped.entry(*category).or_default();
    }

    let mut indexes: Vec<_> = grouped
        .into_iter()
        .map(|(category, mut articles)| {
            articles.sort_by(compare_summaries);
            CategoryIndex {
                category,
                landing: landings_by_category.remove(&category),
                articles,
            }
        })
        .collect();
    indexes.sort_by(|a, b| a.category.as_str().cmp(b.category.as_str()));
    indexes
}

/// Build site metadata from completed category indexes.
pub fn build_site_metadata(category_indexes: &[CategoryIndex]) -> SiteMetadata {
    let categories = category_indexes
        .iter()
        .map(|index| CategoryMetadata {
            category: index.category,
            article_count: index.articles.len(),
        })
        .collect();

    SiteMetadata {
        total_articles: category_indexes
            .iter()
            .map(|index| index.articles.len())
            .sum(),
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

    fn build_category_landing(category: Category, title: &str) -> CategoryLandingMeta {
        CategoryLandingMeta {
            category,
            title: Title::new(title.to_string()).unwrap(),
            description: Some(format!("{title} landing")),
            updated_at: Timestamp::new("2025-01-03T00:00:00+09:00".to_string()).unwrap(),
        }
    }

    fn build_article(
        title: &str,
        slug: &str,
        category: Category,
        priority: Option<i32>,
        created_at: &str,
    ) -> PublishableArticle {
        let meta = ArticleMeta {
            slug: Slug::new(slug.to_string()).unwrap(),
            title: Title::new(title.to_string()).unwrap(),
            category,
            section_path: SectionPath::default(),
            description: Some(format!("{title} summary")),
            tags: vec!["tag".to_string()],
            priority,
            created_at: Timestamp::new(created_at.to_string()).unwrap(),
            updated_at: Timestamp::new(created_at.to_string()).unwrap(),
        };
        let body = ArticleBody::new(format!("<p>{title}</p>")).unwrap();
        PublishableArticle::new(meta, body)
    }

    #[test]
    fn test_article_body_validation() {
        assert!(ArticleBody::new("   ".to_string()).is_err());
        let body = ArticleBody::new("<p>body</p>".to_string()).unwrap();

        assert_eq!(body.as_str(), "<p>body</p>");
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
        let indexes = build_category_indexes(
            &metas,
            vec![
                build_category_landing(Category::Tech, "Technology"),
                build_category_landing(Category::Physics, "Physics"),
            ],
        );

        assert_eq!(indexes.len(), 3);
        assert_eq!(indexes[0].category, Category::Daily);
        assert_eq!(indexes[0].articles.len(), 1);
        assert_eq!(indexes[1].category, Category::Physics);
        assert_eq!(indexes[1].articles.len(), 0);
        assert_eq!(
            indexes[1].landing.as_ref().unwrap().title.as_str(),
            "Physics"
        );
        assert_eq!(indexes[2].category, Category::Tech);
        assert_eq!(indexes[2].articles.len(), 1);
        assert_eq!(
            indexes[2].landing.as_ref().unwrap().title.as_str(),
            "Technology"
        );
    }

    #[test]
    fn test_build_site_metadata_includes_landing_only_category() {
        let indexes = build_category_indexes(
            &[],
            vec![build_category_landing(Category::Physics, "Physics")],
        );
        let metadata = build_site_metadata(&indexes);

        assert_eq!(metadata.total_articles, 0);
        assert_eq!(metadata.categories.len(), 1);
        assert_eq!(metadata.categories[0].category, Category::Physics);
        assert_eq!(metadata.categories[0].article_count, 0);
    }
}
