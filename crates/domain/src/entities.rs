//! ドメインモデル - Rustの型システムでビジネスルールを表現
//!
//! ADT (Algebraic Data Types) を活用したドメインモデリング

use crate::error::{DomainError, Result};
use std::{fmt, str::FromStr};

// =============================================================================
// Value Objects - 値オブジェクト（Rustの newtype pattern）
// =============================================================================

/// 記事ID - 型安全性を確保
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArticleId(String);

impl ArticleId {
    pub fn new() -> Self {
        // 簡易的なID生成（実装では外部でUUID生成）
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .hash(&mut hasher);
        let id = format!("article_{}", hasher.finish());
        Self(id)
    }

    pub fn from_string(id: String) -> Result<Self> {
        if id.is_empty() {
            return Err(DomainError::InvalidId {
                id: "IDは空にできません".to_string(),
            });
        }
        Ok(Self(id))
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        Self::from_string(s.to_string())
    }
}

/// スラッグ - URLセーフな識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Slug(String);

impl Slug {
    pub fn new(value: String) -> Result<Self> {
        if value.is_empty() {
            return Err(DomainError::InvalidSlug {
                slug: "スラッグは空にできません".to_string(),
            });
        }

        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DomainError::InvalidSlug {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title(String);

impl Title {
    pub fn new(value: String) -> Result<Self> {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::InvalidTitle {
                reason: "タイトルは空にできません".to_string(),
            });
        }

        if trimmed.len() > 200 {
            return Err(DomainError::InvalidTitle {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "tech" => Ok(Category::Tech),
            "daily" => Ok(Category::Daily),
            "statistics" => Ok(Category::Statistics),
            "physics" => Ok(Category::Physics),
            _ => Err(DomainError::InvalidCategory {
                category: s.to_string(),
            }),
        }
    }
}

// =============================================================================
// Entities - エンティティ（ビジネスロジックを持つ構造体）
// =============================================================================

/// 記事エンティティ - ビジネスロジックをメソッドで表現
#[derive(Debug, Clone, PartialEq)]
pub struct Article {
    pub id: ArticleId,
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub content: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String, // ISO8601文字列
    pub updated_at: String,   // ISO8601文字列
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

        // 現在時刻をISO8601文字列として生成
        let now = Self::current_timestamp();

        Ok(Self {
            id: ArticleId::new(),
            slug,
            title,
            category,
            content,
            summary: None,
            tags: Vec::new(),
            published_at: now.clone(),
            updated_at: now,
            is_published: false,
        })
    }

    /// 現在時刻をISO8601文字列として取得
    fn current_timestamp() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時刻が正常に取得できません");

        // 簡易的なISO8601形式（実際のプロジェクトではより正確な実装を使用）
        format!(
            "2025-09-20T{:02}:{:02}:{:02}Z",
            duration.as_secs() % 86400 / 3600,
            duration.as_secs() % 3600 / 60,
            duration.as_secs() % 60
        )
    }

    /// 記事を公開する（ビジネスルール）
    pub fn publish(&mut self) -> Result<()> {
        unimplemented!("publish method not yet implemented")
    }

    /// 記事を更新する
    pub fn update_content(&mut self, _content: String) -> Result<()> {
        unimplemented!("update_content method not yet implemented")
    }

    /// サマリーを設定
    pub fn set_summary(&mut self, _summary: String) {
        unimplemented!("set_summary method not yet implemented")
    }

    /// タグを追加
    pub fn add_tag(&mut self, _tag: String) {
        unimplemented!("add_tag method not yet implemented")
    }

    /// URLを生成
    pub fn url(&self) -> String {
        unimplemented!("url method not yet implemented")
    }

    /// 日本語形式の公開日
    pub fn published_date_jp(&self) -> String {
        unimplemented!("published_date_jp method not yet implemented")
    }
}

/// 記事サマリー - 一覧表示用の軽量版
#[derive(Debug, Clone, PartialEq)]
pub struct ArticleSummary {
    pub id: ArticleId,
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String, // ISO8601文字列
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
            published_at: article.published_at.clone(),
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
    fn test_placeholder() {
        // テスト実装は後で追加予定
        // todo!("モデルテストの実装");
    }
}
