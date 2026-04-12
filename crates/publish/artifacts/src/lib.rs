mod error;

pub use error::{ArtifactsError, Result};

use domain::{
    ArticleIndexDocument, ArticleMeta, Category, CategoryIndexDocument, PageArtifactDocument,
    SiteMetadata, SiteMetadataDocument, Slug, build_article_index, build_category_indexes,
    build_site_metadata,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

/// Complete artifact bundle produced from already-validated article metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteArtifacts {
    pub article_index: Vec<domain::PublishedArticleSummary>,
    pub category_indexes: Vec<domain::CategoryIndex>,
    pub category_landings: Vec<CategoryLandingMetadata>,
    pub site_metadata: SiteMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryLandingMetadata {
    pub category: Category,
    pub title: String,
    pub description: Option<String>,
    pub updated_at: String,
}

/// Output directories for generated local site artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteDirectories {
    pub articles_dir: PathBuf,
    pub categories_dir: PathBuf,
    pub metadata_dir: PathBuf,
    pub pages_dir: PathBuf,
}

impl SiteDirectories {
    pub fn prepare(output_dir: impl AsRef<Path>) -> Result<Self> {
        let site_root = output_dir.as_ref().join("site");
        let site_directories = Self {
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

pub fn build_site_artifacts(
    article_metas: Vec<ArticleMeta>,
    mut category_landings: Vec<CategoryLandingMetadata>,
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

pub fn write_page_document(
    site_directories: &SiteDirectories,
    page_document: &PageArtifactDocument,
) -> Result<PathBuf> {
    let output_file_path = site_directories
        .pages_dir
        .join(format!("{}.json", page_document.page));
    write_json_pretty(&output_file_path, page_document)?;
    Ok(output_file_path)
}

pub fn write_category_page(
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

pub fn write_site_artifacts(
    site_directories: &SiteDirectories,
    site_artifacts: &SiteArtifacts,
) -> Result<()> {
    write_json_pretty(
        &site_directories.articles_dir.join("index.json"),
        &ArticleIndexDocument::from(site_artifacts.article_index.as_slice()),
    )?;
    let category_landings_by_category: HashMap<_, _> = site_artifacts
        .category_landings
        .iter()
        .map(|landing| (landing.category, landing))
        .collect();

    for category_index in &site_artifacts.category_indexes {
        let category_dir = site_directories
            .categories_dir
            .join(category_index.category.as_str());
        fs::create_dir_all(&category_dir)?;
        let landing = category_landings_by_category
            .get(&category_index.category)
            .copied();
        write_json_pretty(
            &category_dir.join("index.json"),
            &CategoryIndexDocument {
                category: category_index.category.as_str().to_string(),
                title: landing.map(|landing| landing.title.clone()),
                description: landing.and_then(|landing| landing.description.clone()),
                updated_at: landing.map(|landing| landing.updated_at.clone()),
                articles: category_index
                    .articles
                    .iter()
                    .map(domain::ArticleSummaryDocument::from)
                    .collect(),
            },
        )?;

        if landing.is_none() {
            fs::write(
                category_dir.join("page.html"),
                build_fallback_category_page_html(category_index),
            )?;
        }
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

fn build_fallback_category_page_html(category_index: &domain::CategoryIndex) -> String {
    format!(
        "<article><h1>{}</h1><p>{}カテゴリの記事一覧です。</p></article>",
        category_index.category.display_name(),
        category_index.category.display_name(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ArticleMeta, ArticleMetaInput, Category, PageKey, Title};
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
        let artifacts = build_site_artifacts(
            vec![
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
            ],
            vec![],
        );

        assert_eq!(artifacts.article_index.len(), 2);
        assert_eq!(artifacts.category_indexes.len(), 2);
        assert!(artifacts.category_landings.is_empty());
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
        let site_artifacts = build_site_artifacts(
            vec![article_meta.clone()],
            vec![CategoryLandingMetadata {
                category: Category::Tech,
                title: "Tech".to_string(),
                description: Some("Tech landing".to_string()),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            }],
        );

        let article_path = write_article_page(
            &site_directories,
            &article_meta.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        let category_page_path =
            write_category_page(&site_directories, Category::Tech, "<h1>Tech</h1>").unwrap();
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();

        assert!(article_path.exists());
        assert!(category_page_path.exists());
        assert!(
            site_directories.articles_dir.join("index.json").exists(),
            "articles/index.json should exist"
        );
        assert!(
            site_directories
                .categories_dir
                .join("tech/index.json")
                .exists(),
            "categories/tech/index.json should exist"
        );
        assert!(
            site_directories
                .categories_dir
                .join("tech/page.html")
                .exists(),
            "categories/tech/page.html should exist"
        );
        assert!(
            site_directories.metadata_dir.join("site.json").exists(),
            "metadata/site.json should exist"
        );
        assert!(
            site_directories.pages_dir.exists(),
            "pages directory should exist"
        );
    }

    #[test]
    fn test_write_page_document() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();

        let output_path = write_page_document(
            &site_directories,
            &PageArtifactDocument {
                page: PageKey::new("about".to_string()).unwrap(),
                title: "About".to_string(),
                description: Some("About this site".to_string()),
                html: "<article><h1>About</h1></article>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            },
        )
        .unwrap();

        assert_eq!(output_path, site_directories.pages_dir.join("about.json"));
        assert!(output_path.exists());
    }

    #[test]
    fn test_build_site_artifacts_includes_landing_only_category_in_indexes_and_metadata() {
        let artifacts = build_site_artifacts(
            vec![],
            vec![CategoryLandingMetadata {
                category: Category::Physics,
                title: "Physics".to_string(),
                description: None,
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            }],
        );

        assert_eq!(artifacts.category_indexes.len(), 1);
        assert_eq!(artifacts.category_indexes[0].category, Category::Physics);
        assert_eq!(artifacts.site_metadata.categories.len(), 1);
        assert_eq!(artifacts.site_metadata.categories[0].article_count, 0);
    }

    #[test]
    fn test_write_site_artifacts_creates_fallback_category_page_when_landing_is_missing() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let article_meta = build_article_meta(
            "Artifact Test",
            "artifact00001",
            Category::Tech,
            Some(1),
            "2025-01-01T00:00:00+09:00",
        );
        let site_artifacts = build_site_artifacts(vec![article_meta], vec![]);

        write_site_artifacts(&site_directories, &site_artifacts).unwrap();

        let fallback_html = fs::read_to_string(
            site_directories
                .categories_dir
                .join("tech")
                .join("page.html"),
        )
        .unwrap();

        assert!(fallback_html.contains("<h1>技術</h1>"));
        assert!(fallback_html.contains("技術カテゴリの記事一覧です。"));
    }
}
