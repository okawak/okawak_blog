use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, HeaderValue,
        error::{internal_server_error, not_found},
        header, path_param,
        response::Response,
        route,
    },
};
path_param!(asset_name);

#[route(GET)]
async fn content_asset(cx: &Cx) -> Result<Response> {
    let name = domain::ContentAssetName::new(path_param::<AssetName>(cx).to_string())
        .map_err(|_| not_found())?;
    let bytes = crate::app::page_loader(cx)
        .loader()
        .load_asset(&name)
        .await
        .map_err(|error| internal_server_error(std::io::Error::other(error)))?
        .ok_or_else(not_found)?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(name.media_type()),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}
