pub mod app;
pub mod components;
pub mod error;
pub mod models;
pub mod routes;
#[cfg(feature = "ssr")]
pub mod services;

// サーバーサイドで使用するための関数やツールをエクスポート
pub use app::{App, shell};
pub use error::AppError;

// クライアントサイドのハイドレーション用エントリーポイント
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    // パニック時にブラウザコンソールにエラーを表示するためのフック設定
    console_error_panic_hook::set_once();
    // Appコンポーネントを使用してbody要素をハイドレーション
    leptos::mount::hydrate_body(App);
}

// ログ設定用ヘルパー関数（任意）
#[cfg(feature = "ssr")]
pub fn setup_logging() {
    use log::Level;
    let log_level = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info".to_string())
        .parse::<Level>()
        .unwrap_or(Level::Info);

    simple_logger::init_with_level(log_level).expect("Failed to initialize logger");
}
