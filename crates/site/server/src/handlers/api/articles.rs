//! API handlers backed by generated site artifacts.

use axum::{Json, http::StatusCode};
use domain::ArticleIndexDocument;
use infra::LocalArtifactReader;
use std::sync::Arc;

pub async fn list_articles(
    artifact_reader: Arc<LocalArtifactReader>,
) -> Result<Json<ArticleIndexDocument>, StatusCode> {
    let document = artifact_reader
        .read_article_index()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(document))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::ArticleSummaryDocument;
    use infra::LocalArtifactReader;
    use std::{fs, sync::Arc};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_list_articles_reads_generated_index() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("articles")).unwrap();
        fs::write(
            temp_dir.path().join("articles/index.json"),
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

        let Json(document) = list_articles(Arc::new(LocalArtifactReader::new(temp_dir.path())))
            .await
            .unwrap();

        assert_eq!(document.articles.len(), 1);
        assert_eq!(document.articles[0].slug, "sample0000001");
    }
}
