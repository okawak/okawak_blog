use domain::{CategoryIndex, PublishedArticleSummary, SiteMetadata};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq)]
pub struct ArticleSummaryJson {
    pub slug: String,
    pub title: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&PublishedArticleSummary> for ArticleSummaryJson {
    fn from(summary: &PublishedArticleSummary) -> Self {
        Self {
            slug: summary.slug.as_str().to_string(),
            title: summary.title.as_str().to_string(),
            category: summary.category.as_str().to_string(),
            description: summary.description.clone(),
            tags: summary.tags.clone(),
            priority: summary.priority,
            created_at: summary.created_at.clone(),
            updated_at: summary.updated_at.clone(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ArticleIndexJson {
    pub articles: Vec<ArticleSummaryJson>,
}

impl From<&[PublishedArticleSummary]> for ArticleIndexJson {
    fn from(articles: &[PublishedArticleSummary]) -> Self {
        Self {
            articles: articles.iter().map(ArticleSummaryJson::from).collect(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryIndexJson {
    pub category: String,
    pub articles: Vec<ArticleSummaryJson>,
}

impl From<&CategoryIndex> for CategoryIndexJson {
    fn from(index: &CategoryIndex) -> Self {
        Self {
            category: index.category.as_str().to_string(),
            articles: index
                .articles
                .iter()
                .map(ArticleSummaryJson::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CategoryMetadataJson {
    pub category: String,
    pub article_count: usize,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SiteMetadataJson {
    pub total_articles: usize,
    pub categories: Vec<CategoryMetadataJson>,
}

impl From<&SiteMetadata> for SiteMetadataJson {
    fn from(metadata: &SiteMetadata) -> Self {
        Self {
            total_articles: metadata.total_articles,
            categories: metadata
                .categories
                .iter()
                .map(|category| CategoryMetadataJson {
                    category: category.category.as_str().to_string(),
                    article_count: category.article_count,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Category, Slug, Title};

    #[test]
    fn test_article_summary_json_conversion() {
        let summary = PublishedArticleSummary {
            slug: Slug::new("abc123def456".to_string()).unwrap(),
            title: Title::new("Test Output".to_string()).unwrap(),
            category: Category::Tech,
            description: Some("Test description".to_string()),
            tags: vec!["test".to_string()],
            priority: Some(1),
            created_at: "2025-01-01T00:00:00+09:00".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
        };

        let json = serde_json::to_string(&ArticleSummaryJson::from(&summary)).unwrap();

        assert!(json.contains("\"title\":\"Test Output\""));
        assert!(json.contains("\"slug\":\"abc123def456\""));
        assert!(json.contains("\"category\":\"tech\""));
    }
}
