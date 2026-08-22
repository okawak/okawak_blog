use crate::error::Result;
use domain::{
    ArticleMeta, CategoryArtifactDocument, HomeFragmentArtifactDocument, PageArtifactDocument,
    PublishableCategoryLanding, SiteMetadata, build_article_index, build_category_indexes,
    build_site_metadata,
};

/// Complete artifact bundle produced from validated content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteArtifacts {
    pub(crate) article_index: Vec<domain::PublishedArticleSummary>,
    pub(super) category_documents: Vec<CategoryArtifactDocument>,
    pub(super) page_documents: Vec<PageArtifactDocument>,
    pub(super) home_fragment: Option<HomeFragmentArtifactDocument>,
    pub(super) site_metadata: SiteMetadata,
}

pub(crate) fn build_site_artifacts(
    article_metas: Vec<ArticleMeta>,
    category_landings: Vec<PublishableCategoryLanding>,
    page_documents: Vec<PageArtifactDocument>,
    home_fragment: Option<HomeFragmentArtifactDocument>,
) -> Result<SiteArtifacts> {
    let category_metas = category_landings
        .iter()
        .map(|landing| landing.meta.clone())
        .collect();
    let article_index = build_article_index(&article_metas);
    let category_indexes = build_category_indexes(&article_metas, category_metas);
    let site_metadata = build_site_metadata(&category_indexes);
    let category_documents = category_indexes
        .iter()
        .map(|index| {
            let landing = category_landings
                .iter()
                .find(|landing| landing.meta.category == index.category)
                .ok_or_else(|| domain::DomainError::validation("category_landing"))?;
            CategoryArtifactDocument::try_from((index, landing.body.as_str()))
        })
        .collect::<domain::Result<Vec<_>>>()?;

    Ok(SiteArtifacts {
        article_index,
        category_documents,
        page_documents,
        home_fragment,
        site_metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        Category, CategoryLandingBody, CategoryLandingMeta, SectionPath, Slug, Timestamp, Title,
    };

    fn article_meta(
        title: &str,
        slug: &str,
        category: Category,
        priority: Option<i32>,
        created_at: &str,
    ) -> ArticleMeta {
        ArticleMeta {
            slug: Slug::new(slug.to_string()).unwrap(),
            title: Title::new(title.to_string()).unwrap(),
            category,
            section_path: SectionPath::default(),
            description: Some(format!("{title} summary")),
            tags: vec!["rust".to_string()],
            priority,
            created_at: Timestamp::new(created_at.to_string()).unwrap(),
            updated_at: Timestamp::new(created_at.to_string()).unwrap(),
        }
    }

    fn category_landing(category: Category, title: &str) -> CategoryLandingMeta {
        CategoryLandingMeta {
            category,
            title: Title::new(title.to_string()).unwrap(),
            description: None,
            updated_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
        }
    }

    fn publishable_category_landing(category: Category, title: &str) -> PublishableCategoryLanding {
        PublishableCategoryLanding::new(
            category_landing(category, title),
            CategoryLandingBody::new(format!("<p>{title}</p>")).unwrap(),
        )
    }

    #[test]
    fn test_build_site_artifacts() {
        let artifacts = build_site_artifacts(
            vec![
                article_meta(
                    "First",
                    "first0000001",
                    Category::Tech,
                    Some(1),
                    "2025-01-01T00:00:00+09:00",
                ),
                article_meta(
                    "Second",
                    "second000002",
                    Category::Daily,
                    Some(10),
                    "2025-01-02T00:00:00+09:00",
                ),
            ],
            vec![
                publishable_category_landing(Category::Tech, "Tech"),
                publishable_category_landing(Category::Daily, "Daily"),
            ],
            vec![],
            None,
        )
        .unwrap();

        assert_eq!(artifacts.article_index.len(), 2);
        assert_eq!(artifacts.category_documents.len(), 2);
        assert_eq!(artifacts.site_metadata.total_articles, 2);
        assert_eq!(artifacts.article_index[0].slug.as_str(), "second000002");
    }

    #[test]
    fn test_build_site_artifacts_includes_landing_only_category() {
        let artifacts = build_site_artifacts(
            vec![],
            vec![publishable_category_landing(Category::Physics, "Physics")],
            vec![],
            None,
        )
        .unwrap();

        assert_eq!(artifacts.category_documents.len(), 1);
        assert_eq!(artifacts.category_documents[0].category, "physics");
        assert_eq!(artifacts.site_metadata.categories.len(), 1);
        assert_eq!(artifacts.site_metadata.categories[0].article_count, 0);
    }

    #[test]
    fn test_build_site_artifacts_requires_category_landing() {
        let result = build_site_artifacts(
            vec![article_meta(
                "First",
                "first0000001",
                Category::Tech,
                Some(1),
                "2025-01-01T00:00:00+09:00",
            )],
            vec![],
            vec![],
            None,
        );

        assert!(result.is_err());
    }
}
