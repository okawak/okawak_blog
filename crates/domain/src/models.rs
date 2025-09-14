//! ドメインモデル - Rustの型システムでビジネスルールを表現
//!
//! ADT (Algebraic Data Types) を活用したドメインモデリング

use crate::error::{CoreError, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

// =============================================================================
// Value Objects - 値オブジェクト（Rustの newtype pattern）
// =============================================================================

/// 記事ID - 型安全性を確保
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArticleId(Uuid);

impl ArticleId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for ArticleId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArticleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ArticleId {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(s).map_err(|_| CoreError::InvalidSlug {
            slug: s.to_string(),
        })?;
        Ok(Self(uuid))
    }
}

/// スラッグ - URLセーフな識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Slug(String);

impl Slug {
    pub fn new(value: String) -> Result<Self> {
        if value.is_empty() {
            return Err(CoreError::InvalidSlug {
                slug: "スラッグは空にできません".to_string(),
            });
        }

        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(CoreError::InvalidSlug {
                slug: "スラッグは英数字、ハイフン、アンダースコアのみ使用可能です".to_string(),
            });
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 記事タイトル - ビジネスルールを型で表現
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Title(String);

impl Title {
    pub fn new(value: String) -> Result<Self> {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(CoreError::InvalidTitle {
                reason: "タイトルは空にできません".to_string(),
            });
        }

        if trimmed.len() > 200 {
            return Err(CoreError::InvalidTitle {
                reason: "タイトルは200文字以内である必要があります".to_string(),
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// カテゴリ - 列挙型でドメインを制限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Tech,
    Daily,
    Statistics,
    Physics,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Tech => "tech",
            Category::Daily => "daily",
            Category::Statistics => "statistics",
            Category::Physics => "physics",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Tech => "技術",
            Category::Daily => "日常",
            Category::Statistics => "統計学",
            Category::Physics => "物理学",
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Category {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "tech" => Ok(Category::Tech),
            "daily" => Ok(Category::Daily),
            "statistics" => Ok(Category::Statistics),
            "physics" => Ok(Category::Physics),
            _ => Err(CoreError::InvalidCategory {
                category: s.to_string(),
            }),
        }
    }
}

// =============================================================================
// Entities - エンティティ（ビジネスロジックを持つ構造体）
// =============================================================================

/// 記事エンティティ - ビジネスロジックをメソッドで表現
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub id: ArticleId,
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub content: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_published: bool,
}

impl Article {
    /// 新しい記事を作成（ファクトリーメソッド）
    pub fn create(
        title: String,
        content: String,
        category: Category,
        slug: String,
    ) -> Result<Self> {
        let title = Title::new(title)?;
        let slug = Slug::new(slug)?;
        let now = Utc::now();

        Ok(Self {
            id: ArticleId::new(),
            slug,
            title,
            category,
            content,
            summary: None,
            tags: Vec::new(),
            published_at: now,
            updated_at: now,
            is_published: false,
        })
    }

    /// 記事を公開する（ビジネスルール）
    pub fn publish(&mut self) -> Result<()> {
        todo!("記事公開機能の実装")
    }

    /// 記事を更新する
    pub fn update_content(&mut self, _content: String) -> Result<()> {
        todo!("記事更新機能の実装")
    }

    /// サマリーを設定
    pub fn set_summary(&mut self, _summary: String) {
        todo!("サマリー設定機能の実装")
    }

    /// タグを追加
    pub fn add_tag(&mut self, _tag: String) {
        todo!("タグ追加機能の実装")
    }

    /// URLを生成
    pub fn url(&self) -> String {
        todo!("URL生成機能の実装")
    }

    /// 日本語形式の公開日
    pub fn published_date_jp(&self) -> String {
        todo!("日本語日付形式変換機能の実装")
    }
}

/// 記事サマリー - 一覧表示用の軽量版
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArticleSummary {
    pub id: ArticleId,
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: DateTime<Utc>,
    pub is_published: bool,
}

impl From<&Article> for ArticleSummary {
    fn from(article: &Article) -> Self {
        Self {
            id: article.id.clone(),
            slug: article.slug.clone(),
            title: article.title.clone(),
            category: article.category,
            summary: article.summary.clone(),
            tags: article.tags.clone(),
            published_at: article.published_at,
            is_published: article.is_published,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_placeholder() {
        // テスト実装は後で追加予定
        todo!("モデルテストの実装");
    }
}
