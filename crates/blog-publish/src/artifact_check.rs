//! CLI validation of completed artifacts using the same blog-domain builders as SSR.
use anyhow::{Context, Result, ensure};
use blog_domain::{
    ArticleArtifactDocument, ArticleIndexDocument, CategoryArtifactDocument,
    HomeFragmentArtifactDocument, PageArtifactDocument, SiteLocalesDocument, SiteMetadataDocument,
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
        let directory = root.join(locale.as_str());
        let index: ArticleIndexDocument = read_json(&root.join(locale.data_key("articles.json")))?;
        let metadata: SiteMetadataDocument = read_json(&root.join(locale.data_key("site.json")))?;
        let home: Option<HomeFragmentArtifactDocument> =
            match fs::read(root.join(locale.page_file("/", "json"))) {
                Ok(bytes) => Some(serde_json::from_slice(&bytes)?),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
        build_home_page_document(&index, &metadata, home.as_ref())?;
        for article in &index.articles {
            let category = article.category.parse()?;
            let slug = article.slug.parse()?;
            let route = blog_domain::build_article_path(&category, &slug);
            let path = root.join(locale.page_file(&route, "json"));
            let document: ArticleArtifactDocument = read_json(&path)?;
            ensure!(
                document.summary == *article,
                "article summary differs from index: {}",
                path.display()
            );
            document.into_article(&category, &slug)?;
        }
        for path in json_files(&directory)? {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("invalid page filename")?;
            match name {
                "index" => {}
                "about" => {
                    let page: PageArtifactDocument = read_json(&path)?;
                    build_static_page_document(&page)
                        .with_context(|| path.display().to_string())?;
                    ensure!(page.page.as_str() == name, "page filename mismatch");
                }
                _ => {
                    let category: CategoryArtifactDocument = read_json(&path)?;
                    build_category_page_document(&category)
                        .with_context(|| path.display().to_string())?;
                    ensure!(category.category == name, "category filename mismatch");
                }
            }
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
