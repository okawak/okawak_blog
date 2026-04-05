//! Blog Server Library - WASM向けweb crate再エクスポート
//!
//! WASMターゲットではweb crateを使用し、
//! ネイティブターゲットではサーバー機能を提供

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod auth;
#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod handlers;
#[cfg(not(target_arch = "wasm32"))]
pub mod presentation;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;

// 旧アーキテクチャとの互換性のためのスタブモジュール
#[cfg(not(target_arch = "wasm32"))]
pub mod error;
#[cfg(not(target_arch = "wasm32"))]
pub mod infrastructure;
#[cfg(not(target_arch = "wasm32"))]
pub mod ports;
#[cfg(not(target_arch = "wasm32"))]
pub mod usecases;
