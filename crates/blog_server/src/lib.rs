pub mod app;
pub mod components;
pub mod error;
pub mod models;
pub mod routes;
#[cfg(feature = "ssr")]
pub mod services;

// サーバーサイドで使用するための関数やツールをエクスポート
pub use app::{shell, App};
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
