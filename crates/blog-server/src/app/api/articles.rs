use blog_infra::DynArtifactSnapshot;
use topcoat::{
    Result,
    context::{Cx, context, try_context},
    openapi::{Operation, Parameter},
    router::{StatusCode, content::Json, error::internal_server_error, route},
};

use blog_infra::DynArtifactReader;

#[route(
    GET,
    with(Operation::new()
        .operation_id("articles")
        .summary("Read the published Japanese article index")
        .description(
            "Compatibility endpoint. Release-aware ETag and Last-Modified validators are available when the configured S3 snapshot cache supplies a stable release identity.",
        )
        .tag("content")
        .parameter(
            Parameter::header("If-None-Match").description(
                "Optional ETag validator. Takes precedence over If-Modified-Since.",
            ),
        )
        .parameter(
            Parameter::header("If-Modified-Since").description(
                "Optional HTTP date validator, used only when If-None-Match is absent.",
            ),
        )
        .response_with::<Json<blog_domain::ArticleIndexDocument>>(
            StatusCode::OK,
            "The published article index.",
        )
        .response(
            StatusCode::NOT_MODIFIED,
            "The cached representation is still current.",
        )
        .response_with::<String>(
            StatusCode::INTERNAL_SERVER_ERROR,
            "The article index could not be read.",
        ))
)]
async fn articles(cx: &Cx) -> Result<Json<blog_domain::ArticleIndexDocument>> {
    let artifact_reader = context::<DynArtifactReader>(cx);
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
