//! Framework-neutral article index access shared by server runtimes.

use domain::ArticleIndexDocument;
use infra::{DynArtifactReader, Result};

pub async fn read_article_index(
    artifact_reader: &DynArtifactReader,
) -> Result<ArticleIndexDocument> {
    let snapshot = artifact_reader.snapshot().await?;
    snapshot.read_article_index().await
}
