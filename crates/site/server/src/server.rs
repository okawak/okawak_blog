//! Server module - アプリケーションサーバー設定（スタブ）

use std::sync::Arc;

use crate::infrastructure::{MemoryArticleRepository, S3Storage};

pub struct AppState {
    pub _repository: Arc<MemoryArticleRepository>,
    pub _storage: Arc<S3Storage>,
}
