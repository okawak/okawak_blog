use domain::{
    ArticleMeta, Category, SiteMetadata, build_article_index, build_category_indexes,
    build_site_metadata,
};

/// Complete artifact bundle produced from already-validated article metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteArtifacts {
    pub(crate) article_index: Vec<domain::PublishedArticleSummary>,
    pub(super) category_indexes: Vec<domain::CategoryIndex>,
    pub(super) category_landings: Vec<CategoryLandingMeta>,
    pub(super) site_metadata: SiteMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryLandingMeta {
    pub(crate) category: Category,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) updated_at: String,
}

pub(crate) fn build_site_artifacts(
    article_metas: Vec<ArticleMeta>,
    mut category_landings: Vec<CategoryLandingMeta>,
) -> SiteArtifacts {
    let article_index = build_article_index(&article_metas);
    let mut category_indexes = build_category_indexes(&article_metas);
    let mut site_metadata = build_site_metadata(&article_metas);

    category_landings.sort_by(|a, b| a.category.as_str().cmp(b.category.as_str()));
    for landing in &category_landings {
        if category_indexes
            .iter()
            .all(|index| index.category != landing.category)
        {
            category_indexes.push(domain::CategoryIndex {
                category: landing.category,
                articles: vec![],
            });
        }

        if site_metadata
            .categories
            .iter()
            .all(|metadata| metadata.category != landing.category)
        {
            site_metadata.categories.push(domain::CategoryMetadata {
                category: landing.category,
                article_count: 0,
            });
        }
    }

    category_indexes.sort_by(|a, b| a.category.as_str().cmp(b.category.as_str()));
    site_metadata
        .categories
        .sort_by(|a, b| a.category.as_str().cmp(b.category.as_str()));

    SiteArtifacts {
        article_index,
        category_indexes,
        category_landings,
        site_metadata,
    }
}
