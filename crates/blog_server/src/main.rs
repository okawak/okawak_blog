#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use blog_server::app::App;
    use blog_server::app::*;
    use blog_server::setup_logging;
    use leptos::logging::log;
    use leptos::prelude::{provide_context, *};
    use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};

    // initialize the logger
    setup_logging();
    log::info!("Starting server...");

    // AWS SDK設定
    // 認証情報は環境変数または~/.aws/credentialsから自動的に読み込まれます
    let aws_config = aws_config::from_env().load().await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);
    let s3_client_ctx = s3_client.clone();

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options.clone();

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    // Axumルーターを設定
    let app = Router::new()
        // SSR 用コンテキストと App シェルを渡す
        .leptos_routes_with_context(
            &leptos_options,
            routes.clone(),
            // additional_context: リクエスト処理前に呼ばれる
            {
                let s3_client = s3_client.clone();
                move || {
                    provide_context(s3_client.clone());
                }
            },
            // app_fn: HTML ドキュメント全体を生成するシェル
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        // 静的ファイル＋404 用ハンドラ（shell 関数だけ渡せば OK）
        .fallback(file_and_error_handler(shell))
        .with_state(leptos_options.clone());

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    log::info!("listening on http://{}", &addr);

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
