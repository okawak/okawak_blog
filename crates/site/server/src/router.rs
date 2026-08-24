//! HTTP router composition for the site runtime.

use std::sync::Arc;

use infra::DynArtifactReader;
use topcoat::{
    asset::{AssetConfig, RouterBuilderAssetExt},
    context::{Cx, app_context},
    router::{
        Body, LayerFn, LayerFuture, Next, Path, Router, StatusCode, request, response::Response,
    },
};

use crate::{
    api::{ArtifactReaderContext, articles, health, readiness},
    http_cache::{ArtifactConditionalGetDecision, ArtifactHttpCacheState},
    page_loader::ArtifactPageLoader,
};
use web::{PageLoaderContext, topcoat_pages};

fn not_modified_response(conditional_get: &ArtifactConditionalGetDecision) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    conditional_get.insert_headers(response.headers_mut());
    response
}

fn artifact_conditional_get<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
    Box::pin(async move {
        let state = app_context::<ArtifactHttpCacheState>(cx);
        let Some(conditional_get) = state
            .conditional_get(request::method(cx), request::uri(cx), request::headers(cx))
            .await
        else {
            return next.run(cx, body).await;
        };

        if conditional_get.should_short_circuit() {
            return Ok(not_modified_response(&conditional_get));
        }

        let mut response = match conditional_get.snapshot() {
            Some(snapshot) => {
                let page_loader = PageLoaderContext(Arc::new(ArtifactPageLoader::from_snapshot(
                    snapshot.clone(),
                )));
                let cx = cx.with(snapshot).with(page_loader);
                next.run(&cx, body).await?
            }
            None => next.run(cx, body).await?,
        };
        if conditional_get.should_return_not_modified_after_response(response.status()) {
            return Ok(not_modified_response(&conditional_get));
        }
        if conditional_get.should_attach_validators(response.status()) {
            conditional_get.insert_headers(response.headers_mut());
        }
        Ok(response)
    })
}

pub fn create_router(
    artifact_reader: DynArtifactReader,
    validators_enabled: bool,
    assets: AssetConfig,
) -> Router {
    Router::builder()
        .route(health)
        .route(readiness)
        .route(articles)
        .route(topcoat_pages::home)
        .route(topcoat_pages::article_page)
        .route(topcoat_pages::category_page)
        .route(topcoat_pages::about)
        // The framework-neutral decision filters APIs, static assets, and unsuccessful responses.
        // One global layer also avoids nested prefix layers acquiring more than one snapshot.
        .layer(LayerFn::new(None::<&Path>, artifact_conditional_get))
        .app_context(ArtifactHttpCacheState::new(
            artifact_reader.clone(),
            validators_enabled,
        ))
        .app_context(PageLoaderContext(Arc::new(
            ArtifactPageLoader::from_reader(artifact_reader.clone()),
        )))
        .app_context(ArtifactReaderContext(artifact_reader))
        .assets(assets)
        .build()
}

#[cfg(test)]
#[path = "router_tests.rs"]
mod tests;
