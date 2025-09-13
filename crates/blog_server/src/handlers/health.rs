//! Health Check Handler

use axum::{http::StatusCode, response::Json};
use serde_json::{Value, json};

/// Health check endpoint
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "ok",
        "service": "blog-server",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
