#![warn(unreachable_pub)]

//! Artifact storage boundary for the site runtime.

mod cache;
mod config;
mod contract;
mod error;
mod local;
mod s3;

pub use cache::CachingArtifactReader;
pub use config::{ArtifactSourceConfig, build_artifact_reader};
pub use contract::{ArtifactReader, ArtifactSnapshot, DynArtifactReader, DynArtifactSnapshot};
pub use error::{InfraError, Result};
pub use local::LocalArtifactReader;
pub use s3::{S3ArtifactLocation, S3ArtifactReader, S3ArtifactSnapshot};
