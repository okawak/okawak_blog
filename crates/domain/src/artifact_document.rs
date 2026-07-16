//! Shared artifact contract persisted by publisher and read by site/server.

use crate::{CategoryIndex, PageKey, PublishedArticleSummary, SiteMetadata};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

pub const ARTIFACT_RELEASE_SCHEMA_VERSION: u32 = 1;

/// Points readers at one immutable set of published artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactReleasePointerDocument {
    pub schema_version: u32,
    pub release_id: String,
    pub artifact_prefix: String,
    pub publisher_commit: String,
    pub source_commit: String,
    pub generated_at: String,
}

impl ArtifactReleasePointerDocument {
    pub fn generated_at_time(&self) -> crate::Result<SystemTime> {
        parse_rfc3339_utc(&self.generated_at)
            .map(|timestamp| timestamp.to_utc().into())
            .ok_or_else(|| crate::DomainError::validation("generated_at"))
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.schema_version != ARTIFACT_RELEASE_SCHEMA_VERSION {
            return Err(crate::DomainError::validation("schema_version"));
        }
        if !is_safe_path_segment(&self.release_id) {
            return Err(crate::DomainError::validation("release_id"));
        }
        if !is_safe_relative_prefix(&self.artifact_prefix) {
            return Err(crate::DomainError::validation("artifact_prefix"));
        }
        for (field, value) in [
            ("publisher_commit", self.publisher_commit.as_str()),
            ("source_commit", self.source_commit.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(crate::DomainError::validation(field));
            }
        }
        self.generated_at_time()?;
        Ok(())
    }
}

fn parse_rfc3339_utc(value: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .filter(|timestamp| timestamp.offset().local_minus_utc() == 0)
}

fn is_safe_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
}

fn is_safe_relative_prefix(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains('\\')
        && value.split('/').all(is_safe_path_segment)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArticleSummaryDocument {
    pub slug: String,
    pub title: String,
    pub category: String,
    #[serde(default)]
    pub section_path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&PublishedArticleSummary> for ArticleSummaryDocument {
    fn from(summary: &PublishedArticleSummary) -> Self {
        Self {
            slug: summary.slug.as_str().to_string(),
            title: summary.title.as_str().to_string(),
            category: summary.category.as_str().to_string(),
            section_path: summary.section_path.clone(),
            description: summary.description.clone(),
            tags: summary.tags.clone(),
            priority: summary.priority,
            created_at: summary.created_at.clone(),
            updated_at: summary.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArticleIndexDocument {
    pub articles: Vec<ArticleSummaryDocument>,
}

impl From<&[PublishedArticleSummary]> for ArticleIndexDocument {
    fn from(articles: &[PublishedArticleSummary]) -> Self {
        Self {
            articles: articles.iter().map(ArticleSummaryDocument::from).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryIndexDocument {
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    pub articles: Vec<ArticleSummaryDocument>,
}

impl From<&CategoryIndex> for CategoryIndexDocument {
    fn from(index: &CategoryIndex) -> Self {
        Self {
            category: index.category.as_str().to_string(),
            title: None,
            description: None,
            updated_at: None,
            articles: index
                .articles
                .iter()
                .map(ArticleSummaryDocument::from)
                .collect(),
        }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageArtifactDocument {
    pub page: PageKey,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub html: String,
    pub updated_at: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Category, PageKey, Slug, Title};

    fn release_pointer(prefix: &str) -> ArtifactReleasePointerDocument {
        ArtifactReleasePointerDocument {
            schema_version: ARTIFACT_RELEASE_SCHEMA_VERSION,
            release_id: "release-123".to_string(),
            artifact_prefix: prefix.to_string(),
            publisher_commit: "publisher-sha".to_string(),
            source_commit: "source-sha".to_string(),
            generated_at: "2026-07-12T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn artifact_release_pointer_accepts_immutable_relative_prefix() {
        assert!(
            release_pointer("releases/release-123/site")
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn artifact_release_pointer_rejects_unsupported_schema() {
        let mut pointer = release_pointer("releases/release-123/site");
        pointer.schema_version += 1;

        assert!(matches!(
            pointer.validate(),
            Err(crate::DomainError::ValidationError { field }) if field == "schema_version"
        ));
    }

    #[test]
    fn artifact_release_pointer_rejects_unsafe_prefixes() {
        for prefix in ["", "/releases/one", "releases/../site", "releases/one/"] {
            assert!(matches!(
                release_pointer(prefix).validate(),
                Err(crate::DomainError::ValidationError { field }) if field == "artifact_prefix"
            ));
        }
    }

    #[test]
    fn artifact_release_pointer_requires_rfc3339_utc_generated_at() {
        for generated_at in [
            "not-a-timestamp",
            "2026-07-12T12:00:00",
            "2026-07-12T21:00:00+09:00",
        ] {
            let mut pointer = release_pointer("releases/release-123/site");
            pointer.generated_at = generated_at.to_string();

            assert!(matches!(
                pointer.validate(),
                Err(crate::DomainError::ValidationError { field }) if field == "generated_at"
            ));
        }

        let mut pointer = release_pointer("releases/release-123/site");
        pointer.generated_at = "2026-07-12T12:00:00+00:00".to_string();
        assert!(pointer.validate().is_ok());
    }

    #[test]
    fn test_article_summary_document_conversion() {
        let summary = PublishedArticleSummary {
            slug: Slug::new("abc123def456".to_string()).unwrap(),
            title: Title::new("Test Output".to_string()).unwrap(),
            category: Category::Tech,
            section_path: vec!["block".to_string()],
            description: Some("Test description".to_string()),
            tags: vec!["test".to_string()],
            priority: Some(1),
            created_at: "2025-01-01T00:00:00+09:00".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
        };

        let json = serde_json::to_string(&ArticleSummaryDocument::from(&summary)).unwrap();

        assert!(json.contains("\"title\":\"Test Output\""));
        assert!(json.contains("\"slug\":\"abc123def456\""));
        assert!(json.contains("\"category\":\"tech\""));
        assert!(json.contains("\"section_path\":[\"block\"]"));
    }

    #[test]
    fn test_article_summary_document_keeps_empty_tags_field() {
        let summary = PublishedArticleSummary {
            slug: Slug::new("emptytags001".to_string()).unwrap(),
            title: Title::new("Empty Tags".to_string()).unwrap(),
            category: Category::Daily,
            section_path: vec![],
            description: None,
            tags: vec![],
            priority: None,
            created_at: "2025-01-01T00:00:00+09:00".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
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

        assert_eq!(document.section_path, Vec::<String>::new());
    }

    #[test]
    fn test_page_artifact_document_serialization() {
        let document = PageArtifactDocument {
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
    }

    #[test]
    fn test_category_index_document_deserialization_defaults_missing_metadata() {
        let json = r#"{
            "category":"tech",
            "articles":[]
        }"#;

        let document: CategoryIndexDocument = serde_json::from_str(json).unwrap();

        assert_eq!(document.title, None);
        assert_eq!(document.description, None);
        assert_eq!(document.updated_at, None);
    }
}
