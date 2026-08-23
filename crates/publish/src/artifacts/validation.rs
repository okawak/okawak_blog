use crate::error::{PublishError, Result};
use domain::{
    ArticleIndexDocument, Category, CategoryArtifactDocument, HomeFragmentArtifactDocument,
    PageArtifactDocument, PublishedArticleSummary, SiteMetadata, SiteMetadataDocument,
};
use std::{
    collections::HashSet,
    fs::{self, File},
    path::{Path, PathBuf},
};

/// Validates that a generated site is complete enough for destructive deployment.
pub(crate) fn validate_site_artifacts(site_root: impl AsRef<Path>) -> Result<()> {
    let site_root = site_root.as_ref();
    let article_index_path = Path::new("articles/index.json");
    let article_index: ArticleIndexDocument = read_json(site_root, article_index_path)?;
    if article_index.articles.is_empty() {
        return Err(PublishError::ArtifactValidation(
            "articles/index.json must contain at least one article".to_string(),
        ));
    }

    let site_metadata_path = Path::new("metadata/site.json");
    let site_metadata_document: SiteMetadataDocument = read_json(site_root, site_metadata_path)?;
    let site_metadata = SiteMetadata::try_from(&site_metadata_document)
        .map_err(|error| invalid_document(site_metadata_path, error))?;
    if site_metadata.total_articles != article_index.articles.len() {
        return Err(PublishError::ArtifactValidation(format!(
            "metadata/site.json total_articles={} does not match articles/index.json count={}",
            site_metadata.total_articles,
            article_index.articles.len(),
        )));
    }

    let article_categories = validate_articles(site_root, article_index_path, &article_index)?;
    validate_categories(
        site_root,
        &article_index,
        &site_metadata,
        &article_categories,
    )?;
    validate_about(site_root)?;
    validate_home(site_root)?;

    Ok(())
}

fn validate_articles(
    site_root: &Path,
    index_path: &Path,
    article_index: &ArticleIndexDocument,
) -> Result<HashSet<Category>> {
    let mut categories = HashSet::new();
    for document in &article_index.articles {
        let article = PublishedArticleSummary::try_from(document)
            .map_err(|error| invalid_document(index_path, error))?;
        let relative_path = PathBuf::from("articles")
            .join(article.category.as_str())
            .join(format!("{}.html", article.slug.as_str()));
        ensure_nonempty_file(site_root, &relative_path)?;
        categories.insert(article.category);
    }
    Ok(categories)
}

fn validate_categories(
    site_root: &Path,
    article_index: &ArticleIndexDocument,
    site_metadata: &SiteMetadata,
    article_categories: &HashSet<Category>,
) -> Result<()> {
    for category in article_categories {
        if !site_metadata
            .categories
            .iter()
            .any(|metadata| metadata.category == *category)
        {
            return Err(PublishError::ArtifactValidation(format!(
                "metadata/site.json is missing article category {}",
                category.as_str(),
            )));
        }
    }

    for category_metadata in &site_metadata.categories {
        let category = category_metadata.category;
        let category_path = PathBuf::from("categories").join(format!("{}.json", category.as_str()));
        let category_document: CategoryArtifactDocument = read_json(site_root, &category_path)?;
        category_document
            .validate()
            .map_err(|error| invalid_document(&category_path, error))?;
        if category_document.category != category.as_str() {
            return Err(PublishError::ArtifactValidation(format!(
                "{} declares category {} instead of {}",
                category_path.display(),
                category_document.category,
                category.as_str(),
            )));
        }
        if !category_document.articles.iter().eq(article_index
            .articles
            .iter()
            .filter(|article| article.category == category.as_str()))
        {
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

    Ok(())
}

fn validate_about(site_root: &Path) -> Result<()> {
    let about_path = Path::new("pages/about.json");
    let about: PageArtifactDocument = read_json(site_root, about_path)?;
    about
        .validate()
        .map_err(|error| invalid_document(about_path, error))?;
    if about.page.as_str() != "about" {
        return Err(PublishError::ArtifactValidation(format!(
            "{} declares page {} instead of about",
            about_path.display(),
            about.page,
        )));
    }
    Ok(())
}

fn validate_home(site_root: &Path) -> Result<()> {
    let home_path = Path::new("home.json");
    if !site_root.join(home_path).try_exists()? {
        return Ok(());
    }
    let home: HomeFragmentArtifactDocument = read_json(site_root, home_path)?;
    home.validate()
        .map_err(|error| invalid_document(home_path, error))
}

fn read_json<T: serde::de::DeserializeOwned>(site_root: &Path, relative_path: &Path) -> Result<T> {
    let file = File::open(site_root.join(relative_path)).map_err(|error| {
        PublishError::ArtifactValidation(format!(
            "required artifact {} cannot be read: {error}",
            relative_path.display()
        ))
    })?;
    serde_json::from_reader(file).map_err(|error| {
        PublishError::ArtifactValidation(format!(
            "{} is not valid artifact JSON: {error}",
            relative_path.display()
        ))
    })
}

fn ensure_nonempty_file(site_root: &Path, relative_path: &Path) -> Result<()> {
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
    Ok(())
}

fn invalid_document(relative_path: &Path, error: domain::DomainError) -> PublishError {
    PublishError::ArtifactValidation(format!(
        "artifact {} is invalid: {error}",
        relative_path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::super::builder::build_site_documents;
    use super::super::writer::{SiteOutput, write_article_page, write_site_documents};
    use super::*;
    use domain::{
        ArticleMeta, CategoryLandingBody, CategoryLandingMeta, PageKey, PublishableCategoryLanding,
        SectionPath, Slug, Timestamp, Title,
    };
    use rstest::rstest;
    use tempfile::TempDir;

    fn write_complete_site() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let output = SiteOutput::prepare(temp_dir.path()).unwrap();
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
        let documents = build_site_documents(
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
            &output,
            article.category,
            &article.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_documents(&output, &documents).unwrap();

        temp_dir
    }

    #[test]
    fn test_validate_site_artifacts_accepts_complete_site() {
        let temp_dir = write_complete_site();

        validate_site_artifacts(temp_dir.path().join("site")).unwrap();
    }

    #[test]
    fn test_validate_site_artifacts_rejects_empty_article_index() {
        let temp_dir = TempDir::new().unwrap();
        let output = SiteOutput::prepare(temp_dir.path()).unwrap();
        write_site_documents(
            &output,
            &build_site_documents(vec![], vec![], vec![], None).unwrap(),
        )
        .unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("at least one article"));
    }

    #[rstest]
    #[case("articles/tech/artifact00001.html")]
    #[case("categories/tech.json")]
    #[case("pages/about.json")]
    fn test_validate_site_artifacts_rejects_missing_required_artifact(#[case] relative_path: &str) {
        let temp_dir = write_complete_site();
        fs::remove_file(temp_dir.path().join("site").join(relative_path)).unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains(relative_path));
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

        assert!(error.to_string().contains("missing article category"));
        assert!(error.to_string().contains("tech"));
    }

    #[test]
    fn test_validate_site_artifacts_rejects_invalid_home_fragment() {
        let temp_dir = write_complete_site();
        fs::write(
            temp_dir.path().join("site/home.json"),
            serde_json::to_vec(&HomeFragmentArtifactDocument {
                title: String::new(),
                description: None,
                html: "<p>Home</p>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            })
            .unwrap(),
        )
        .unwrap();

        let error = validate_site_artifacts(temp_dir.path().join("site")).unwrap_err();

        assert!(error.to_string().contains("home.json"));
    }
}
