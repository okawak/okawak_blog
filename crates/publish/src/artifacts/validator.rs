use crate::error::{PublishError, Result};
use domain::{
    ArticleIndexDocument, Category, CategoryIndexDocument, PageArtifactDocument,
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
        let category_root = PathBuf::from("categories").join(category.as_str());
        let category_index_path = category_root.join("index.json");
        let category_index: CategoryIndexDocument =
            read_required_json(site_root, &category_index_path)?;
        if category_index.category != category.as_str() {
            return Err(PublishError::ArtifactValidation(format!(
                "{} declares category {} instead of {}",
                category_index_path.display(),
                category_index.category,
                category.as_str(),
            )));
        }

        let expected_articles: Vec<_> = article_index
            .articles
            .iter()
            .filter(|article| article.category == category.as_str())
            .cloned()
            .collect();
        if category_index.articles != expected_articles {
            return Err(PublishError::ArtifactValidation(format!(
                "{} does not match articles/index.json",
                category_index_path.display(),
            )));
        }
        if category_metadata.article_count != category_index.articles.len() {
            return Err(PublishError::ArtifactValidation(format!(
                "metadata count for {} is {}, but category index contains {} articles",
                category.as_str(),
                category_metadata.article_count,
                category_index.articles.len(),
            )));
        }

        read_required_nonempty(site_root, &category_root.join("page.html"))?;
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
