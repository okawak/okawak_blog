//! Shared page contracts built from persisted artifact documents.

use crate::{
    ArticleIndexDocument, ArticleSummaryDocument, Category, CategoryIndexDocument, DomainError,
    Result, SiteMetadataDocument, Slug, Title,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteArticleCard {
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub category_display_name: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl TryFrom<&ArticleSummaryDocument> for SiteArticleCard {
    type Error = DomainError;

    fn try_from(summary: &ArticleSummaryDocument) -> Result<Self> {
        let category = Category::from_str(&summary.category)?;

        Ok(Self {
            slug: Slug::new(summary.slug.clone())?,
            title: Title::new(summary.title.clone())?,
            category,
            category_display_name: category.display_name().to_string(),
            description: summary.description.clone(),
            tags: summary.tags.clone(),
            priority: summary.priority,
            created_at: summary.created_at.clone(),
            updated_at: summary.updated_at.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteCategorySummary {
    pub category: Category,
    pub category_display_name: String,
    pub article_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomePageDocument {
    pub total_articles: usize,
    pub categories: Vec<SiteCategorySummary>,
    pub articles: Vec<SiteArticleCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArticlePageDocument {
    pub article: SiteArticleCard,
    pub html: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryPageDocument {
    pub category: Category,
    pub category_display_name: String,
    pub articles: Vec<SiteArticleCard>,
}

pub fn build_home_page_document(
    article_index: &ArticleIndexDocument,
    site_metadata: &SiteMetadataDocument,
) -> Result<HomePageDocument> {
    let articles = article_index
        .articles
        .iter()
        .map(SiteArticleCard::try_from)
        .collect::<Result<Vec<_>>>()?;
    let categories = site_metadata
        .categories
        .iter()
        .map(|category| {
            let parsed = Category::from_str(&category.category)?;
            Ok(SiteCategorySummary {
                category: parsed,
                category_display_name: parsed.display_name().to_string(),
                article_count: category.article_count,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(HomePageDocument {
        total_articles: site_metadata.total_articles,
        categories,
        articles,
    })
}

pub fn build_article_page_document(
    summary: &ArticleSummaryDocument,
    html: &str,
) -> Result<ArticlePageDocument> {
    if html.trim().is_empty() {
        return Err(DomainError::validation("html"));
    }

    Ok(ArticlePageDocument {
        article: SiteArticleCard::try_from(summary)?,
        html: html.to_string(),
    })
}

pub fn build_category_page_document(index: &CategoryIndexDocument) -> Result<CategoryPageDocument> {
    let category = Category::from_str(&index.category)?;
    let articles = index
        .articles
        .iter()
        .map(SiteArticleCard::try_from)
        .collect::<Result<Vec<_>>>()?;

    Ok(CategoryPageDocument {
        category,
        category_display_name: category.display_name().to_string(),
        articles,
    })
}

pub fn find_article_summary<'a>(
    article_index: &'a ArticleIndexDocument,
    slug: &Slug,
) -> Option<&'a ArticleSummaryDocument> {
    article_index
        .articles
        .iter()
        .find(|article| article.slug == slug.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CategoryMetadataDocument;

    fn sample_summary() -> ArticleSummaryDocument {
        ArticleSummaryDocument {
            slug: "intro00000001".to_string(),
            title: "Intro".to_string(),
            category: "tech".to_string(),
            description: Some("summary".to_string()),
            tags: vec!["rust".to_string()],
            priority: Some(10),
            created_at: "2025-01-01T00:00:00+09:00".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
        }
    }

    #[test]
    fn test_build_site_article_card() {
        let card = SiteArticleCard::try_from(&sample_summary()).unwrap();

        assert_eq!(card.slug.as_str(), "intro00000001");
        assert_eq!(card.title.as_str(), "Intro");
        assert_eq!(card.category, Category::Tech);
        assert_eq!(card.category_display_name, "技術");
    }

    #[test]
    fn test_build_home_page_document() {
        let document = build_home_page_document(
            &ArticleIndexDocument {
                articles: vec![sample_summary()],
            },
            &SiteMetadataDocument {
                total_articles: 1,
                categories: vec![CategoryMetadataDocument {
                    category: "tech".to_string(),
                    article_count: 1,
                }],
            },
        )
        .unwrap();

        assert_eq!(document.total_articles, 1);
        assert_eq!(document.categories.len(), 1);
        assert_eq!(document.categories[0].category_display_name, "技術");
        assert_eq!(document.articles[0].title.as_str(), "Intro");
    }

    #[test]
    fn test_build_article_page_document() {
        let document =
            build_article_page_document(&sample_summary(), "<article><h1>Intro</h1></article>")
                .unwrap();

        assert_eq!(document.article.slug.as_str(), "intro00000001");
        assert!(document.html.contains("<h1>Intro</h1>"));
    }

    #[test]
    fn test_build_article_page_document_rejects_blank_html() {
        let result = build_article_page_document(&sample_summary(), "   ");

        assert_eq!(result, Err(DomainError::validation("html")));
    }

    #[test]
    fn test_build_category_page_document() {
        let document = build_category_page_document(&CategoryIndexDocument {
            category: "daily".to_string(),
            articles: vec![ArticleSummaryDocument {
                category: "daily".to_string(),
                ..sample_summary()
            }],
        })
        .unwrap();

        assert_eq!(document.category, Category::Daily);
        assert_eq!(document.category_display_name, "日常");
        assert_eq!(document.articles.len(), 1);
    }

    #[test]
    fn test_find_article_summary() {
        let index = ArticleIndexDocument {
            articles: vec![sample_summary()],
        };
        let slug = Slug::new("intro00000001".to_string()).unwrap();

        let article = find_article_summary(&index, &slug).unwrap();

        assert_eq!(article.title, "Intro");
    }
}
