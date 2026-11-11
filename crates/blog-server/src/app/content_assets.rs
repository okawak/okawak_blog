mod asset_name;

use topcoat::{
    Result,
    context::Cx,
    router::{error::not_found, response::Response, route},
};

// Reserve the namespace root so it cannot fall through to the category page.
#[route(GET)]
async fn content_assets_root(cx: &Cx) -> Result<Response> {
    Err(not_found().into())
}
