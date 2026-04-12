mod error;

pub use error::{InfraError, Result};

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use domain::{
    ArticleIndexDocument, Category, CategoryIndexDocument, PageArtifactDocument, PageKey,
    SiteMetadataDocument, Slug,
};
use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

const DEFAULT_LOCAL_SITE_ROOT: &str = "crates/publish/publisher/dist/site";
const ARTIFACT_SOURCE_ENV: &str = "OKAWAK_BLOG_ARTIFACT_SOURCE";
const ARTIFACT_LOCAL_ROOT_ENV: &str = "OKAWAK_BLOG_ARTIFACT_LOCAL_ROOT";
const ARTIFACT_BUCKET_ENV: &str = "OKAWAK_BLOG_ARTIFACT_BUCKET";
const ARTIFACT_PREFIX_ENV: &str = "OKAWAK_BLOG_ARTIFACT_PREFIX";

pub type DynArtifactReader = Arc<dyn ArtifactReader>;

#[async_trait]
pub trait ArtifactReader: Send + Sync {
    async fn read_article_index(&self) -> Result<ArticleIndexDocument>;
    async fn read_category_index(&self, category: &str) -> Result<CategoryIndexDocument>;
    async fn read_category_html(&self, category: &Category) -> Result<String>;
    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument>;
    async fn read_article_html(&self, slug: &Slug) -> Result<String>;
    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalArtifactReader {
    site_root: PathBuf,
}

impl LocalArtifactReader {
    pub fn new(site_root: impl AsRef<Path>) -> Self {
        Self {
            site_root: site_root.as_ref().to_path_buf(),
        }
    }

    pub fn site_root(&self) -> &Path {
        &self.site_root
    }

    fn artifact_path(&self, relative: &str) -> PathBuf {
        self.site_root.join(relative)
    }

    async fn read_json<T>(&self, relative: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let content = tokio::fs::read_to_string(self.artifact_path(relative)).await?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[async_trait]
impl ArtifactReader for LocalArtifactReader {
    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.read_json("articles/index.json").await
    }

    async fn read_category_index(&self, category: &str) -> Result<CategoryIndexDocument> {
        self.read_json(&format!("categories/{category}/index.json"))
            .await
    }

    async fn read_category_html(&self, category: &Category) -> Result<String> {
        Ok(tokio::fs::read_to_string(
            self.artifact_path(&format!("categories/{}/page.html", category.as_str())),
        )
        .await?)
    }

    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        self.read_json("metadata/site.json").await
    }

    async fn read_article_html(&self, slug: &Slug) -> Result<String> {
        Ok(tokio::fs::read_to_string(
            self.artifact_path(&format!("articles/{}.html", slug.as_str())),
        )
        .await?)
    }

    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
        self.read_json(&format!("pages/{}.json", page.as_str()))
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3ArtifactLocation {
    bucket: String,
    prefix: Option<String>,
}

impl S3ArtifactLocation {
    pub fn new(bucket: impl Into<String>, prefix: Option<impl Into<String>>) -> Result<Self> {
        let bucket = bucket.into().trim().to_string();
        if bucket.is_empty() {
            return Err(InfraError::MissingConfig(ARTIFACT_BUCKET_ENV));
        }

        let prefix = prefix
            .map(Into::into)
            .map(|value| value.trim_matches('/').to_string())
            .filter(|value| !value.is_empty());

        Ok(Self { bucket, prefix })
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn key_for(&self, relative: &str) -> String {
        let relative = relative.trim_start_matches('/');
        match &self.prefix {
            Some(prefix) => format!("{prefix}/{relative}"),
            None => relative.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct S3ArtifactReader {
    client: Client,
    location: S3ArtifactLocation,
}

impl S3ArtifactReader {
    pub fn new(client: Client, location: S3ArtifactLocation) -> Self {
        Self { client, location }
    }

    pub fn location(&self) -> &S3ArtifactLocation {
        &self.location
    }

    async fn read_text(&self, relative: &str) -> Result<String> {
        let key = self.location.key_for(relative);
        let response = self
            .client
            .get_object()
            .bucket(self.location.bucket())
            .key(&key)
            .send()
            .await
            .map_err(|source| InfraError::s3_read(self.location.bucket(), key.clone(), source))?;
        let bytes =
            response.body.collect().await.map_err(|source| {
                InfraError::s3_read(self.location.bucket(), key.clone(), source)
            })?;

        Ok(String::from_utf8(bytes.into_bytes().to_vec())?)
    }

    async fn read_json<T>(&self, relative: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let content = self.read_text(relative).await?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[async_trait]
impl ArtifactReader for S3ArtifactReader {
    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.read_json("articles/index.json").await
    }

    async fn read_category_index(&self, category: &str) -> Result<CategoryIndexDocument> {
        self.read_json(&format!("categories/{category}/index.json"))
            .await
    }

    async fn read_category_html(&self, category: &Category) -> Result<String> {
        self.read_text(&format!("categories/{}/page.html", category.as_str()))
            .await
    }

    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        self.read_json("metadata/site.json").await
    }

    async fn read_article_html(&self, slug: &Slug) -> Result<String> {
        self.read_text(&format!("articles/{}.html", slug.as_str()))
            .await
    }

    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
        let text = self
            .read_text(&format!("pages/{}.json", page.as_str()))
            .await?;
        Ok(serde_json::from_str(&text)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactSourceConfig {
    Local { site_root: PathBuf },
    S3 { location: S3ArtifactLocation },
}

impl ArtifactSourceConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_env_with(|key| env::var(key).ok())
    }

    fn from_env_with(mut read_var: impl FnMut(&str) -> Option<String>) -> Result<Self> {
        let source = read_var(ARTIFACT_SOURCE_ENV).unwrap_or_else(|| "local".to_string());
        match source.as_str() {
            "local" => Ok(Self::Local {
                site_root: PathBuf::from(
                    read_var(ARTIFACT_LOCAL_ROOT_ENV)
                        .unwrap_or_else(|| DEFAULT_LOCAL_SITE_ROOT.to_string()),
                ),
            }),
            "s3" => {
                let bucket = read_var(ARTIFACT_BUCKET_ENV)
                    .ok_or(InfraError::MissingConfig(ARTIFACT_BUCKET_ENV))?;
                let prefix = read_var(ARTIFACT_PREFIX_ENV);
                Ok(Self::S3 {
                    location: S3ArtifactLocation::new(bucket, prefix)?,
                })
            }
            unsupported => Err(InfraError::UnsupportedSource(unsupported.to_string())),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Local { .. } => "local",
            Self::S3 { .. } => "s3",
        }
    }
}

pub async fn build_artifact_reader(config: ArtifactSourceConfig) -> Result<DynArtifactReader> {
    match config {
        ArtifactSourceConfig::Local { site_root } => {
            Ok(Arc::new(LocalArtifactReader::new(site_root)))
        }
        ArtifactSourceConfig::S3 { location } => {
            let shared_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
            let client = Client::new(&shared_config);
            Ok(Arc::new(S3ArtifactReader::new(client, location)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ArticleSummaryDocument, CategoryMetadataDocument};
    use std::fs;
    use tempfile::TempDir;

    fn write_fixture_site(root: &Path) {
        fs::create_dir_all(root.join("articles")).unwrap();
        fs::create_dir_all(root.join("categories/tech")).unwrap();
        fs::create_dir_all(root.join("metadata")).unwrap();
        fs::create_dir_all(root.join("pages")).unwrap();

        fs::write(
            root.join("articles/index.json"),
            serde_json::to_string_pretty(&ArticleIndexDocument {
                articles: vec![ArticleSummaryDocument {
                    slug: "intro00000001".to_string(),
                    title: "Intro".to_string(),
                    category: "tech".to_string(),
                    section_path: vec!["block".to_string()],
                    description: Some("intro".to_string()),
                    tags: vec!["rust".to_string()],
                    priority: Some(1),
                    created_at: "2025-01-01T00:00:00+09:00".to_string(),
                    updated_at: "2025-01-01T00:00:00+09:00".to_string(),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("categories/tech/index.json"),
            serde_json::to_string_pretty(&CategoryIndexDocument {
                category: "tech".to_string(),
                title: Some("Tech".to_string()),
                description: Some("Tech landing".to_string()),
                updated_at: Some("2025-01-01T00:00:00+09:00".to_string()),
                articles: vec![ArticleSummaryDocument {
                    slug: "intro00000001".to_string(),
                    title: "Intro".to_string(),
                    category: "tech".to_string(),
                    section_path: vec!["block".to_string()],
                    description: Some("intro".to_string()),
                    tags: vec!["rust".to_string()],
                    priority: Some(1),
                    created_at: "2025-01-01T00:00:00+09:00".to_string(),
                    updated_at: "2025-01-01T00:00:00+09:00".to_string(),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("categories/tech/page.html"),
            "<article><h1>Tech</h1></article>",
        )
        .unwrap();
        fs::write(
            root.join("metadata/site.json"),
            serde_json::to_string_pretty(&SiteMetadataDocument {
                total_articles: 1,
                categories: vec![CategoryMetadataDocument {
                    category: "tech".to_string(),
                    article_count: 1,
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(root.join("articles/intro00000001.html"), "<h1>Intro</h1>").unwrap();
        fs::write(
            root.join("pages/about.json"),
            serde_json::to_string_pretty(&PageArtifactDocument {
                page: PageKey::new("about".to_string()).unwrap(),
                title: "About".to_string(),
                description: Some("About this site".to_string()),
                html: "<article><h1>About</h1></article>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            })
            .unwrap(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_local_artifact_reader_reads_fixture_site() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        let reader = LocalArtifactReader::new(temp_dir.path());

        let document = reader.read_article_index().await.unwrap();
        let category = reader.read_category_index("tech").await.unwrap();
        let category_html = reader.read_category_html(&Category::Tech).await.unwrap();
        let metadata = reader.read_site_metadata().await.unwrap();
        let html = reader
            .read_article_html(&Slug::new("intro00000001".to_string()).unwrap())
            .await
            .unwrap();
        let page = reader
            .read_page_document(&PageKey::new("about".to_string()).unwrap())
            .await
            .unwrap();

        assert_eq!(document.articles.len(), 1);
        assert_eq!(document.articles[0].slug, "intro00000001");
        assert_eq!(category.category, "tech");
        assert_eq!(category.title.as_deref(), Some("Tech"));
        assert_eq!(category_html, "<article><h1>Tech</h1></article>");
        assert_eq!(metadata.total_articles, 1);
        assert_eq!(html, "<h1>Intro</h1>");
        assert_eq!(page.page.as_str(), "about");
        assert_eq!(page.title, "About");
    }

    #[test]
    fn test_s3_artifact_location_builds_prefixed_keys() {
        let location = S3ArtifactLocation::new("blog-bucket", Some("/site/")).unwrap();

        assert_eq!(location.bucket(), "blog-bucket");
        assert_eq!(
            location.key_for("articles/index.json"),
            "site/articles/index.json"
        );
        assert_eq!(
            location.key_for("/metadata/site.json"),
            "site/metadata/site.json"
        );
    }

    #[test]
    fn test_artifact_source_config_defaults_to_local_reader() {
        let source = ArtifactSourceConfig::Local {
            site_root: PathBuf::from(DEFAULT_LOCAL_SITE_ROOT),
        };

        assert_eq!(source.kind(), "local");
    }

    #[test]
    fn test_artifact_source_config_from_env_defaults_to_local_site_root() {
        let source = ArtifactSourceConfig::from_env_with(|_| None).unwrap();

        assert_eq!(
            source,
            ArtifactSourceConfig::Local {
                site_root: PathBuf::from(DEFAULT_LOCAL_SITE_ROOT),
            }
        );
    }

    #[test]
    fn test_artifact_source_config_from_env_uses_local_override() {
        let source = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("local".to_string()),
            ARTIFACT_LOCAL_ROOT_ENV => Some("/tmp/site".to_string()),
            _ => None,
        })
        .unwrap();

        assert_eq!(
            source,
            ArtifactSourceConfig::Local {
                site_root: PathBuf::from("/tmp/site"),
            }
        );
    }

    #[test]
    fn test_artifact_source_config_from_env_builds_s3_location() {
        let source = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("s3".to_string()),
            ARTIFACT_BUCKET_ENV => Some("blog-bucket".to_string()),
            ARTIFACT_PREFIX_ENV => Some("/public/site/".to_string()),
            _ => None,
        })
        .unwrap();

        assert_eq!(
            source,
            ArtifactSourceConfig::S3 {
                location: S3ArtifactLocation::new("blog-bucket", Some("/public/site/")).unwrap(),
            }
        );
    }

    #[test]
    fn test_artifact_source_config_from_env_requires_s3_bucket() {
        let result = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("s3".to_string()),
            _ => None,
        });

        assert!(matches!(
            result,
            Err(InfraError::MissingConfig(ARTIFACT_BUCKET_ENV))
        ));
    }

    #[test]
    fn test_artifact_source_config_from_env_rejects_unsupported_source() {
        let result = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("filesystem".to_string()),
            _ => None,
        });

        assert!(matches!(
            result,
            Err(InfraError::UnsupportedSource(source)) if source == "filesystem"
        ));
    }
}
