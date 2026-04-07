//! Converters between domain models and DTOs.

use domain::{Article, ArticleSummary, Category, Slug, Title};
use std::str::FromStr;

use super::dto::*;

/// Converts a domain `Article` into an `ArticleDto`.
impl From<Article> for ArticleDto {
    fn from(article: Article) -> Self {
        Self {
            id: article.id.as_str().to_string(),
            slug: article.slug.as_str().to_string(),
            title: article.title.as_str().to_string(),
            category: article.category.as_str().to_string(),
            content: article.content,
            summary: article.summary,
            tags: article.tags,
            published_at: article.published_at,
            updated_at: article.updated_at,
            is_published: article.is_published,
        }
    }
}

/// Converts a domain `ArticleSummary` into an `ArticleSummaryDto`.
impl From<ArticleSummary> for ArticleSummaryDto {
    fn from(summary: ArticleSummary) -> Self {
        Self {
            id: summary.id.as_str().to_string(),
            slug: summary.slug.as_str().to_string(),
            title: summary.title.as_str().to_string(),
            category: summary.category.as_str().to_string(),
            summary: summary.summary,
            tags: summary.tags,
            published_at: summary.published_at,
            is_published: summary.is_published,
        }
    }
}

/// Converts `CreateArticleDto` into a domain `Article`.
impl TryFrom<CreateArticleDto> for Article {
    type Error = domain::DomainError;

    fn try_from(dto: CreateArticleDto) -> domain::Result<Self> {
        let category = Category::from_str(&dto.category)?;

        // Generate a slug from the title when the request does not provide one.
        let slug = if let Some(slug_str) = dto.slug {
            Slug::new(slug_str)?
        } else {
            let title = Title::new(dto.title.clone())?;
            domain::generate_slug_from_title(&title)?
        };

        Article::create(dto.title, dto.content, category, slug.as_str().to_string())
    }
}

/// Converts `Category` into `CategoryDto`.
impl From<Category> for CategoryDto {
    fn from(category: Category) -> Self {
        Self {
            id: category.as_str().to_string(),
            name: category.as_str().to_string(),
            display_name: category.display_name().to_string(),
        }
    }
}

/// Generic helpers for DTO ↔ domain conversions.
pub fn domain_to_dto<T, D>(domain_item: T) -> D
where
    D: From<T>,
{
    D::from(domain_item)
}

pub fn dto_to_domain<D, T>(dto: D) -> domain::Result<T>
where
    T: TryFrom<D, Error = domain::DomainError>,
{
    T::try_from(dto)
}
