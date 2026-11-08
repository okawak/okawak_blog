use async_trait::async_trait;
use blog_domain::{
    ArticleIndexDocument, Category, CategoryArtifactDocument, ContentAssetName,
    HomeFragmentArtifactDocument, Locale, PageArtifactDocument, PageKey, SiteLocalesDocument,
    SiteMetadataDocument, Slug,
};
use blog_infra::{ArtifactSnapshot, InfraError, Result};

// A reader implementing only the original required contract remains Japanese-only.
struct LegacySnapshot;

fn unavailable<T>() -> Result<T> {
    Err(InfraError::Io(std::io::Error::other(
        "not provided by this fixture",
    )))
}

#[async_trait]
impl ArtifactSnapshot for LegacySnapshot {
    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        unavailable()
    }
    async fn read_category_document(&self, _: &Category) -> Result<CategoryArtifactDocument> {
        unavailable()
    }
    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        unavailable()
    }
    async fn read_article_html(&self, _: &Category, _: &Slug) -> Result<String> {
        unavailable()
    }
    async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
        unavailable()
    }
    async fn read_page_document(&self, _: &PageKey) -> Result<PageArtifactDocument> {
        unavailable()
    }
}

#[tokio::test]
async fn optional_contract_defaults_do_not_invent_assets_or_release_validators() {
    let snapshot = LegacySnapshot;
    let name = ContentAssetName::new(format!("{}.png", "a".repeat(64))).unwrap();
    assert_eq!(snapshot.cache_identity(), None);
    assert_eq!(snapshot.last_modified(), None);
    assert_eq!(snapshot.read_content_asset(&name).await.unwrap(), None);
    assert_eq!(
        snapshot.read_locales().await.unwrap(),
        SiteLocalesDocument::default()
    );
    assert!(snapshot.read_tag_labels().await.unwrap().is_empty());
    assert!(snapshot.localized(Locale::En).await.unwrap().is_none());
}
