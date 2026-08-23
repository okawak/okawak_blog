//! Framework-neutral article index access shared by server runtimes.

use domain::ArticleIndexDocument;
use infra::{DynArtifactReader, DynArtifactSnapshot, Result};

pub async fn read_article_index_from_snapshot(
    snapshot: &DynArtifactSnapshot,
) -> Result<ArticleIndexDocument> {
    snapshot.read_article_index().await
}

pub async fn read_article_index(
    artifact_reader: &DynArtifactReader,
) -> Result<ArticleIndexDocument> {
    let snapshot = artifact_reader.snapshot().await?;
    read_article_index_from_snapshot(&snapshot).await
}
