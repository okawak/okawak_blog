//! Blog Server Main - Leptos SSR統合サーバー

use axum::{Router, routing::get};
use infra::LocalArtifactReader;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};
use server::handlers::create_api_router;
use std::{path::PathBuf, sync::Arc};
use tower_http::services::ServeDir;
use web::app::{App, shell};

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Leptos設定取得
    let conf = get_configuration(Some("crates/site/server/Cargo.toml")).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let artifact_reader = Arc::new(LocalArtifactReader::new(PathBuf::from(
        "crates/publish/publisher/dist/site",
    )));

    println!("Starting Leptos blog server on http://{}", addr);
    println!("Leptos設定読み込み完了: {:?}", addr);

    // Leptosルート生成
    let routes = generate_route_list(App);

    // 統合Axumアプリケーション作成
    let app = Router::new()
        // API routes
        .nest("/api", create_api_router(artifact_reader))
        .route("/api/health", get(health))
        // 静的ファイル配信
        .nest_service(
            "/pkg",
            axum::routing::get_service(ServeDir::new("target/site/pkg")),
        )
        .nest_service(
            "/assets",
            axum::routing::get_service(ServeDir::new("target/site/assets")),
        )
        // Leptos SSRルート（最後に配置）
        .leptos_routes(&leptos_options, routes, {
            let opts = leptos_options.clone();
            move || shell(opts.clone())
        })
        // フォールバック
        .fallback(file_and_error_handler(shell))
        .with_state(leptos_options);

    println!("Server listening on http://{}", &addr);
    println!("Visit http://{} to see the Leptos app", &addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
