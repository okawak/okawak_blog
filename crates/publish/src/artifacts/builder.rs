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
