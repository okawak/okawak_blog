//! Web-specific type definitions.

use serde::{Deserialize, Serialize};

/// Article summary used by the web crate in place of `domain::ArticleSummary`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticleSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub category: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String, // ISO8601 string
    pub is_published: bool,
}

/// Full article representation used by the web crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub category: String,
    pub content: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String,
    pub updated_at: String,
    pub is_published: bool,
}

/// Category representation used by the web crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub display_name: String,
}
