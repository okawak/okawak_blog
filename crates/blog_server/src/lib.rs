//! Blog Server - Pure Backend API
//!
//! Axum-based REST API server that implements infrastructure concerns
//! and provides endpoints for the blog application.

pub mod config;
pub mod handlers;
pub mod infrastructure;
pub mod server;

pub use config::Config;
pub use server::create_app;

/// Re-export blog_core for convenience
pub use blog_core;
