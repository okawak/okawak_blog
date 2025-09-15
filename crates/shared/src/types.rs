use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 記事サマリー - 一覧表示用の軽量版（shared 版）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticleSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub category: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: DateTime<Utc>,
    pub is_published: bool,
}
