use crate::error::{PublishError, Result};
use domain::{
    ArticleIndexDocument, Category, CategoryArtifactDocument, PageArtifactDocument,
    SiteMetadataDocument, Slug,
};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArtifactValidationSummary {
    pub(crate) article_count: usize,
    pub(crate) category_count: usize,
}

/// Validates that a generated site is complete enough for destructive deployment.
pub(crate) fn validate_site_artifacts(
    site_root: impl AsRef<Path>,
) -> Result<ArtifactValidationSummary> {
    let site_root = site_root.as_ref();
    let article_index: ArticleIndexDocument =
        read_required_json(site_root, Path::new("articles/index.json"))?;
    if article_index.articles.is_empty() {
        return Err(PublishError::ArtifactValidation(
            "articles/index.json must contain at least one article".to_string(),
        ));
    }

    let site_metadata: SiteMetadataDocument =
        read_required_json(site_root, Path::new("metadata/site.json"))?;
    if site_metadata.total_articles != article_index.articles.len() {
        return Err(PublishError::ArtifactValidation(format!(
            "metadata/site.json total_articles={} does not match articles/index.json count={}",
            site_metadata.total_articles,
            article_index.articles.len(),
        )));
    }

    let mut article_categories = HashSet::new();
    for article in &article_index.articles {
        let category = article.category.parse::<Category>().map_err(|error| {
            PublishError::ArtifactValidation(format!(
                "articles/index.json contains invalid category {}: {error}",
                article.category
            ))
        })?;
        let slug = Slug::new(article.slug.clone()).map_err(|error| {
            PublishError::ArtifactValidation(format!(
                "articles/index.json contains invalid slug {}: {error}",
                article.slug
            ))
        })?;
        let relative_path = PathBuf::from("articles")
            .join(category.as_str())
            .join(format!("{}.html", slug.as_str()));
        read_required_nonempty(site_root, &relative_path)?;
        article_categories.insert(category);
    }

    let metadata_category_names: HashSet<_> = site_metadata
        .categories
        .iter()
        .map(|category| category.category.as_str())
        .collect();
    let mut missing_article_categories: Vec<_> = article_categories
        .iter()
        .filter(|category| !metadata_category_names.contains(category.as_str()))
        .map(|category| category.as_str())
        .collect();
    if !missing_article_categories.is_empty() {
        missing_article_categories.sort_unstable();
        return Err(PublishError::ArtifactValidation(format!(
            "metadata/site.json is missing article categories: {}",
            missing_article_categories.join(", "),
        )));
    }

    for category_metadata in &site_metadata.categories {
        let category = category_metadata
            .category
            .parse::<Category>()
            .map_err(|error| {
                PublishError::ArtifactValidation(format!(
                    "metadata/site.json contains invalid category {}: {error}",
                    category_metadata.category
                ))
            })?;
        let category_path = PathBuf::from("categories").join(format!("{}.json", category.as_str()));
        let category_document: CategoryArtifactDocument =
            read_required_json(site_root, &category_path)?;
        if category_document.category != category.as_str() {
            return Err(PublishError::ArtifactValidation(format!(
                "{} declares category {} instead of {}",
                category_path.display(),
                category_document.category,
                category.as_str(),
            )));
        }
        if category_document.title.trim().is_empty() {
            return Err(PublishError::ArtifactValidation(format!(
                "required artifact {} contains empty title",
                category_path.display(),
            )));
        }
        if category_document.html.trim().is_empty() {
            return Err(PublishError::ArtifactValidation(format!(
                "required artifact {} contains empty html",
                category_path.display(),
            )));
        }

        let expected_articles: Vec<_> = article_index
            .articles
            .iter()
            .filter(|article| article.category == category.as_str())
            .cloned()
            .collect();
        if category_document.articles != expected_articles {
            return Err(PublishError::ArtifactValidation(format!(
                "{} does not match articles/index.json",
                category_path.display(),
            )));
        }
        if category_metadata.article_count != category_document.articles.len() {
            return Err(PublishError::ArtifactValidation(format!(
                "metadata count for {} is {}, but category artifact contains {} articles",
                category.as_str(),
                category_metadata.article_count,
                category_document.articles.len(),
            )));
        }
    }

    let about_path = Path::new("pages/about.json");
    let about: PageArtifactDocument = read_required_json(site_root, about_path)?;
    if about.page.as_str() != "about" {
        return Err(PublishError::ArtifactValidation(format!(
            "{} declares page {} instead of about",
            about_path.display(),
            about.page,
        )));
    }
    if about.html.trim().is_empty() {
        return Err(PublishError::ArtifactValidation(format!(
            "required artifact {} contains empty html",
            about_path.display(),
        )));
    }

    Ok(ArtifactValidationSummary {
        article_count: article_index.articles.len(),
        category_count: site_metadata.categories.len(),
    })
}

fn read_required_json<T: serde::de::DeserializeOwned>(
    site_root: &Path,
    relative_path: &Path,
) -> Result<T> {
    let contents = read_required_nonempty(site_root, relative_path)?;
    serde_json::from_str(&contents).map_err(|error| {
        PublishError::ArtifactValidation(format!(
            "{} is not valid artifact JSON: {error}",
            relative_path.display()
        ))
    })
}

fn read_required_nonempty(site_root: &Path, relative_path: &Path) -> Result<String> {
    let path = site_root.join(relative_path);
    let contents = fs::read_to_string(&path).map_err(|error| {
        PublishError::ArtifactValidation(format!(
            "required artifact {} cannot be read: {error}",
            relative_path.display()
        ))
    })?;
    if contents.trim().is_empty() {
        return Err(PublishError::ArtifactValidation(format!(
            "required artifact {} is empty",
            relative_path.display()
        )));
    }
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::super::builder::build_site_artifacts;
    use super::super::writer::{SiteDirectories, write_article_page, write_site_artifacts};
    use super::*;
    use domain::{
        ArticleMeta, CategoryLandingBody, CategoryLandingMeta, PageKey, PublishableCategoryLanding,
        SectionPath, Timestamp, Title,
    };
    use tempfile::TempDir;

    const ARTICLE_PATH: &str = "site/articles/tech/artifact00001.html";
    const CATEGORY_PATH: &str = "site/categories/tech.json";
    const ABOUT_PATH: &str = "site/pages/about.json";

    fn write_complete_site() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        let timestamp = Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap();
        let article = ArticleMeta {
            slug: Slug::new("artifact00001".to_string()).unwrap(),
            title: Title::new("Artifact Test".to_string()).unwrap(),
            category: Category::Tech,
            section_path: SectionPath::default(),
            description: Some("Artifact summary".to_string()),
            tags: vec!["rust".to_string()],
            priority: Some(1),
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };
        let landing = CategoryLandingMeta {
            category: Category::Tech,
            title: Title::new("Tech".to_string()).unwrap(),
            description: Some("Tech landing".to_string()),
            updated_at: timestamp,
        };
        let artifacts = build_site_artifacts(
            vec![article.clone()],
            vec![PublishableCategoryLanding::new(
                landing,
                CategoryLandingBody::new("<h1>Tech</h1>".to_string()).unwrap(),
            )],
            vec![PageArtifactDocument {
                page: PageKey::new("about".to_string()).unwrap(),
                title: "About".to_string(),
                description: Some("About this site".to_string()),
                html: "<article><h1>About</h1></article>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            }],
            None,
        )
        .unwrap();

        write_article_page(
            &directories,
            article.category,
            &article.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_artifacts(&directories, &artifacts).unwrap();

        temp_dir
    }

    #[test]
    fn test_validate_site_artifacts_accepts_complete_site() {
        let temp_dir = write_complete_site();

        let summary = validate_site_artifacts(temp_dir.path().join("site")).unwrap();

        assert_eq!(summary.article_count, 1);
        assert_eq!(summary.category_count, 1);
    }

    #[test]
    fn test_validate_site_artifacts_rejects_empty_article_index() {
        let temp_dir = TempDir::new().unwrap();
        let directories = SiteDirectories::prepare(temp_dir.path()).unwrap();
        write_site_artifacts(
            &directories,
            &build_site_artifacts(vec![], vec![], vec![], None).unwrap(),
        )
        .unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("at least one article"));
    }

    #[test]
    fn test_validate_site_artifacts_rejects_missing_article_html() {
        let temp_dir = write_complete_site();
        fs::remove_file(temp_dir.path().join(ARTICLE_PATH)).unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("articles/tech/artifact00001.html")
        );
    }

    #[test]
    fn test_validate_site_artifacts_rejects_missing_category_artifact() {
        let temp_dir = write_complete_site();
        fs::remove_file(temp_dir.path().join(CATEGORY_PATH)).unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("categories/tech.json"));
    }

    #[test]
    fn test_validate_site_artifacts_rejects_missing_about_page() {
        let temp_dir = write_complete_site();
        fs::remove_file(temp_dir.path().join(ABOUT_PATH)).unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("pages/about.json"));
    }

    #[test]
    fn test_validate_site_artifacts_rejects_article_category_missing_from_metadata() {
        let temp_dir = write_complete_site();
        fs::write(
            temp_dir.path().join("site/metadata/site.json"),
            serde_json::to_string_pretty(&SiteMetadataDocument {
                total_articles: 1,
                categories: vec![],
            })
            .unwrap(),
        )
        .unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("missing article categories"));
        assert!(error.to_string().contains("tech"));
    }
}
