//! Article API Handlers - blog_core のユースケースを使用

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use core::Category;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::infrastructure::{InMemoryArticleRepository, InMemorySearchService, S3FileStorage};

/// アプリケーション状態
#[derive(Clone)]
pub struct AppState {
    pub repository: Arc<InMemoryArticleRepository>,
    pub storage: Arc<S3FileStorage>,
    pub search: Arc<InMemorySearchService>,
}

// DTO for API responses
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub message: String,
}

#[derive(Deserialize)]
pub struct CreateArticleRequest {
    pub title: String,
    pub content: String,
    pub category: Category,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

/// 記事作成 API (簡易実装)
pub async fn create_article_handler(
    State(_state): State<AppState>,
    Json(_req): Json<CreateArticleRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // 簡易実装 - 後で本格改修予定
    Ok(Json(ApiResponse {
        data: "new-article-id".to_string(),
        message: "記事が作成されました".to_string(),
    }))
}

/// 記事公開 API (簡易実装)
pub async fn publish_article_handler(
    State(_state): State<AppState>,
    Path(_article_id): Path<String>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // 簡易実装 - 後で本格改修予定
    Ok(Json(ApiResponse {
        data: "published".to_string(),
        message: "記事が公開されました".to_string(),
    }))
}

/// 記事取得 API (簡易実装)
pub async fn get_article_handler(
    State(_state): State<AppState>,
    Path((_category, _slug)): Path<(String, String)>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // 簡易実装 - 後で本格改修予定
    Ok(Json(ApiResponse {
        data: "記事データ（未実装）".to_string(),
        message: "記事を取得しました".to_string(),
    }))
}

/// カテゴリ別記事一覧 API (簡易実装)
pub async fn list_articles_by_category_handler(
    State(_state): State<AppState>,
    Path(_category): Path<String>,
    Query(_query): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    // 簡易実装 - 後で本格改修予定
    Ok(Json(ApiResponse {
        data: vec!["記事1".to_string(), "記事2".to_string()],
        message: "記事一覧を取得しました".to_string(),
    }))
}

/// 最新記事一覧 API (簡易実装)
pub async fn list_latest_articles_handler(
    State(_state): State<AppState>,
    Query(_query): Query<ListQuery>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    // 簡易実装 - 後で本格改修予定
    Ok(Json(ApiResponse {
        data: vec!["最新記事1".to_string(), "最新記事2".to_string()],
        message: "最新記事一覧を取得しました".to_string(),
    }))
}

/// 記事検索 API (簡易実装)
pub async fn search_articles_handler(
    State(_state): State<AppState>,
    Query(_query): Query<SearchQuery>,
) -> Result<Json<ApiResponse<Vec<String>>>, StatusCode> {
    // 簡易実装 - 後で本格改修予定
    Ok(Json(ApiResponse {
        data: vec!["検索結果1".to_string(), "検索結果2".to_string()],
        message: "検索結果を取得しました".to_string(),
    }))
}
