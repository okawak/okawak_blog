use topcoat::{
    Result,
    context::{Cx, base_url},
    openapi::{Document, OpenApi},
    router::{route, router, segment},
};

segment!(rename: "openapi.json");

#[route(GET)]
async fn openapi(cx: &Cx) -> Result<Document> {
    Ok(OpenApi::new("okawak_blog API", env!("CARGO_PKG_VERSION"))
        .description("Published article index and service health endpoints.")
        .server(base_url(cx).as_str().to_owned())
        .generate(&router(cx)))
}
