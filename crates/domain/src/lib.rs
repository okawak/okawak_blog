//! Blog domain types and business rules.
//!
//! Pure synchronous domain modeling without I/O.
//! Business rules are expressed through Rust's type system.

// Domain entities and value objects.
pub mod artifact_document;
pub mod entities;
pub mod publishable;
pub mod site_page;

// Business rules implemented as pure functions.
pub mod business_rules;

// Domain error types.
pub mod error;

// Re-exports.
pub use artifact_document::*;
pub use business_rules::*;
pub use entities::*;
pub use error::{DomainError, Result};
pub use publishable::*;
pub use site_page::*;
