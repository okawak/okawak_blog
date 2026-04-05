//! Data Transfer Objects - API用のデータ転送オブジェクト

use serde::{Deserialize, Serialize};

/// 記事DTO - JSON API用
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

/// 記事サマリーDTO - 一覧用
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

/// 記事作成リクエストDTO
#[derive(Debug, Clone, Deserialize)]
pub struct CreateArticleDto {
    pub title: String,
    pub content: String,
    pub category: String,
    pub slug: Option<String>, // 未指定の場合はタイトルから生成
    pub tags: Option<Vec<String>>,
}

/// 記事更新リクエストDTO
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateArticleDto {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// カテゴリーDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub display_name: String,
}

/// API統一レスポンス形式
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
