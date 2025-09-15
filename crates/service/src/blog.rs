use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "ssr")]
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use server_fn::error::NoCustomError;

use crate::{Result, config::ServiceConfig, error::ServiceError, s3::S3Service};
use domain::{Article, ArticleId, ArticleSummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostMetadata {
    pub title: String,
    pub slug: String,
    pub published_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub excerpt: Option<String>,
    pub author: String,
}

#[derive(Clone)]
pub struct BlogService {
    s3: S3Service,
    config: ServiceConfig,
    cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, (Article, DateTime<Utc>)>>>,
}

impl BlogService {
    pub async fn new(config: ServiceConfig) -> Result<Self> {
        let s3 = S3Service::new(&config.s3).await?;
        let cache = std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new()));

        Ok(Self { s3, config, cache })
    }

    pub async fn get_post(&self, id: &ArticleId) -> Result<Article> {
        let cache_key = id.to_string();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some((post, cached_at)) = cache.get(&cache_key) {
                let cache_duration =
                    chrono::Duration::seconds(self.config.blog.cache_duration_seconds as i64);
                if Utc::now().signed_duration_since(*cached_at) < cache_duration {
                    return Ok(post.clone());
                }
            }
        }

        // Fetch from S3
        let s3_key = format!("{}{}.md", self.config.blog.posts_prefix, id.as_str());
        let content = self
            .s3
            .get_object(&s3_key)
            .await
            .map_err(|_| ServiceError::ArticleNotFound { id: id.to_string() })?;

        let post = self.parse_markdown_post(&content, id)?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, (post.clone(), Utc::now()));
        }

        Ok(post)
    }

    pub async fn list_posts(&self) -> Result<Vec<ArticleSummary>> {
        let keys = self.s3.list_objects(&self.config.blog.posts_prefix).await?;

        let mut summaries = Vec::new();

        for key in keys {
            if !key.ends_with(".md") {
                continue;
            }

            // Extract post ID from key
            let _post_id = key
                .strip_prefix(&self.config.blog.posts_prefix)
                .and_then(|s| s.strip_suffix(".md"))
                .ok_or_else(|| {
                    ServiceError::Internal(format!("Invalid post key format: {}", key))
                })?;

            let id = ArticleId::default();

            // For listing, we only need basic metadata - could be optimized with metadata files
            if let Ok(post) = self.get_post(&id).await {
                summaries.push(ArticleSummary::from(&post));
            }
        }

        // Sort by published date (newest first)
        summaries.sort_by(|a, b| b.published_at.cmp(&a.published_at));

        Ok(summaries)
    }

    fn parse_markdown_post(&self, content: &str, id: &ArticleId) -> Result<Article> {
        // Simple markdown parsing - in a real implementation, this would be more sophisticated
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Err(ServiceError::Internal("Empty markdown content".to_string()));
        }

        // Extract frontmatter if present (between --- lines)
        let (metadata, content_start) = if lines[0] == "---" {
            let end_index = lines[1..]
                .iter()
                .position(|&line| line == "---")
                .map(|i| i + 1)
                .ok_or_else(|| ServiceError::Internal("Unclosed frontmatter".to_string()))?;

            let frontmatter = lines[1..=end_index - 1].join("\n");
            let metadata: BlogPostMetadata = serde_yaml::from_str(&frontmatter).map_err(|e| {
                ServiceError::Internal(format!("Failed to parse frontmatter: {}", e))
            })?;

            (metadata, end_index + 1)
        } else {
            // Default metadata if no frontmatter
            (
                BlogPostMetadata {
                    title: format!("Post {}", id.as_str()),
                    slug: id.as_str().to_string(),
                    published_at: Utc::now(),
                    updated_at: None,
                    tags: vec![],
                    excerpt: None,
                    author: "Unknown".to_string(),
                },
                0,
            )
        };

        let content_body = lines[content_start..].join("\n");

        // domainのArticle::createを使用して適切にエンティティを作成
        let mut article = Article::create(
            metadata.title,
            content_body,
            domain::Category::Tech, // デフォルトカテゴリ、後で適切に設定
            metadata.slug,
        )
        .map_err(|e| ServiceError::DomainError(e))?;

        // その他のフィールドを設定
        article.tags = metadata.tags;
        article.published_at = metadata.published_at;
        if let Some(updated_at) = metadata.updated_at {
            article.updated_at = updated_at;
        }

        Ok(article)
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

// Leptos Server Functions for web layer integration (ダミー実装)
#[cfg(feature = "ssr")]
#[server(GetBlogPost, "/api")]
pub async fn get_blog_post(_id: String) -> std::result::Result<Article, ServerFnError> {
    // 一旦ダミーデータを返す
    let article = Article::create(
        "サンプル記事".to_string(),
        "これはサンプル記事の内容です。".to_string(),
        domain::Category::Tech,
        "sample-article".to_string(),
    )
    .map_err(|e| ServerFnError::<NoCustomError>::ServerError(e.to_string()))?;

    Ok(article)
}

#[cfg(feature = "ssr")]
#[server(ListBlogPosts, "/api")]
pub async fn list_blog_posts() -> std::result::Result<Vec<ArticleSummary>, ServerFnError> {
    // 一旦空のリストを返す
    Ok(vec![])
}
