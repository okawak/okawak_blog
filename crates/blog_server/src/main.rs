#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use blog_server::app::App;
    use blog_server::app::*;
    use chrono::format;
    use leptos::prelude::{provide_context, *};
    use leptos::{logging::log, server_fn::response};
    use leptos_axum::{LeptosRoutes, generate_route_list};

    // initialize the logger
    setup_logging();
    log::info!("Starting server...");

    // AWS SDK設定
    // 認証情報は環境変数または~/.aws/credentialsから自動的に読み込まれます
    let aws_config = aws_config::from_env().load().await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    // アプリケーションの状態としてS3クライアントを保存するための準備
    let s3_resource = leptos::create_server_resource(move || s3_client.clone());

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Axumルーターを設定
    let app = Router::new()
        .leptos_routes_with_handler(
            &leptos_options,
            routes,
            {
                let leptos_options = leptos_options.clone();
                move || {
                    provide_context(s3_resource.clone());
                    shell(leptos_options.clone())
                }
            },
            |errors| {
                log::error!("Error: {:?}", errors);
                let mut response =
                    axum::response::Response::new(format!("Error: {:?}", errors).into());
                *response.status_mut() = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
                response
            },
        )
        .fallback(leptos_axum::file_and_error_handler(move || {
            provide_context(s3_resource.clone());
            shell(leptos_options.clone())
        }))
        .with_state(leptos_options);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log::info!("listening on http://{}", &addr);
    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
