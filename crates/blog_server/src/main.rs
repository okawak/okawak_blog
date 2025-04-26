#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use blog_server::app::App;
    use blog_server::app::*;
    use leptos::prelude::{provide_context, *};
    use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};

    // initialize the logger
    let env = env_logger::Env::default()
        .filter_or("RUST_LOG", "info")
        .write_style_or("LOG_STYLE", "auto");

    env_logger::init_from_env(env);
    log::info!("Starting server...");

    // AWS SDK設定
    // 認証情報は環境変数または~/.aws/credentialsから自動的に読み込まれます
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region("ap-northeast-1")
        .load()
        .await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

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
