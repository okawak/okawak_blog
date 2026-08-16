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

        if category_index.landing.is_none() {
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
    use super::super::builder::build_site_artifacts;
    use super::super::validator::validate_site_artifacts;
    use super::*;
    use domain::{
        ArticleMeta, Category, CategoryLandingMeta, PageKey, SectionPath, Timestamp, Title,
    };
    use tempfile::TempDir;

    fn build_article_meta(
        title: &str,
        slug: &str,
        category: Category,
        priority: Option<i32>,
        created_at: &str,
    ) -> ArticleMeta {
        ArticleMeta {
            slug: Slug::new(slug.to_string()).unwrap(),
            title: Title::new(title.to_string()).unwrap(),
            category,
            section_path: SectionPath::default(),
            description: Some(format!("{title} summary")),
            tags: vec!["rust".to_string()],
            priority,
            created_at: Timestamp::new(created_at.to_string()).unwrap(),
            updated_at: Timestamp::new(created_at.to_string()).unwrap(),
        }
    }

    fn build_category_landing(
        category: Category,
        title: &str,
        description: Option<&str>,
    ) -> CategoryLandingMeta {
        CategoryLandingMeta {
            category,
            title: Title::new(title.to_string()).unwrap(),
            description: description.map(str::to_string),
            updated_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
        }
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
        assert!(
            artifacts
                .category_indexes
                .iter()
                .all(|index| index.landing.is_none())
        );
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
            vec![build_category_landing(
                Category::Tech,
                "Tech",
                Some("Tech landing"),
            )],
        );

        let article_path = write_article_page(
            &site_directories,
            Category::Tech,
            &article_meta.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        let category_page_path =
            write_category_page(&site_directories, Category::Tech, "<h1>Tech</h1>").unwrap();
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();

        assert!(article_path.exists());
        assert_eq!(
            article_path,
            site_directories
                .articles_dir
                .join("tech")
                .join("artifact00001.html")
        );
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
    fn test_write_home_fragment() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();

        let output_path = write_home_fragment(
            &site_directories,
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
        assert!(!site_directories.pages_dir.join("home.json").exists());
    }

    #[test]
    fn test_build_site_artifacts_includes_landing_only_category_in_indexes_and_metadata() {
        let artifacts = build_site_artifacts(
            vec![],
            vec![build_category_landing(Category::Physics, "Physics", None)],
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

    #[test]
    fn test_validate_site_artifacts_accepts_complete_site() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let article_meta = build_article_meta(
            "Artifact Test",
            "artifact00001",
            Category::Tech,
            Some(1),
            "2025-01-01T00:00:00+09:00",
        );
        let site_artifacts = build_site_artifacts(vec![article_meta.clone()], vec![]);

        write_article_page(
            &site_directories,
            Category::Tech,
            &article_meta.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();
        write_required_about_page(&site_directories);

        let summary = validate_site_artifacts(temp_dir.path().join("site")).unwrap();

        assert_eq!(summary.article_count, 1);
        assert_eq!(summary.category_count, 1);
    }

    #[test]
    fn test_validate_site_artifacts_rejects_empty_article_index() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let site_artifacts = build_site_artifacts(vec![], vec![]);
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("at least one article"));
    }

    #[test]
    fn test_validate_site_artifacts_rejects_missing_article_html() {
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

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("articles/tech/artifact00001.html")
        );
    }

    #[test]
    fn test_validate_site_artifacts_rejects_missing_category_page() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let article_meta = build_article_meta(
            "Artifact Test",
            "artifact00001",
            Category::Tech,
            Some(1),
            "2025-01-01T00:00:00+09:00",
        );
        let site_artifacts = build_site_artifacts(vec![article_meta.clone()], vec![]);
        write_article_page(
            &site_directories,
            Category::Tech,
            &article_meta.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();
        fs::remove_file(
            site_directories
                .categories_dir
                .join("tech")
                .join("page.html"),
        )
        .unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("categories/tech/page.html"));
    }

    #[test]
    fn test_validate_site_artifacts_rejects_missing_about_page() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let article_meta = build_article_meta(
            "Artifact Test",
            "artifact00001",
            Category::Tech,
            Some(1),
            "2025-01-01T00:00:00+09:00",
        );
        let site_artifacts = build_site_artifacts(vec![article_meta.clone()], vec![]);
        write_article_page(
            &site_directories,
            Category::Tech,
            &article_meta.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("pages/about.json"));
    }

    #[test]
    fn test_validate_site_artifacts_rejects_article_category_missing_from_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let site_directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let article_meta = build_article_meta(
            "Artifact Test",
            "artifact00001",
            Category::Tech,
            Some(1),
            "2025-01-01T00:00:00+09:00",
        );
        let site_artifacts = build_site_artifacts(vec![article_meta.clone()], vec![]);
        write_article_page(
            &site_directories,
            Category::Tech,
            &article_meta.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_artifacts(&site_directories, &site_artifacts).unwrap();
        write_required_about_page(&site_directories);
        write_json_pretty(
            &site_directories.metadata_dir.join("site.json"),
            &SiteMetadataDocument {
                total_articles: 1,
                categories: vec![],
            },
        )
        .unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("missing article categories"));
        assert!(error.to_string().contains("tech"));
    }

    fn write_required_about_page(site_directories: &SiteDirectories) {
        write_page_document(
            site_directories,
            &PageArtifactDocument {
                page: PageKey::new("about".to_string()).unwrap(),
                title: "About".to_string(),
                description: Some("About this site".to_string()),
                html: "<article><h1>About</h1></article>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            },
        )
        .unwrap();
    }
}
