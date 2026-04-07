//! Data transfer objects used by the API.

use serde::{Deserialize, Serialize};

/// Article DTO for the JSON API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleDto {
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

/// Article summary DTO for list responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleSummaryDto {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub category: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String,
    pub is_published: bool,
}

/// Request DTO for creating an article.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateArticleDto {
    pub title: String,
    pub content: String,
    pub category: String,
    pub slug: Option<String>, // Generated from the title when omitted.
    pub tags: Option<Vec<String>>,
}

/// Request DTO for updating an article.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateArticleDto {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Category DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub display_name: String,
}

/// Unified API response shape.
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}
