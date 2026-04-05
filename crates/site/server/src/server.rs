//! Server module - アプリケーションサーバー設定（スタブ）

use std::sync::Arc;

use infra::ArtifactReader;

pub struct AppState {
    pub artifact_reader: Arc<dyn ArtifactReader>,
}
