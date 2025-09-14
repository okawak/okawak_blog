use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// フロントエンド用のエラータイプ
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum FrontendError {
    #[error("ネットワークエラー: {message}")]
    NetworkError { message: String },

    #[error("データの読み込みに失敗しました: {message}")]
    LoadError { message: String },

    #[error("レンダリングエラー: {message}")]
    RenderError { message: String },

    #[error("ナビゲーションエラー: {message}")]
    NavigationError { message: String },
}

// thiserrorと手動実装を組み合わせた簡潔な変換
impl From<reqwest::Error> for FrontendError {
    fn from(err: reqwest::Error) -> Self {
        Self::NetworkError {
            message: err.to_string(),
        }
    }
}

impl From<lol_html::errors::RewritingError> for FrontendError {
    fn from(err: lol_html::errors::RewritingError) -> Self {
        Self::RenderError {
            message: err.to_string(),
        }
    }
}

impl FrontendError {
    /// ネットワークエラーを作成するヘルパーメソッド
    pub fn network_error<S: Into<String>>(message: S) -> Self {
        Self::NetworkError {
            message: message.into(),
        }
    }

    /// データ読み込みエラーを作成するヘルパーメソッド
    pub fn load_error<S: Into<String>>(message: S) -> Self {
        Self::LoadError {
            message: message.into(),
        }
    }
}

/// エラーを表示するコンポーネント
#[component]
pub fn ErrorTemplate(#[prop(into)] err: String) -> impl IntoView {
    // エラーをログに記録
    log::error!("Error: {err}");

    view! {
        <div class="error-container">
            <div class="error-message">
                <h1>エラーが発生しました</h1>
                <p>{err}</p>
                <a href="/">トップページに戻る</a>
            </div>
        </div>
    }
}

/// エラーを直接表示するためのコンポーネント
#[component]
pub fn DisplayError(error: FrontendError) -> impl IntoView {
    let err_string = error.to_string();
    view! { <ErrorTemplate err=err_string /> }
}
