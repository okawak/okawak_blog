use topcoat::{
    Result,
    context::Cx,
    openapi::Operation,
    router::{StatusCode, route},
};

#[route(
    GET,
    with(Operation::new()
        .operation_id("health")
        .summary("Check process liveness")
        .description(
            "Does not read artifacts. Returns OK while the process can serve requests.",
        )
        .tag("health")
        .response_with::<&str>(StatusCode::OK, "OK"))
)]
async fn health(cx: &Cx) -> Result<&'static str> {
    Ok("OK")
}
