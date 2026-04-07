//! Blog server library with wasm-aware exports.
//!
//! The wasm target re-exports the `web` crate,
//! while native targets expose server functionality.

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
#[cfg(all(not(target_arch = "wasm32"), test))]
pub mod test_support;
