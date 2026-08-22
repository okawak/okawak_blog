use domain::{
    ArticleMeta, CategoryLandingMeta, SiteMetadata, build_article_index, build_category_indexes,
    build_site_metadata,
};

/// Complete artifact bundle produced from already-validated article metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteArtifacts {
    pub(crate) article_index: Vec<domain::PublishedArticleSummary>,
    pub(super) category_indexes: Vec<domain::CategoryIndex>,
    pub(super) site_metadata: SiteMetadata,
}

pub(crate) fn build_site_artifacts(
    article_metas: Vec<ArticleMeta>,
    category_landings: Vec<CategoryLandingMeta>,
) -> SiteArtifacts {
    let article_index = build_article_index(&article_metas);
    let category_indexes = build_category_indexes(&article_metas, category_landings);
    let site_metadata = build_site_metadata(&category_indexes);

    SiteArtifacts {
        article_index,
        category_indexes,
        site_metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Category, SectionPath, Slug, Timestamp, Title};

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
            vec![],
        );

        assert_eq!(artifacts.article_index.len(), 2);
        assert_eq!(artifacts.category_indexes.len(), 2);
        assert!(
            artifacts
                .category_indexes
                .iter()
                .all(|index| index.landing.is_none())
        );
        assert_eq!(artifacts.site_metadata.total_articles, 2);
        assert_eq!(artifacts.article_index[0].slug.as_str(), "second000002");
    }

    #[test]
    fn test_build_site_artifacts_includes_landing_only_category() {
        let artifacts =
            build_site_artifacts(vec![], vec![category_landing(Category::Physics, "Physics")]);

        assert_eq!(artifacts.category_indexes.len(), 1);
        assert_eq!(artifacts.category_indexes[0].category, Category::Physics);
        assert_eq!(artifacts.site_metadata.categories.len(), 1);
        assert_eq!(artifacts.site_metadata.categories[0].article_count, 0);
    }
}
