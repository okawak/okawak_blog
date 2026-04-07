use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type used by the frontend.
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

// No `DomainError` conversion is provided because `web` does not depend on `domain`.

impl FrontendError {
    /// Helper for constructing network errors.
    pub fn network_error<S: Into<String>>(message: S) -> Self {
        Self::NetworkError {
            message: message.into(),
        }
    }

    /// Helper for constructing data loading errors.
    pub fn load_error<S: Into<String>>(message: S) -> Self {
        Self::LoadError {
            message: message.into(),
        }
    }
}

/// Component that renders an error message.
#[component]
pub fn ErrorTemplate(#[prop(into)] err: String) -> impl IntoView {
    // Log the error before rendering it.
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

/// Component that renders a typed frontend error directly.
#[component]
pub fn DisplayError(error: FrontendError) -> impl IntoView {
    let err_string = error.to_string();
    view! { <ErrorTemplate err=err_string /> }
}
