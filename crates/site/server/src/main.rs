//! Main entry point for the integrated Leptos SSR server.

use axum::{Router, routing::get};
use infra::{ArtifactSourceConfig, build_artifact_reader};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};
use server::handlers::create_api_router;
use tower_http::services::ServeDir;
use web::app::{App, shell};

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the Leptos configuration.
    let conf = get_configuration(Some("crates/site/server/Cargo.toml")).unwrap();
    let leptos_options = conf.leptos_options.clone();
    let addr = leptos_options.site_addr;
    let artifact_source = ArtifactSourceConfig::from_env()?;
    let artifact_reader = build_artifact_reader(artifact_source.clone()).await?;

    println!("Starting Leptos blog server on http://{}", addr);
    println!("Leptos設定読み込み完了: {:?}", addr);
    println!("Artifact source: {}", artifact_source.kind());

    // Generate Leptos routes.
    let routes = generate_route_list(App);

    // Build the integrated Axum application.
    let app = Router::new()
        // API routes
        .nest("/api", create_api_router(artifact_reader.clone()))
        .route("/api/health", get(health))
        // Static file serving.
        .nest_service(
            "/pkg",
            axum::routing::get_service(ServeDir::new("target/site/pkg")),
        )
        .nest_service(
            "/assets",
            axum::routing::get_service(ServeDir::new("target/site/assets")),
        )
        // Leptos SSR routes must be registered last.
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let artifact_reader = artifact_reader.clone();
                move || provide_context(artifact_reader.clone())
            },
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        // Fallback handler.
        .fallback(file_and_error_handler(shell))
        .with_state(leptos_options);

    println!("Server listening on http://{}", &addr);
    println!("Visit http://{} to see the Leptos app", &addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
