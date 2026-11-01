//! Runtime API endpoints backed by the configured artifact reader.

mod articles;
mod health;
mod ready;

use blog_infra::DynArtifactReader;

#[derive(Clone)]
pub(super) struct ArtifactReaderContext(pub(super) DynArtifactReader);
