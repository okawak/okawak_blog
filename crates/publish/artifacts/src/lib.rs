mod error;

pub use error::{ArtifactsError, Result};

use domain::{
    ArticleIndexDocument, ArticleMeta, CategoryIndexDocument, SiteMetadata, SiteMetadataDocument,
    Slug, build_article_index, build_category_indexes, build_site_metadata,
};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

/// Complete artifact bundle produced from already-validated article metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteArtifacts {
    pub article_index: Vec<domain::PublishedArticleSummary>,
    pub category_indexes: Vec<domain::CategoryIndex>,
    pub site_metadata: SiteMetadata,
}

/// Output directories for generated local site artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteDirectories {
    pub articles_dir: PathBuf,
    pub categories_dir: PathBuf,
    pub metadata_dir: PathBuf,
}

impl SiteDirectories {
    pub fn prepare(output_dir: impl AsRef<Path>) -> Result<Self> {
        let site_root = output_dir.as_ref().join("site");
        let site_directories = Self {
            articles_dir: site_root.join("articles"),
            categories_dir: site_root.join("categories"),
            metadata_dir: site_root.join("metadata"),
        };

        fs::create_dir_all(&site_directories.articles_dir)?;
        fs::create_dir_all(&site_directories.categories_dir)?;
        fs::create_dir_all(&site_directories.metadata_dir)?;

        Ok(site_directories)
    }
}

pub fn build_site_artifacts(article_metas: Vec<ArticleMeta>) -> SiteArtifacts {
    let article_index = build_article_index(&article_metas);
    let category_indexes = build_category_indexes(&article_metas);
    let site_metadata = build_site_metadata(&article_metas);

    SiteArtifacts {
        article_index,
        category_indexes,
        site_metadata,
    }
}

pub fn write_article_page(
    site_directories: &SiteDirectories,
    slug: &Slug,
    html: &str,
) -> Result<PathBuf> {
    let output_file_path = site_directories
        .articles_dir
        .join(format!("{}.html", slug.as_str()));
    fs::write(&output_file_path, html)?;
    Ok(output_file_path)
}

pub fn write_site_artifacts(
    site_directories: &SiteDirectories,
    site_artifacts: &SiteArtifacts,
) -> Result<()> {
    write_json_pretty(
        &site_directories.articles_dir.join("index.json"),
        &ArticleIndexDocument::from(site_artifacts.article_index.as_slice()),
    )?;

    for category_index in &site_artifacts.category_indexes {
        write_json_pretty(
            &site_directories
                .categories_dir
                .join(format!("{}.json", category_index.category.as_str())),
            &CategoryIndexDocument::from(category_index),
        )?;
    }

    write_json_pretty(
        &site_directories.metadata_dir.join("site.json"),
        &SiteMetadataDocument::from(&site_artifacts.site_metadata),
    )?;

    Ok(())
}

fn write_json_pretty(path: &Path, value: &impl Serialize) -> Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ArticleMeta, ArticleMetaInput, Category, Title};
    use tempfile::TempDir;

    fn build_article_meta(
        title: &str,
        slug: &str,
        category: Category,
        priority: Option<i32>,
        created_at: &str,
    ) -> ArticleMeta {
        ArticleMeta::new(ArticleMetaInput {
            slug: Slug::new(slug.to_string()).unwrap(),
            title: Title::new(title.to_string()).unwrap(),
            category,
            section_path: vec![],
            description: Some(format!("{title} summary")),
            tags: vec!["rust".to_string()],
            priority,
            created_at: created_at.to_string(),
            updated_at: created_at.to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_build_site_artifacts() {
        let artifacts = build_site_artifacts(vec![
            build_article_meta(
                "First",
                "first0000001",
                Category::Tech,
                Some(1),
                "2025-01-01T00:00:00+09:00",
            ),
            build_article_meta(
                "Second",
                "second000002",
                Category::Daily,
                Some(10),
                "2025-01-02T00:00:00+09:00",
            ),
        ]);

        assert_eq!(artifacts.article_index.len(), 2);
        assert_eq!(artifacts.category_indexes.len(), 2);
        assert_eq!(artifacts.site_metadata.total_articles, 2);
        assert_eq!(artifacts.article_index[0].slug.as_str(), "second000002");
    }

    #[test]
    fn test_write_local_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let article_meta = build_article_meta(
            "Artifact Test",
            "artifact00001",
            Category::Tech,
            Some(1),
            "2025-01-01T00:00:00+09:00",
        );
        let site_artifacts = build_site_artifacts(vec![article_meta.clone()]);

        let article_path = write_article_page(
            &site_directories,
            &article_meta.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();

        assert!(article_path.exists());
        assert!(
            site_directories.articles_dir.join("index.json").exists(),
            "articles/index.json should exist"
        );
        assert!(
            site_directories.categories_dir.join("tech.json").exists(),
            "categories/tech.json should exist"
        );
        assert!(
            site_directories.metadata_dir.join("site.json").exists(),
            "metadata/site.json should exist"
        );
    }
}
