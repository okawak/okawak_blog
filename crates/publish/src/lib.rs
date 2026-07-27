#![warn(unreachable_pub)]

mod artifacts;
mod classify;
mod error;
mod links;
mod pipeline;
mod render;
mod slug;
mod vault;

pub use error::{PublishError, Result};
pub use pipeline::{publish, publish_with_bookmark_enricher};
pub use render::BookmarkEnricher;
