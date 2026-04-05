mod error;

pub use error::{InfraError, Result};

use domain::{ArticleIndexDocument, CategoryIndexDocument, SiteMetadataDocument, Slug};
use std::{
    fs,
    path::{Path, PathBuf},
};

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

    pub fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.read_json(self.site_root.join("articles/index.json"))
    }

    pub fn read_category_index(&self, category: &str) -> Result<CategoryIndexDocument> {
        self.read_json(
            self.site_root
                .join("categories")
                .join(format!("{category}.json")),
        )
    }

    pub fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        self.read_json(self.site_root.join("metadata/site.json"))
    }

    pub fn read_article_html(&self, slug: &Slug) -> Result<String> {
        Ok(fs::read_to_string(
            self.site_root
                .join("articles")
                .join(format!("{}.html", slug.as_str())),
        )?)
    }

    fn read_json<T>(&self, path: PathBuf) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let content = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ArticleSummaryDocument, Slug};
    use tempfile::TempDir;

    fn write_fixture_site(root: &Path) {
        fs::create_dir_all(root.join("articles")).unwrap();
        fs::create_dir_all(root.join("categories")).unwrap();
        fs::create_dir_all(root.join("metadata")).unwrap();

        fs::write(
            root.join("articles/index.json"),
            serde_json::to_string_pretty(&ArticleIndexDocument {
                articles: vec![ArticleSummaryDocument {
                    slug: "intro00000001".to_string(),
                    title: "Intro".to_string(),
                    category: "tech".to_string(),
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
            serde_json::to_string_pretty(&CategoryIndexDocument {
                category: "tech".to_string(),
                articles: vec![ArticleSummaryDocument {
                    slug: "intro00000001".to_string(),
                    title: "Intro".to_string(),
                    category: "tech".to_string(),
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
                categories: vec![domain::CategoryMetadataDocument {
                    category: "tech".to_string(),
                    article_count: 1,
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(root.join("articles/intro00000001.html"), "<h1>Intro</h1>").unwrap();
    }

    #[test]
    fn test_read_article_index() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        let reader = LocalArtifactReader::new(temp_dir.path());

        let document = reader.read_article_index().unwrap();

        assert_eq!(document.articles.len(), 1);
        assert_eq!(document.articles[0].slug, "intro00000001");
    }

    #[test]
    fn test_read_category_index() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        let reader = LocalArtifactReader::new(temp_dir.path());

        let document = reader.read_category_index("tech").unwrap();

        assert_eq!(document.category, "tech");
        assert_eq!(document.articles.len(), 1);
    }

    #[test]
    fn test_read_site_metadata_and_article_html() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        let reader = LocalArtifactReader::new(temp_dir.path());

        let metadata = reader.read_site_metadata().unwrap();
        let html = reader
            .read_article_html(&Slug::new("intro00000001".to_string()).unwrap())
            .unwrap();

        assert_eq!(metadata.total_articles, 1);
        assert_eq!(metadata.categories[0].category, "tech");
        assert_eq!(html, "<h1>Intro</h1>");
    }
}
