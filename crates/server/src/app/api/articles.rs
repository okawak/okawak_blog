use infra::DynArtifactSnapshot;
use topcoat::{
    Result,
    context::{Cx, app_context, try_request_context},
    router::{content::Json, error::internal_server_error, route},
};

use super::ArtifactReaderContext;

#[route(GET)]
async fn articles(cx: &Cx) -> Result<Json<domain::ArticleIndexDocument>> {
    let artifact_reader = &app_context::<ArtifactReaderContext>(cx).0;
    let document = match try_request_context::<DynArtifactSnapshot>(cx) {
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
