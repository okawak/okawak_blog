use crate::error::Result;

use super::attributes::{Category, Title};
use super::identifiers::{ArticleId, Slug};

/// Article entity with domain behavior expressed as methods.
#[derive(Debug, Clone, PartialEq)]
pub struct Article {
    pub id: ArticleId,
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub content: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String, // ISO8601 string
    pub updated_at: String,   // ISO8601 string
    pub is_published: bool,
}

impl Article {
    /// Creates a new article.
    pub fn create(
        title: String,
        content: String,
        category: Category,
        slug: String,
    ) -> Result<Self> {
        let title = Title::new(title)?;
        let slug = Slug::new(slug)?;

        // Generate the current timestamp as an ISO8601 string.
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

    /// Returns the current timestamp as an ISO8601 string.
    fn current_timestamp() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        // TODO: chrono クレートを導入して正確な ISO8601 日時文字列を生成する
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("時刻が正常に取得できません")
            .as_secs();
        format!("{}", secs)
    }

    /// Publishes the article.
    pub fn publish(&mut self) -> Result<()> {
        todo!("publish method not yet implemented")
    }

    /// Updates the article content.
    pub fn update_content(&mut self, _content: String) -> Result<()> {
        todo!("update_content method not yet implemented")
    }

    /// Sets the summary.
    pub fn set_summary(&mut self, _summary: String) {
        todo!("set_summary method not yet implemented")
    }

    /// Adds a tag.
    pub fn add_tag(&mut self, _tag: String) {
        todo!("add_tag method not yet implemented")
    }

    /// Builds the article URL.
    pub fn url(&self) -> String {
        todo!("url method not yet implemented")
    }

    /// Returns the published date in Japanese display format.
    pub fn published_date_jp(&self) -> String {
        todo!("published_date_jp method not yet implemented")
    }
}

/// Lightweight article summary for listing pages.
#[derive(Debug, Clone, PartialEq)]
pub struct ArticleSummary {
    pub id: ArticleId,
    pub slug: Slug,
    pub title: Title,
    pub category: Category,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub published_at: String, // ISO8601 string
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
