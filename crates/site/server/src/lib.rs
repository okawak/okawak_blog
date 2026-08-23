//! Blog server library with wasm-aware exports.
//!
//! The wasm target re-exports the `web` crate,
//! while native targets expose server functionality.

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod article_index;

#[cfg(not(target_arch = "wasm32"))]
pub mod handlers;

#[cfg(not(target_arch = "wasm32"))]
pub mod http_cache;

#[cfg(not(target_arch = "wasm32"))]
pub mod readiness;

#[cfg(not(target_arch = "wasm32"))]
pub mod topcoat_runtime;

#[cfg(not(target_arch = "wasm32"))]
mod topcoat_pages;
