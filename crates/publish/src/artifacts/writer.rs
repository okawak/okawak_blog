use super::builder::SiteArtifacts;
use crate::error::Result;

use domain::{
    ArticleIndexDocument, Category, CategoryIndexDocument, HomeFragmentArtifactDocument,
    PageArtifactDocument, SiteMetadataDocument, Slug,
};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

/// Output directories for generated local site artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteDirectories {
    home_fragment_path: PathBuf,
    articles_dir: PathBuf,
    categories_dir: PathBuf,
    metadata_dir: PathBuf,
    pages_dir: PathBuf,
}

impl SiteDirectories {
    pub(crate) fn prepare(output_dir: impl AsRef<Path>) -> Result<Self> {
        let site_root = output_dir.as_ref().join("site");
        let site_directories = Self {
            home_fragment_path: site_root.join("home.json"),
            articles_dir: site_root.join("articles"),
            categories_dir: site_root.join("categories"),
            metadata_dir: site_root.join("metadata"),
            pages_dir: site_root.join("pages"),
        };

        fs::create_dir_all(&site_directories.articles_dir)?;
        fs::create_dir_all(&site_directories.categories_dir)?;
        fs::create_dir_all(&site_directories.metadata_dir)?;
        fs::create_dir_all(&site_directories.pages_dir)?;

        Ok(site_directories)
    }
}

pub(crate) fn write_article_page(
    site_directories: &SiteDirectories,
    category: Category,
    slug: &Slug,
    html: &str,
) -> Result<PathBuf> {
    let article_dir = site_directories.articles_dir.join(category.as_str());
    fs::create_dir_all(&article_dir)?;
    let output_file_path = article_dir.join(format!("{}.html", slug.as_str()));
    fs::write(&output_file_path, html)?;
    Ok(output_file_path)
}

pub(crate) fn write_page_document(
    site_directories: &SiteDirectories,
    page_document: &PageArtifactDocument,
) -> Result<PathBuf> {
    let output_file_path = site_directories
        .pages_dir
        .join(format!("{}.json", page_document.page));
    write_json_pretty(&output_file_path, page_document)?;
    Ok(output_file_path)
}

pub(crate) fn write_home_fragment(
    site_directories: &SiteDirectories,
    home_fragment: &HomeFragmentArtifactDocument,
) -> Result<PathBuf> {
    write_json_pretty(&site_directories.home_fragment_path, home_fragment)?;
    Ok(site_directories.home_fragment_path.clone())
}

pub(crate) fn write_category_page(
    site_directories: &SiteDirectories,
    category: Category,
    html: &str,
) -> Result<PathBuf> {
    let category_dir = site_directories.categories_dir.join(category.as_str());
    fs::create_dir_all(&category_dir)?;
    let output_file_path = category_dir.join("page.html");
    fs::write(&output_file_path, html)?;
    Ok(output_file_path)
}

pub(crate) fn write_site_artifacts(
    site_directories: &SiteDirectories,
    site_artifacts: &SiteArtifacts,
) -> Result<()> {
    write_json_pretty(
        &site_directories.articles_dir.join("index.json"),
        &ArticleIndexDocument::from(site_artifacts.article_index.as_slice()),
    )?;
    for category_index in &site_artifacts.category_indexes {
        let category_dir = site_directories
            .categories_dir
            .join(category_index.category.as_str());
        fs::create_dir_all(&category_dir)?;
        write_json_pretty(
            &category_dir.join("index.json"),
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
    use super::super::builder::build_site_artifacts;
    use super::*;
    use domain::{ArticleMeta, CategoryLandingMeta, PageKey, SectionPath, Timestamp, Title};
    use tempfile::TempDir;

    fn article_meta() -> ArticleMeta {
        ArticleMeta {
            slug: Slug::new("artifact00001".to_string()).unwrap(),
            title: Title::new("Artifact Test".to_string()).unwrap(),
            category: Category::Tech,
            section_path: SectionPath::default(),
            description: Some("Artifact summary".to_string()),
            tags: vec!["rust".to_string()],
            priority: Some(1),
            created_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
            updated_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
        }
    }

    fn category_landing() -> CategoryLandingMeta {
        CategoryLandingMeta {
            category: Category::Tech,
            title: Title::new("Tech".to_string()).unwrap(),
            description: Some("Tech landing".to_string()),
            updated_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
        }
    }

    #[test]
    fn test_write_local_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let article = article_meta();
        let artifacts = build_site_artifacts(vec![article.clone()], vec![category_landing()]);

        let article_path = write_article_page(
            &directories,
            article.category,
            &article.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        let category_path =
            write_category_page(&directories, Category::Tech, "<h1>Tech</h1>").unwrap();
        write_site_artifacts(&directories, &artifacts).unwrap();

        for path in [
            article_path,
            category_path,
            directories.articles_dir.join("index.json"),
            directories.categories_dir.join("tech/index.json"),
            directories.metadata_dir.join("site.json"),
        ] {
            assert!(path.exists(), "{} should exist", path.display());
        }
    }

    #[test]
    fn test_write_page_document() {
        let temp_dir = TempDir::new().unwrap();
        let directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let output_path = write_page_document(
            &directories,
            &PageArtifactDocument {
                page: PageKey::new("about".to_string()).unwrap(),
                title: "About".to_string(),
                description: Some("About this site".to_string()),
                html: "<article><h1>About</h1></article>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            },
        )
        .unwrap();

        assert_eq!(output_path, directories.pages_dir.join("about.json"));
        assert!(output_path.exists());
    }

    #[test]
    fn test_write_home_fragment() {
        let temp_dir = TempDir::new().unwrap();
        let directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let output_path = write_home_fragment(
            &directories,
            &HomeFragmentArtifactDocument {
                title: "Home".to_string(),
                description: Some("Home introduction".to_string()),
                html: "<p>Welcome</p>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            },
        )
        .unwrap();

        assert_eq!(output_path, temp_dir.path().join("site/home.json"));
        assert!(output_path.exists());
        assert!(!directories.pages_dir.join("home.json").exists());
    }
}
