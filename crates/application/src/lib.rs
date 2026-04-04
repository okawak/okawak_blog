//! Application layer for publisher-oriented use cases.
//!
//! This crate keeps orchestration logic pure and builds artifact bundles from
//! domain publishable articles without performing I/O itself.

use domain::{
    CategoryIndex, PublishableArticle, PublishedArticleSummary, SiteMetadata, Slug,
    build_article_index, build_category_indexes, build_site_metadata,
};

/// Rendered article page artifact written by the publisher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleArtifact {
    pub slug: Slug,
    pub html: String,
}

/// Complete artifact bundle produced from publishable articles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteArtifacts {
    pub article_pages: Vec<ArticleArtifact>,
    pub article_index: Vec<PublishedArticleSummary>,
    pub category_indexes: Vec<CategoryIndex>,
    pub site_metadata: SiteMetadata,
}

/// Build the site artifact bundle from already-rendered publishable articles.
pub fn build_site_artifacts(articles: Vec<PublishableArticle>) -> SiteArtifacts {
    let article_index = build_article_index(&articles);
    let category_indexes = build_category_indexes(&articles);
    let site_metadata = build_site_metadata(&articles);
    let article_pages = articles
        .into_iter()
        .map(|article| ArticleArtifact {
            slug: article.meta.slug.clone(),
            html: article.body.html,
        })
        .collect();

    SiteArtifacts {
        article_pages,
        article_index,
        category_indexes,
        site_metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        ArticleBody, ArticleMeta, ArticleMetaInput, Category, PublishableArticle, Slug, Title,
    };

    fn build_article(
        title: &str,
        slug: &str,
        category: Category,
        priority: Option<i32>,
        created_at: &str,
    ) -> PublishableArticle {
        let meta = ArticleMeta::new(ArticleMetaInput {
            slug: Slug::new(slug.to_string()).unwrap(),
            title: Title::new(title.to_string()).unwrap(),
            category,
            description: Some(format!("{title} summary")),
            tags: vec!["rust".to_string()],
            priority,
            created_at: created_at.to_string(),
            updated_at: created_at.to_string(),
        })
        .unwrap();
        let body = ArticleBody::new(format!("<h1>{title}</h1>")).unwrap();

        PublishableArticle::new(meta, body)
    }

    #[test]
    fn test_build_site_artifacts() {
        let artifacts = build_site_artifacts(vec![
            build_article(
                "First",
                "first0000001",
                Category::Tech,
                Some(1),
                "2025-01-01T00:00:00+09:00",
            ),
            build_article(
                "Second",
                "second000002",
                Category::Daily,
                Some(10),
                "2025-01-02T00:00:00+09:00",
            ),
        ]);

        assert_eq!(artifacts.article_pages.len(), 2);
        assert_eq!(artifacts.article_index.len(), 2);
        assert_eq!(artifacts.category_indexes.len(), 2);
        assert_eq!(artifacts.site_metadata.total_articles, 2);
        assert_eq!(artifacts.article_index[0].slug.as_str(), "second000002");
    }
}
