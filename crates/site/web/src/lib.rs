#![recursion_limit = "512"]

pub mod app;
pub mod components;
pub mod error;
pub mod routes;
pub mod types; // Web-specific type definitions.

// Re-export functions and types used on the server side.
pub use app::{App, shell};
pub use error::FrontendError;

// Client-side hydration entry point.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    // Forward panic output to the browser console.
    console_error_panic_hook::set_once();
    // Hydrate the body using the App component.
    leptos::mount::hydrate_body(App);
}
