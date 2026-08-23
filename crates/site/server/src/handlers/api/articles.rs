//! API handlers backed by generated site artifacts.

use axum::{Extension, Json, http::StatusCode};
use domain::ArticleIndexDocument;
use infra::{DynArtifactReader, DynArtifactSnapshot};

use crate::article_index::{read_article_index, read_article_index_from_snapshot};

pub async fn list_articles(
    Extension(artifact_reader): Extension<DynArtifactReader>,
    snapshot: Option<Extension<DynArtifactSnapshot>>,
) -> Result<Json<ArticleIndexDocument>, StatusCode> {
    let document = match snapshot {
        Some(Extension(snapshot)) => read_article_index_from_snapshot(&snapshot).await,
        None => read_article_index(&artifact_reader).await,
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(document))
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ArticleSummaryDocument, SectionPath};
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
                    section_path: SectionPath::new(vec!["block".to_string()]),
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

        let Json(document) = list_articles(
            Extension(Arc::new(LocalArtifactReader::new(temp_dir.path()))),
            None,
        )
        .await
        .unwrap();

        assert_eq!(document.articles.len(), 1);
        assert_eq!(document.articles[0].slug, "sample0000001");
    }
}
