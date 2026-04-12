//! API handlers backed by page contracts derived from generated artifacts.

use axum::{Extension, Json, extract::Path, http::StatusCode};
use domain::{
    ArticlePageDocument, Category, CategoryPageDocument, HomePageDocument, PageKey, Slug,
    StaticPageDocument, build_article_page_document, build_category_page_document,
    build_home_page_document, build_static_page_document, find_article_summary,
};
use infra::DynArtifactReader;
use std::str::FromStr;

pub async fn get_home_page(
    Extension(artifact_reader): Extension<DynArtifactReader>,
) -> Result<Json<HomePageDocument>, StatusCode> {
    let article_index = artifact_reader
        .read_article_index()
        .await
        .map_err(map_infra_error)?;
    let site_metadata = artifact_reader
        .read_site_metadata()
        .await
        .map_err(map_infra_error)?;
    let page = build_home_page_document(&article_index, &site_metadata)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(page))
}

pub async fn get_article_page(
    Path((category, slug)): Path<(String, String)>,
    Extension(artifact_reader): Extension<DynArtifactReader>,
) -> Result<Json<ArticlePageDocument>, StatusCode> {
    let category = Category::from_str(&category).map_err(|_| StatusCode::NOT_FOUND)?;
    let slug = Slug::new(slug).map_err(|_| StatusCode::NOT_FOUND)?;
    let article_index = artifact_reader
        .read_article_index()
        .await
        .map_err(map_infra_error)?;
    let summary =
        find_article_summary(&article_index, &category, &slug).ok_or(StatusCode::NOT_FOUND)?;
    let html = artifact_reader
        .read_article_html(&category, &slug)
        .await
        .map_err(map_infra_error)?;
    let page = build_article_page_document(summary, &html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(page))
}

pub async fn get_category_page(
    Path(category): Path<String>,
    Extension(artifact_reader): Extension<DynArtifactReader>,
) -> Result<Json<CategoryPageDocument>, StatusCode> {
    let category = Category::from_str(&category).map_err(|_| StatusCode::NOT_FOUND)?;
    let category_index = artifact_reader
        .read_category_index(category.as_str())
        .await
        .map_err(map_infra_error)?;
    let html = artifact_reader
        .read_category_html(&category)
        .await
        .map_err(map_infra_error)?;
    let page = build_category_page_document(&category_index, &html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(page))
}

pub async fn get_static_page(
    Path(page): Path<String>,
    Extension(artifact_reader): Extension<DynArtifactReader>,
) -> Result<Json<StaticPageDocument>, StatusCode> {
    let page = PageKey::new(page).map_err(|_| StatusCode::NOT_FOUND)?;
    let artifact = artifact_reader
        .read_page_document(&page)
        .await
        .map_err(map_infra_error)?;
    let document =
        build_static_page_document(&artifact).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(document))
}

fn map_infra_error(error: infra::InfraError) -> StatusCode {
    if error.is_not_found() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::write_fixture_site;
    use infra::LocalArtifactReader;
    use std::{fs, sync::Arc};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_get_home_page_reads_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let Json(page) = get_home_page(Extension(Arc::new(LocalArtifactReader::new(
            temp_dir.path(),
        ))))
        .await
        .unwrap();

        assert_eq!(page.total_articles, 1);
        assert_eq!(page.articles.len(), 1);
        assert_eq!(page.categories[0].category_display_name, "技術");
    }

    #[tokio::test]
    async fn test_get_article_page_reads_html_and_summary() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let Json(page) = get_article_page(
            Path(("tech".to_string(), "sample0000001".to_string())),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await
        .unwrap();

        assert_eq!(page.article.title.as_str(), "Sample");
        assert!(page.html.contains("<h1>Sample</h1>"));
    }

    #[tokio::test]
    async fn test_get_category_page_reads_category_index() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let Json(page) = get_category_page(
            Path("tech".to_string()),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await
        .unwrap();

        assert_eq!(page.category_display_name, "技術");
        assert_eq!(page.title, "Tech");
        assert!(page.html.contains("Category landing"));
        assert_eq!(page.articles.len(), 1);
        assert_eq!(page.sections.len(), 1);
    }

    #[tokio::test]
    async fn test_get_category_page_rejects_invalid_category() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let result = get_category_page(
            Path("../secrets".to_string()),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await;

        assert!(matches!(result, Err(StatusCode::NOT_FOUND)));
    }

    #[tokio::test]
    async fn test_get_article_page_returns_not_found_when_html_artifact_is_missing() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        fs::remove_file(temp_dir.path().join("articles/tech/sample0000001.html")).unwrap();

        let result = get_article_page(
            Path(("tech".to_string(), "sample0000001".to_string())),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await;

        assert!(matches!(result, Err(StatusCode::NOT_FOUND)));
    }

    #[tokio::test]
    async fn test_get_category_page_returns_not_found_when_category_artifact_is_missing() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        fs::remove_file(temp_dir.path().join("categories/tech/index.json")).unwrap();

        let result = get_category_page(
            Path("tech".to_string()),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await;

        assert!(matches!(result, Err(StatusCode::NOT_FOUND)));
    }

    #[tokio::test]
    async fn test_get_category_page_returns_not_found_when_category_page_html_is_missing() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        fs::remove_file(temp_dir.path().join("categories/tech/page.html")).unwrap();

        let result = get_category_page(
            Path("tech".to_string()),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await;

        assert!(matches!(result, Err(StatusCode::NOT_FOUND)));
    }

    #[tokio::test]
    async fn test_get_static_page_reads_page_artifact() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());

        let Json(page) = get_static_page(
            Path("about".to_string()),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await
        .unwrap();

        assert_eq!(page.page.as_str(), "about");
        assert_eq!(page.title, "About");
        assert!(page.html.contains("<h1>About</h1>"));
    }

    #[tokio::test]
    async fn test_get_static_page_returns_not_found_when_page_artifact_is_missing() {
        let temp_dir = TempDir::new().unwrap();
        write_fixture_site(temp_dir.path());
        fs::remove_file(temp_dir.path().join("pages/about.json")).unwrap();

        let result = get_static_page(
            Path("about".to_string()),
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
        )
        .await;

        assert!(matches!(result, Err(StatusCode::NOT_FOUND)));
    }
}
