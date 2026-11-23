use crate::{
    ArticleBody, ArticleMeta, Category, CategoryIndex, CategoryMetadata, PageKey,
    PublishableArticle, SectionPath, SiteMetadata, Slug, Timestamp, Title,
};
use serde::{Deserialize, Serialize};

/// A single article can be read without loading the site-wide index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArticleArtifactDocument {
    pub summary: ArticleSummaryDocument,
    pub html: String,
}

impl ArticleArtifactDocument {
    pub fn into_article(
        self,
        category: &Category,
        slug: &Slug,
    ) -> crate::Result<PublishableArticle> {
        if self.summary.category != category.as_str() || self.summary.slug != slug.as_str() {
            return Err(crate::DomainError::validation("article identity mismatch"));
        }
        Ok(PublishableArticle::new(
            self.summary.try_into()?,
            ArticleBody::new(self.html)?,
        ))
    }
}

impl From<&crate::ArticleMeta> for ArticleSummaryDocument {
    fn from(meta: &crate::ArticleMeta) -> Self {
        Self {
            slug: meta.slug.to_string(),
            title: meta.title.to_string(),
            category: meta.category.to_string(),
            section_path: meta.section_path.clone(),
            description: meta.description.clone(),
            tags: meta.tags.iter().map(ToString::to_string).collect(),
            priority: meta.priority,
            created_at: meta.created_at.to_string(),
            updated_at: meta.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArticleSummaryDocument {
    pub slug: String,
    pub title: String,
    pub category: String,
    #[serde(default)]
    pub section_path: SectionPath,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl TryFrom<ArticleSummaryDocument> for ArticleMeta {
    type Error = crate::DomainError;

    fn try_from(document: ArticleSummaryDocument) -> crate::Result<Self> {
        Ok(Self {
            slug: Slug::new(document.slug)?,
            title: Title::new(document.title)?.trimmed(),
            category: document.category.parse::<Category>()?,
            section_path: document.section_path,
            description: document.description,
            tags: document
                .tags
                .into_iter()
                .map(crate::TagId::new)
                .collect::<crate::Result<_>>()?,
            priority: document.priority,
            created_at: Timestamp::new(document.created_at)?.trimmed(),
            updated_at: Timestamp::new(document.updated_at)?.trimmed(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArticleIndexDocument {
    pub articles: Vec<ArticleSummaryDocument>,
}

impl From<&[ArticleMeta]> for ArticleIndexDocument {
    fn from(articles: &[ArticleMeta]) -> Self {
        Self {
            articles: articles.iter().map(ArticleSummaryDocument::from).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryArtifactDocument {
    pub category: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub html: String,
    pub updated_at: String,
    pub articles: Vec<ArticleSummaryDocument>,
}

impl TryFrom<CategoryIndex> for CategoryArtifactDocument {
    type Error = crate::DomainError;

    fn try_from(index: CategoryIndex) -> crate::Result<Self> {
        let landing = index
            .landing
            .ok_or_else(|| crate::DomainError::validation("category_landing"))?;

        Ok(Self {
            category: index.category.as_str().to_string(),
            title: landing.meta.title.to_string(),
            description: landing.meta.description,
            html: landing.body.into_string(),
            updated_at: landing.meta.updated_at.to_string(),
            articles: index
                .articles
                .iter()
                .map(ArticleSummaryDocument::from)
                .collect(),
        })
    }
}

impl CategoryArtifactDocument {
    pub fn validate_landing(&self) -> crate::Result<()> {
        self.category.parse::<Category>()?;
        Title::new(self.title.clone())?;
        validate_html(&self.html)?;
        Timestamp::new(self.updated_at.clone())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryMetadataDocument {
    pub category: String,
    pub article_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteMetadataDocument {
    pub total_articles: usize,
    pub categories: Vec<CategoryMetadataDocument>,
}

impl TryFrom<&SiteMetadataDocument> for SiteMetadata {
    type Error = crate::DomainError;

    fn try_from(document: &SiteMetadataDocument) -> crate::Result<Self> {
        let categories = document
            .categories
            .iter()
            .map(|category| {
                Ok(CategoryMetadata {
                    category: category.category.parse::<Category>()?,
                    article_count: category.article_count,
                })
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(Self {
            total_articles: document.total_articles,
            categories,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageArtifactDocument {
    pub page: PageKey,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub html: String,
    pub updated_at: String,
}

impl PageArtifactDocument {
    pub fn validate(&self) -> crate::Result<()> {
        Title::new(self.title.clone())?;
        validate_html(&self.html)?;
        Timestamp::new(self.updated_at.clone())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HomeFragmentArtifactDocument {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub html: String,
    pub updated_at: String,
}

impl HomeFragmentArtifactDocument {
    pub fn validate(&self) -> crate::Result<()> {
        Title::new(self.title.clone())?;
        validate_html(&self.html)?;
        Timestamp::new(self.updated_at.clone())?;
        Ok(())
    }
}

impl From<&SiteMetadata> for SiteMetadataDocument {
    fn from(metadata: &SiteMetadata) -> Self {
        Self {
            total_articles: metadata.total_articles,
            categories: metadata
                .categories
                .iter()
                .map(|category| CategoryMetadataDocument {
                    category: category.category.as_str().to_string(),
                    article_count: category.article_count,
                })
                .collect(),
        }
    }
}

fn validate_html(html: &str) -> crate::Result<()> {
    if html.trim().is_empty() {
        return Err(crate::DomainError::validation("html"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Category, CategoryLandingBody, CategoryLandingMeta, PageKey, PublishableCategoryLanding,
        Slug, Timestamp, Title,
    };

    #[test]
    fn test_article_summary_document_conversion() {
        let summary = ArticleMeta {
            slug: Slug::new("abc123def456".to_string()).unwrap(),
            title: Title::new("Test Output".to_string()).unwrap(),
            category: Category::Tech,
            section_path: SectionPath::new(vec!["block".to_string()]).unwrap(),
            description: Some("Test description".to_string()),
            tags: vec!["test".parse().unwrap()],
            priority: Some(1),
            created_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
            updated_at: Timestamp::new("2025-01-02T00:00:00+09:00".to_string()).unwrap(),
        };

        let document = ArticleSummaryDocument::from(&summary);
        let json = serde_json::to_string(&document).unwrap();

        assert!(json.contains("\"title\":\"Test Output\""));
        assert!(json.contains("\"slug\":\"abc123def456\""));
        assert!(json.contains("\"category\":\"tech\""));
        assert!(json.contains("\"section_path\":[\"block\"]"));
        assert_eq!(ArticleMeta::try_from(document).unwrap(), summary);
    }

    #[test]
    fn article_artifact_converts_to_normalized_metadata_without_changing_html() {
        let document = ArticleArtifactDocument {
            summary: ArticleSummaryDocument {
                slug: "intro".into(),
                title: "  Intro  ".into(),
                category: "tech".into(),
                section_path: SectionPath::default(),
                description: None,
                tags: vec!["rust".into()],
                priority: None,
                created_at: " 2025-01-01T00:00:00Z ".into(),
                updated_at: " 2025-01-02T00:00:00Z ".into(),
            },
            html: " <p>Intro</p> ".into(),
        };
        for (category, slug) in [(Category::Daily, "intro"), (Category::Tech, "other")] {
            assert!(
                document
                    .clone()
                    .into_article(&category, &slug.parse().unwrap())
                    .is_err()
            );
        }
        let article = document
            .into_article(&Category::Tech, &"intro".parse().unwrap())
            .unwrap();
        assert_eq!(article.meta.title.as_str(), "Intro");
        assert_eq!(article.meta.created_at.as_str(), "2025-01-01T00:00:00Z");
        assert_eq!(article.meta.updated_at.as_str(), "2025-01-02T00:00:00Z");
        assert_eq!(article.meta.tags[0].as_str(), "rust");
        assert_eq!(article.body.as_str(), " <p>Intro</p> ");
    }

    #[test]
    fn test_article_summary_document_keeps_empty_tags_field() {
        let summary = ArticleMeta {
            slug: Slug::new("emptytags001".to_string()).unwrap(),
            title: Title::new("Empty Tags".to_string()).unwrap(),
            category: Category::Daily,
            section_path: SectionPath::default(),
            description: None,
            tags: vec![],
            priority: None,
            created_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
            updated_at: Timestamp::new("2025-01-02T00:00:00+09:00".to_string()).unwrap(),
        };

        let json = serde_json::to_string(&ArticleSummaryDocument::from(&summary)).unwrap();

        assert!(json.contains("\"tags\":[]"));
    }

    #[test]
    fn test_article_summary_document_deserialization_defaults_missing_section_path() {
        let json = r#"{
            "slug":"legacy0000001",
            "title":"Legacy",
            "category":"tech",
            "description":"legacy",
            "tags":[],
            "priority":1,
            "created_at":"2025-01-01T00:00:00+09:00",
            "updated_at":"2025-01-01T00:00:00+09:00"
        }"#;

        let document: ArticleSummaryDocument = serde_json::from_str(json).unwrap();

        assert_eq!(document.section_path, SectionPath::default());
    }

    #[test]
    fn test_article_summary_document_rejects_invalid_value_objects() {
        let document = ArticleSummaryDocument {
            slug: "invalid/slug".to_string(),
            title: "Article".to_string(),
            category: "tech".to_string(),
            section_path: SectionPath::default(),
            description: None,
            tags: vec![],
            priority: None,
            created_at: "2025-01-01T00:00:00+09:00".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
        };

        assert!(ArticleMeta::try_from(document).is_err());
    }

    #[test]
    fn test_page_artifact_document_serialization() {
        let mut document = PageArtifactDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: Some("About this site".to_string()),
            html: "<h1>About</h1>".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
        };

        let json = serde_json::to_string(&document).unwrap();

        assert!(json.contains("\"page\":\"about\""));
        assert!(json.contains("\"title\":\"About\""));
        assert!(json.contains("\"html\":\"<h1>About</h1>\""));
        assert!(document.validate().is_ok());

        document.html.clear();
        assert!(document.validate().is_err());
    }

    #[test]
    fn test_home_fragment_artifact_document_serialization() {
        let mut document = HomeFragmentArtifactDocument {
            title: "Home".to_string(),
            description: Some("Home introduction".to_string()),
            html: "<p>Welcome</p>".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
        };

        let json = serde_json::to_string(&document).unwrap();

        assert!(!json.contains("\"page\""));
        assert!(json.contains("\"title\":\"Home\""));
        assert!(json.contains("\"html\":\"<p>Welcome</p>\""));
        assert!(document.validate().is_ok());

        document.title.clear();
        assert!(document.validate().is_err());
    }

    #[test]
    fn test_site_metadata_document_validates_categories() {
        let document = SiteMetadataDocument {
            total_articles: 0,
            categories: vec![CategoryMetadataDocument {
                category: "unknown".to_string(),
                article_count: 0,
            }],
        };

        assert!(SiteMetadata::try_from(&document).is_err());
    }

    #[test]
    fn test_category_artifact_document_combines_landing_and_index() {
        let landing = CategoryLandingMeta {
            category: Category::Tech,
            title: Title::new("Technology".to_string()).unwrap(),
            description: Some("Technology landing".to_string()),
            updated_at: Timestamp::new("2025-01-02T00:00:00+09:00".to_string()).unwrap(),
        };
        let index = CategoryIndex {
            category: Category::Tech,
            landing: Some(PublishableCategoryLanding::new(
                landing,
                CategoryLandingBody::new("<p>Technology</p>".into()).unwrap(),
            )),
            articles: vec![],
        };

        let document = CategoryArtifactDocument::try_from(index).unwrap();

        assert_eq!(document.title, "Technology");
        assert_eq!(document.description.as_deref(), Some("Technology landing"));
        assert_eq!(document.html, "<p>Technology</p>");
        assert_eq!(document.updated_at, "2025-01-02T00:00:00+09:00");
        assert!(document.validate_landing().is_ok());
    }

    #[test]
    fn test_category_artifact_document_requires_landing() {
        let index = CategoryIndex {
            category: Category::Tech,
            landing: None,
            articles: vec![],
        };

        assert!(CategoryArtifactDocument::try_from(index).is_err());
    }

    #[test]
    fn test_category_artifact_document_rejects_blank_html() {
        let document = CategoryArtifactDocument {
            category: "tech".into(),
            title: "Technology".into(),
            description: None,
            html: "  ".into(),
            updated_at: "2025-01-02T00:00:00+09:00".into(),
            articles: vec![],
        };
        assert!(document.validate_landing().is_err());
    }
}
