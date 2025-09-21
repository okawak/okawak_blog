//! Web Types - フロントエンド専用型定義

use serde::{Deserialize, Serialize};

/// 記事サマリー - web専用型（domain::ArticleSummaryの代替）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticleSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub category: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String, // ISO8601文字列
    pub is_published: bool,
}

/// 記事詳細 - web専用型
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

/// カテゴリー - web専用型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub display_name: String,
}
