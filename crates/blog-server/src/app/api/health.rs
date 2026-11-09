use topcoat::{Result, context::Cx, router::route};

#[route(GET)]
async fn health(cx: &Cx) -> Result<&'static str> {
    Ok("OK")
}
