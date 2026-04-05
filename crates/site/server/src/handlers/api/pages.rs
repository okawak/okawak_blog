//! API handlers backed by page contracts derived from generated artifacts.

use axum::{Extension, Json, extract::Path, http::StatusCode};
use domain::{
    ArticlePageDocument, CategoryPageDocument, HomePageDocument, Slug, build_article_page_document,
    build_category_page_document, build_home_page_document, find_article_summary,
};
use infra::DynArtifactReader;

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
    Path(slug): Path<String>,
    Extension(artifact_reader): Extension<DynArtifactReader>,
) -> Result<Json<ArticlePageDocument>, StatusCode> {
    let slug = Slug::new(slug).map_err(|_| StatusCode::NOT_FOUND)?;
    let article_index = artifact_reader
        .read_article_index()
        .await
        .map_err(map_infra_error)?;
    let summary = find_article_summary(&article_index, &slug).ok_or(StatusCode::NOT_FOUND)?;
    let html = artifact_reader
        .read_article_html(&slug)
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
    let category_index = artifact_reader
        .read_category_index(&category)
        .await
        .map_err(map_infra_error)?;
    let page = build_category_page_document(&category_index)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(page))
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
    use domain::{
        ArticleIndexDocument, ArticleSummaryDocument, CategoryMetadataDocument,
        SiteMetadataDocument,
    };
    use infra::LocalArtifactReader;
    use std::{fs, sync::Arc};
    use tempfile::TempDir;

    fn write_fixture_site(root: &std::path::Path) {
        fs::create_dir_all(root.join("articles")).unwrap();
        fs::create_dir_all(root.join("categories")).unwrap();
        fs::create_dir_all(root.join("metadata")).unwrap();

        fs::write(
            root.join("articles/index.json"),
            serde_json::to_string_pretty(&ArticleIndexDocument {
                articles: vec![ArticleSummaryDocument {
                    slug: "sample0000001".to_string(),
                    title: "Sample".to_string(),
                    category: "tech".to_string(),
                    description: Some("summary".to_string()),
                    tags: vec!["rust".to_string()],
                    priority: Some(1),
                    created_at: "2025-01-01T00:00:00+09:00".to_string(),
                    updated_at: "2025-01-01T00:00:00+09:00".to_string(),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("categories/tech.json"),
            serde_json::to_string_pretty(&domain::CategoryIndexDocument {
                category: "tech".to_string(),
                articles: vec![ArticleSummaryDocument {
                    slug: "sample0000001".to_string(),
                    title: "Sample".to_string(),
                    category: "tech".to_string(),
                    description: Some("summary".to_string()),
                    tags: vec!["rust".to_string()],
                    priority: Some(1),
                    created_at: "2025-01-01T00:00:00+09:00".to_string(),
                    updated_at: "2025-01-01T00:00:00+09:00".to_string(),
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("metadata/site.json"),
            serde_json::to_string_pretty(&SiteMetadataDocument {
                total_articles: 1,
                categories: vec![CategoryMetadataDocument {
                    category: "tech".to_string(),
                    article_count: 1,
                }],
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("articles/sample0000001.html"),
            "<article><h1>Sample</h1></article>",
        )
        .unwrap();
    }

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
            Path("sample0000001".to_string()),
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
        assert_eq!(page.articles.len(), 1);
    }
}
