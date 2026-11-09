use blog_infra::DynArtifactSnapshot;
use topcoat::{
    Result,
    context::{Cx, context, try_context},
    router::{content::Json, error::internal_server_error, route},
};

use super::ArtifactReaderContext;

#[route(GET)]
async fn articles(cx: &Cx) -> Result<Json<blog_domain::ArticleIndexDocument>> {
    let artifact_reader = &context::<ArtifactReaderContext>(cx).0;
    let document = match try_context::<DynArtifactSnapshot>(cx) {
        Some(snapshot) => snapshot.read_article_index().await,
        None => {
            let snapshot = artifact_reader
                .snapshot()
                .await
                .map_err(internal_server_error)?;
            snapshot.read_article_index().await
        }
    }
    .map_err(internal_server_error)?;
    Ok(Json(document))
}
