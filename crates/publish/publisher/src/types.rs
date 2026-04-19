use artifacts::CategoryLandingMetadata;
use domain::{ArticleMeta, Category, PageArtifactDocument, PageKey};
use ingest::ObsidianFrontMatter;

pub(crate) struct RenderedArticle {
    pub(crate) meta: ArticleMeta,
    pub(crate) html: String,
}

pub(crate) struct RenderedPage {
    pub(crate) document: PageArtifactDocument,
}

pub(crate) struct RenderedCategoryLanding {
    pub(crate) metadata: CategoryLandingMetadata,
    pub(crate) html: String,
}

pub(crate) struct ParsedArticleFile {
    pub(crate) category: Category,
    pub(crate) slug: String,
    pub(crate) mapping_key: String,
    pub(crate) section_path: Vec<String>,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedPageFile {
    pub(crate) page: PageKey,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedCategoryFile {
    pub(crate) category: Category,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}
