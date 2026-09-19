//! CLI validation of completed artifacts using the same domain builders as SSR.
use anyhow::{Context, Result, ensure};
use domain::{
    ArticleIndexDocument, CategoryArtifactDocument, HomeFragmentArtifactDocument,
    PageArtifactDocument, SiteLocalesDocument, SiteMetadataDocument, build_article_page_document,
    build_category_page_document, build_home_page_document, build_static_page_document,
};
use serde::de::DeserializeOwned;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn validate(root: &Path) -> Result<()> {
    let locales: SiteLocalesDocument = read_json(&root.join("locales.json"))?;
    locales.validate()?;
    let available = locales.routes.get("/").context("home locales missing")?;
    for locale in available {
        let root = root.join(locale.artifact_key(""));
        let index: ArticleIndexDocument = read_json(&root.join("articles/index.json"))?;
        let metadata: SiteMetadataDocument = read_json(&root.join("metadata/site.json"))?;
        let home: Option<HomeFragmentArtifactDocument> = match fs::read(root.join("home.json")) {
            Ok(bytes) => Some(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        build_home_page_document(&index, &metadata, home.as_ref())?;
        for article in &index.articles {
            let path = root.join(format!(
                "articles/{}/{}.html",
                article.category, article.slug
            ));
            let html = fs::read_to_string(&path).with_context(|| path.display().to_string())?;
            build_article_page_document(article, &html)?;
        }
        for path in json_files(&root.join("categories"))? {
            let category: CategoryArtifactDocument = read_json(&path)?;
            build_category_page_document(&category).with_context(|| path.display().to_string())?;
            ensure!(
                path.file_stem().and_then(|name| name.to_str()) == Some(category.category.as_str()),
                "category filename mismatch"
            );
        }
        for path in json_files(&root.join("pages"))? {
            let page: PageArtifactDocument = read_json(&path)?;
            build_static_page_document(&page).with_context(|| path.display().to_string())?;
            ensure!(
                path.file_stem().and_then(|name| name.to_str()) == Some(page.page.as_str()),
                "page filename mismatch"
            );
        }
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path)?).with_context(|| path.display().to_string())
}

fn json_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.path().extension().is_some_and(|ext| ext == "json") {
            ensure!(
                entry.file_type()?.is_file(),
                "artifact must be a regular file"
            );
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}
