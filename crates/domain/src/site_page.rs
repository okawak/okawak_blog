//! Shared page contracts built from persisted artifact documents.

use crate::{
    ArticleIndexDocument, ArticleSummaryDocument, Category, CategoryIndexDocument, DomainError,
    PageArtifactDocument, PageKey, Result, SiteMetadataDocument, Slug, Title,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteArticleCard {
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub category_display_name: String,
    pub section_path: Vec<String>,
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
            section_path: summary.section_path.clone(),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticPageDocument {
    pub page: PageKey,
    pub title: String,
    pub description: Option<String>,
    pub html: String,
}

pub fn build_home_page_title(site_name: &str) -> String {
    site_name.to_string()
}

pub fn build_home_page_description(document: &HomePageDocument) -> String {
    format!(
        "{}件の記事を{}カテゴリで公開しています。",
        document.total_articles,
        document.categories.len()
    )
}

pub fn build_home_page_canonical_path() -> &'static str {
    "/"
}

pub fn build_article_page_title(document: &ArticlePageDocument, site_name: &str) -> String {
    format!("{} | {}", document.article.title.as_str(), site_name)
}

pub fn build_article_page_description(document: &ArticlePageDocument) -> String {
    document
        .article
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!(
                "{}カテゴリの記事です。",
                document.article.category_display_name
            )
        })
}

pub fn build_article_page_canonical_path(document: &ArticlePageDocument) -> String {
    format!("/articles/{}", document.article.slug.as_str())
}

pub fn build_category_page_title(document: &CategoryPageDocument, site_name: &str) -> String {
    format!("{} | {}", document.category_display_name, site_name)
}

pub fn build_category_page_description(document: &CategoryPageDocument) -> String {
    format!(
        "{}カテゴリの記事一覧です。{}件の記事があります。",
        document.category_display_name,
        document.articles.len()
    )
}

pub fn build_category_page_canonical_path(document: &CategoryPageDocument) -> String {
    format!("/categories/{}", document.category.as_str())
}

pub fn build_static_page_title(document: &StaticPageDocument, site_name: &str) -> String {
    format!("{} | {}", document.title, site_name)
}

pub fn build_static_page_description(document: &StaticPageDocument) -> String {
    document
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} ページです。", document.title))
}

pub fn build_static_page_canonical_path(document: &StaticPageDocument) -> String {
    format!("/{}", document.page.as_str())
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

pub fn build_static_page_document(artifact: &PageArtifactDocument) -> Result<StaticPageDocument> {
    let title = artifact.title.trim();
    let html = artifact.html.trim();

    if title.is_empty() {
        return Err(DomainError::validation("title"));
    }

    if html.is_empty() {
        return Err(DomainError::validation("html"));
    }

    Ok(StaticPageDocument {
        page: artifact.page.clone(),
        title: title.to_string(),
        description: artifact.description.clone(),
        html: artifact.html.clone(),
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
            section_path: vec!["block".to_string()],
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
    fn test_build_static_page_document() {
        let document = build_static_page_document(&PageArtifactDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: Some("About this site".to_string()),
            html: "<article><h1>About</h1></article>".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
        })
        .unwrap();

        assert_eq!(document.page.as_str(), "about");
        assert_eq!(document.title, "About");
        assert!(document.html.contains("<h1>About</h1>"));
    }

    #[test]
    fn test_build_static_page_document_rejects_blank_html() {
        let result = build_static_page_document(&PageArtifactDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: None,
            html: "   ".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
        });

        assert_eq!(result, Err(DomainError::validation("html")));
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

    #[test]
    fn test_build_home_page_metadata() {
        let document = HomePageDocument {
            total_articles: 3,
            categories: vec![
                SiteCategorySummary {
                    category: Category::Tech,
                    category_display_name: "技術".to_string(),
                    article_count: 2,
                },
                SiteCategorySummary {
                    category: Category::Daily,
                    category_display_name: "日常".to_string(),
                    article_count: 1,
                },
            ],
            articles: vec![SiteArticleCard::try_from(&sample_summary()).unwrap()],
        };

        assert_eq!(
            build_home_page_title("ぶくせんの探窟メモ"),
            "ぶくせんの探窟メモ"
        );
        assert_eq!(
            build_home_page_description(&document),
            "3件の記事を2カテゴリで公開しています。"
        );
        assert_eq!(build_home_page_canonical_path(), "/");
    }

    #[test]
    fn test_build_article_page_metadata() {
        let document =
            build_article_page_document(&sample_summary(), "<article><h1>Intro</h1></article>")
                .unwrap();

        assert_eq!(
            build_article_page_title(&document, "ぶくせんの探窟メモ"),
            "Intro | ぶくせんの探窟メモ"
        );
        assert_eq!(build_article_page_description(&document), "summary");
        assert_eq!(
            build_article_page_canonical_path(&document),
            "/articles/intro00000001"
        );
    }

    #[test]
    fn test_build_article_page_description_falls_back_when_missing() {
        let document = build_article_page_document(
            &ArticleSummaryDocument {
                description: None,
                ..sample_summary()
            },
            "<article><h1>Intro</h1></article>",
        )
        .unwrap();

        assert_eq!(
            build_article_page_description(&document),
            "技術カテゴリの記事です。"
        );
    }

    #[test]
    fn test_build_article_page_description_falls_back_when_blank() {
        let document = build_article_page_document(
            &ArticleSummaryDocument {
                description: Some("   ".to_string()),
                ..sample_summary()
            },
            "<article><h1>Intro</h1></article>",
        )
        .unwrap();

        assert_eq!(
            build_article_page_description(&document),
            "技術カテゴリの記事です。"
        );
    }

    #[test]
    fn test_build_category_page_metadata() {
        let document = build_category_page_document(&CategoryIndexDocument {
            category: "tech".to_string(),
            articles: vec![sample_summary()],
        })
        .unwrap();

        assert_eq!(
            build_category_page_title(&document, "ぶくせんの探窟メモ"),
            "技術 | ぶくせんの探窟メモ"
        );
        assert_eq!(
            build_category_page_description(&document),
            "技術カテゴリの記事一覧です。1件の記事があります。"
        );
        assert_eq!(
            build_category_page_canonical_path(&document),
            "/categories/tech"
        );
    }

    #[test]
    fn test_build_static_page_metadata() {
        let document = StaticPageDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: Some("About this site".to_string()),
            html: "<article><h1>About</h1></article>".to_string(),
        };

        assert_eq!(
            build_static_page_title(&document, "ぶくせんの探窟メモ"),
            "About | ぶくせんの探窟メモ"
        );
        assert_eq!(build_static_page_description(&document), "About this site");
        assert_eq!(build_static_page_canonical_path(&document), "/about");
    }
}
