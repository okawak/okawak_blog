//! Blog Handlers - 旧アーキテクチャ互換性スタブ
//! 将来的に削除予定

use axum::Router;

// スタブハンドラー関数
pub async fn list_articles() -> &'static str {
    "[]"
}

pub fn create_blog_router() -> Router {
    Router::new().route("/articles", axum::routing::get(list_articles))
}
