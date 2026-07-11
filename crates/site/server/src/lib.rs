//! Blog server library with wasm-aware exports.
//!
//! The wasm target re-exports the `web` crate,
//! while native targets expose server functionality.

#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
pub mod handlers;
