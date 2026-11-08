//! CLI validation of completed artifacts using the same blog-domain builders as SSR.
use anyhow::{Context, Result, ensure};
use blog_domain::{
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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_site() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("articles")).unwrap();
        fs::create_dir(root.path().join("metadata")).unwrap();
        fs::write(
            root.path().join("locales.json"),
            r#"{"schema_version":1,"routes":{"/":["ja"]}}"#,
        )
        .unwrap();
        fs::write(
            root.path().join("articles/index.json"),
            r#"{"articles":[]}"#,
        )
        .unwrap();
        fs::write(
            root.path().join("metadata/site.json"),
            r#"{"total_articles":0,"categories":[]}"#,
        )
        .unwrap();
        root
    }

    #[test]
    fn optional_home_may_be_absent_but_must_be_readable_and_valid_when_present() {
        let root = minimal_site();
        validate(root.path()).unwrap();
        let home = root.path().join("home.json");
        fs::write(
            &home,
            r#"{"title":"Home","html":"<p>Welcome</p>","updated_at":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();
        validate(root.path()).unwrap();
        fs::write(&home, "invalid").unwrap();
        assert!(validate(root.path()).is_err());
        fs::remove_file(&home).unwrap();
        fs::create_dir(&home).unwrap();
        assert!(validate(root.path()).is_err());
    }

    #[test]
    fn category_and_page_artifacts_are_validated_and_checked_against_their_filenames() {
        for (directory, filename, document) in [
            (
                "categories",
                "tech",
                r#"{"category":"tech","title":"Tech","html":"<p>Tech</p>","updated_at":"2026-01-01T00:00:00Z","articles":[]}"#,
            ),
            (
                "pages",
                "about",
                r#"{"page":"about","title":"About","html":"<p>About</p>","updated_at":"2026-01-01T00:00:00Z"}"#,
            ),
        ] {
            let root = minimal_site();
            let directory = root.path().join(directory);
            fs::create_dir(&directory).unwrap();
            let path = directory.join(format!("{filename}.json"));
            fs::write(&path, document).unwrap();
            fs::write(directory.join("ignored.txt"), "not json").unwrap();
            validate(root.path()).unwrap();
            fs::rename(&path, directory.join("wrong.json")).unwrap();
            assert!(validate(root.path()).is_err());
            fs::rename(directory.join("wrong.json"), &path).unwrap();
            fs::write(&path, "invalid").unwrap();
            assert!(validate(root.path()).is_err());
        }
    }

    #[test]
    fn json_scan_sorts_files_and_distinguishes_missing_directories_from_io_errors() {
        let root = tempfile::tempdir().unwrap();
        assert!(json_files(&root.path().join("missing")).unwrap().is_empty());
        for name in ["b.json", "a.json", "ignored.txt"] {
            fs::write(root.path().join(name), "{}").unwrap();
        }
        assert_eq!(
            json_files(root.path()).unwrap(),
            [root.path().join("a.json"), root.path().join("b.json")]
        );
        assert!(json_files(&root.path().join("a.json")).is_err());
        fs::create_dir(root.path().join("directory.json")).unwrap();
        assert!(json_files(root.path()).is_err());
    }
}
