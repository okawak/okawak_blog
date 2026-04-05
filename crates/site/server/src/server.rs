//! Server module - アプリケーションサーバー設定（スタブ）

use std::sync::Arc;

use infra::LocalArtifactReader;

pub struct AppState {
    pub artifact_reader: Arc<LocalArtifactReader>,
}
