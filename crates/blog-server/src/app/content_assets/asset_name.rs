use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{RouterErrorExt, internal_server_error},
        header, module_param,
        response::IntoResponse,
        route,
    },
};
module_param!(asset_name);

#[route(GET)]
async fn content_asset(cx: &Cx) -> Result<impl IntoResponse> {
    let name = blog_domain::ContentAssetName::new(asset_name(cx).to_string()).ok_or_not_found()?;
    let bytes = crate::app::page_loader(cx)
        .load_asset(&name)
        .await
        .map_err(|error| internal_server_error(std::io::Error::other(error)))?
        .ok_or_not_found()?;
    Ok((
        [
            (header::CONTENT_TYPE, name.media_type()),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        bytes,
    ))
}
