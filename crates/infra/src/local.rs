use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use domain::{
    ArticleIndexDocument, Category, CategoryArtifactDocument, HomeFragmentArtifactDocument,
    PageArtifactDocument, PageKey, SiteMetadataDocument, Slug,
};

use crate::{ArtifactReader, ArtifactSnapshot, DynArtifactSnapshot, Result};

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
    async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
        Ok(Arc::new(self.clone()))
    }
}

#[async_trait]
impl ArtifactSnapshot for LocalArtifactReader {
    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.read_json("articles/index.json").await
    }

    async fn read_category_document(
        &self,
        category: &Category,
    ) -> Result<CategoryArtifactDocument> {
        self.read_json(&format!("categories/{}.json", category.as_str()))
            .await
    }

    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        self.read_json("metadata/site.json").await
    }

    async fn read_article_html(&self, category: &Category, slug: &Slug) -> Result<String> {
        Ok(tokio::fs::read_to_string(self.artifact_path(&format!(
            "articles/{}/{}.html",
            category.as_str(),
            slug.as_str()
        )))
        .await?)
    }

    async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
        self.read_json("home.json").await
    }

    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
        self.read_json(&format!("pages/{}.json", page.as_str()))
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use domain::{ArticleSummaryDocument, CategoryMetadataDocument, SectionPath};
    use tempfile::TempDir;

    use super::*;

    fn write_fixture_site(root: &Path) {
        fs::create_dir_all(root.join("articles")).unwrap();
        fs::create_dir_all(root.join("categories")).unwrap();
        fs::create_dir_all(root.join("metadata")).unwrap();
        fs::create_dir_all(root.join("pages")).unwrap();

        fs::write(
            root.join("articles/index.json"),
            serde_json::to_string_pretty(&ArticleIndexDocument {
                articles: vec![ArticleSummaryDocument {
                    slug: "intro00000001".to_string(),
                    title: "Intro".to_string(),
                    category: "tech".to_string(),
                    section_path: SectionPath::new(vec!["block".to_string()]),
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
            root.join("categories/tech.json"),
            serde_json::to_string_pretty(&CategoryArtifactDocument {
                category: "tech".to_string(),
                title: "Tech".to_string(),
                description: Some("Tech landing".to_string()),
                html: "<article><h1>Tech</h1></article>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
                articles: vec![ArticleSummaryDocument {
                    slug: "intro00000001".to_string(),
                    title: "Intro".to_string(),
                    category: "tech".to_string(),
                    section_path: SectionPath::new(vec!["block".to_string()]),
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
        fs::create_dir_all(root.join("articles/tech")).unwrap();
        fs::write(
            root.join("articles/tech/intro00000001.html"),
            "<h1>Intro</h1>",
        )
        .unwrap();
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
        fs::write(
            root.join("home.json"),
            serde_json::to_string_pretty(&HomeFragmentArtifactDocument {
                title: "Home".to_string(),
                description: Some("Home introduction".to_string()),
                html: "<p>Welcome</p>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            })
            .unwrap(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn reads_fixture_site() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        let reader = LocalArtifactReader::new(temp_dir.path());
        let snapshot = reader.snapshot().await.unwrap();

        let document = snapshot.read_article_index().await.unwrap();
        let category = snapshot
            .read_category_document(&Category::Tech)
            .await
            .unwrap();
        let metadata = snapshot.read_site_metadata().await.unwrap();
        let html = snapshot
            .read_article_html(
                &Category::Tech,
                &Slug::new("intro00000001".to_string()).unwrap(),
            )
            .await
            .unwrap();
        let page = snapshot
            .read_page_document(&PageKey::new("about".to_string()).unwrap())
            .await
            .unwrap();
        let home_fragment = snapshot.read_home_fragment().await.unwrap();

        assert_eq!(document.articles.len(), 1);
        assert_eq!(document.articles[0].slug, "intro00000001");
        assert_eq!(category.category, "tech");
        assert_eq!(category.title, "Tech");
        assert_eq!(category.html, "<article><h1>Tech</h1></article>");
        assert_eq!(metadata.total_articles, 1);
        assert_eq!(html, "<h1>Intro</h1>");
        assert_eq!(page.page.as_str(), "about");
        assert_eq!(page.title, "About");
        assert_eq!(home_fragment.title, "Home");
        assert_eq!(home_fragment.html, "<p>Welcome</p>");
    }
}
