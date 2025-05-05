#[cfg(feature = "ssr")]
use aws_smithy_runtime_api::client::result::SdkError;

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// アプリケーション全体で使用するエラータイプ
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum AppError {
    #[error("S3アクセスエラー: {0}")]
    S3Error(String),

    #[error("rewriteエラー: {0}")]
    RewriteError(String),

    #[error("reqwestエラー: {0}")]
    ReqwestError(String),

    #[error("ファイル '{0}' が見つかりません")]
    FileNotFound(String),

    #[error("Markdownパースエラー: {0}")]
    MarkdownError(String),

    #[error("内部サーバーエラー: {0}")]
    ServerError(String),

    #[error("認証エラー: {0}")]
    AuthError(String),
}

// FromトレイトをRewritingErrorに対して実装
impl From<lol_html::errors::RewritingError> for AppError {
    fn from(err: lol_html::errors::RewritingError) -> Self {
        Self::RewriteError(err.to_string())
    }
}

// Fromトレイトをreqwest::Errorに対して実装
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::ReqwestError(err.to_string())
    }
}

impl AppError {
    /// 文字列からMarkdownエラーを作成するヘルパーメソッド
    pub fn markdown_error<S: Into<String>>(message: S) -> Self {
        Self::MarkdownError(message.into())
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
pub fn DisplayError(error: AppError) -> impl IntoView {
    let err_string = error.to_string();
    view! { <ErrorTemplate err=err_string /> }
}

/// AWS SDKのエラーをAppErrorに変換するためのFromトレイト実装
#[cfg(feature = "ssr")]
impl From<aws_sdk_s3::Error> for AppError {
    fn from(err: aws_sdk_s3::Error) -> Self {
        Self::S3Error(err.to_string())
    }
}

/// すべてのSdkErrorタイプをAppErrorに変換する汎用実装
#[cfg(feature = "ssr")]
impl<E, O> From<SdkError<E, O>> for AppError
where
    E: std::fmt::Display,
    O: std::fmt::Debug,
{
    fn from(err: SdkError<E, O>) -> Self {
        Self::S3Error(err.to_string())
    }
}

/// 標準ライブラリのIOエラーの変換
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::ServerError(err.to_string())
    }
}
