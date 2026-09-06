use std::{env, path::PathBuf, sync::Arc, time::Duration};

use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;

use crate::{
    CachingArtifactReader, DynArtifactReader, InfraError, LocalArtifactReader, Result,
    S3ArtifactLocation, S3ArtifactReader, s3::ARTIFACT_BUCKET_ENV,
};

const DEFAULT_LOCAL_SITE_ROOT: &str = "crates/publish/dist/site";
const ARTIFACT_SOURCE_ENV: &str = "OKAWAK_BLOG_ARTIFACT_SOURCE";
const ARTIFACT_LOCAL_ROOT_ENV: &str = "OKAWAK_BLOG_ARTIFACT_LOCAL_ROOT";
const ARTIFACT_PREFIX_ENV: &str = "OKAWAK_BLOG_ARTIFACT_PREFIX";
const ARTIFACT_CACHE_TTL_SECONDS_ENV: &str = "OKAWAK_BLOG_ARTIFACT_CACHE_TTL_SECONDS";
const DEFAULT_ARTIFACT_CACHE_TTL_SECONDS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactSourceConfig {
    Local {
        site_root: PathBuf,
    },
    S3 {
        location: S3ArtifactLocation,
        cache_ttl: Duration,
    },
}

impl ArtifactSourceConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_env_with(|key| env::var(key).ok())
    }

    fn from_env_with(mut read_var: impl FnMut(&str) -> Option<String>) -> Result<Self> {
        let source = read_var(ARTIFACT_SOURCE_ENV).unwrap_or_else(|| "local".to_string());
        match source.as_str() {
            "local" => Ok(Self::Local {
                site_root: PathBuf::from(
                    read_var(ARTIFACT_LOCAL_ROOT_ENV)
                        .unwrap_or_else(|| DEFAULT_LOCAL_SITE_ROOT.to_string()),
                ),
            }),
            "s3" => {
                let bucket = read_var(ARTIFACT_BUCKET_ENV)
                    .ok_or(InfraError::MissingConfig(ARTIFACT_BUCKET_ENV))?;
                let prefix = read_var(ARTIFACT_PREFIX_ENV);
                let cache_ttl = read_var(ARTIFACT_CACHE_TTL_SECONDS_ENV)
                    .map(|value| {
                        value.parse::<u64>().map(Duration::from_secs).map_err(|_| {
                            InfraError::InvalidConfig {
                                key: ARTIFACT_CACHE_TTL_SECONDS_ENV,
                                value,
                            }
                        })
                    })
                    .transpose()?
                    .unwrap_or(Duration::from_secs(DEFAULT_ARTIFACT_CACHE_TTL_SECONDS));
                Ok(Self::S3 {
                    location: S3ArtifactLocation::new(bucket, prefix)?,
                    cache_ttl,
                })
            }
            unsupported => Err(InfraError::UnsupportedSource(unsupported.to_string())),
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Local { .. } => "local",
            Self::S3 { .. } => "s3",
        }
    }
}

pub async fn build_artifact_reader(config: ArtifactSourceConfig) -> Result<DynArtifactReader> {
    match config {
        ArtifactSourceConfig::Local { site_root } => {
            Ok(Arc::new(LocalArtifactReader::new(site_root)))
        }
        ArtifactSourceConfig::S3 {
            location,
            cache_ttl,
        } => {
            let shared_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
            let client = Client::new(&shared_config);
            let reader: DynArtifactReader = Arc::new(S3ArtifactReader::new(client, location));
            Ok(Arc::new(CachingArtifactReader::new(reader, cache_ttl)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_local_reader() {
        let source = ArtifactSourceConfig::Local {
            site_root: PathBuf::from(DEFAULT_LOCAL_SITE_ROOT),
        };

        assert_eq!(source.kind(), "local");
    }

    #[test]
    fn env_defaults_to_local_site_root() {
        let source = ArtifactSourceConfig::from_env_with(|_| None).unwrap();

        assert_eq!(
            source,
            ArtifactSourceConfig::Local {
                site_root: PathBuf::from(DEFAULT_LOCAL_SITE_ROOT),
            }
        );
    }

    #[test]
    fn env_uses_local_override() {
        let source = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("local".to_string()),
            ARTIFACT_LOCAL_ROOT_ENV => Some("/tmp/site".to_string()),
            _ => None,
        })
        .unwrap();

        assert_eq!(
            source,
            ArtifactSourceConfig::Local {
                site_root: PathBuf::from("/tmp/site"),
            }
        );
    }

    #[test]
    fn env_builds_s3_location() {
        let source = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("s3".to_string()),
            ARTIFACT_BUCKET_ENV => Some("blog-bucket".to_string()),
            ARTIFACT_PREFIX_ENV => Some("/public/site/".to_string()),
            _ => None,
        })
        .unwrap();

        assert_eq!(
            source,
            ArtifactSourceConfig::S3 {
                location: S3ArtifactLocation::new("blog-bucket", Some("/public/site/")).unwrap(),
                cache_ttl: Duration::from_secs(DEFAULT_ARTIFACT_CACHE_TTL_SECONDS),
            }
        );
    }

    #[test]
    fn env_uses_s3_cache_ttl_override() {
        let source = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("s3".to_string()),
            ARTIFACT_BUCKET_ENV => Some("blog-bucket".to_string()),
            ARTIFACT_CACHE_TTL_SECONDS_ENV => Some("0".to_string()),
            _ => None,
        })
        .unwrap();

        assert_eq!(
            source,
            ArtifactSourceConfig::S3 {
                location: S3ArtifactLocation::new("blog-bucket", None::<String>).unwrap(),
                cache_ttl: Duration::ZERO,
            }
        );
    }

    #[test]
    fn env_rejects_invalid_s3_cache_ttl() {
        let result = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("s3".to_string()),
            ARTIFACT_BUCKET_ENV => Some("blog-bucket".to_string()),
            ARTIFACT_CACHE_TTL_SECONDS_ENV => Some("soon".to_string()),
            _ => None,
        });

        assert!(matches!(
            result,
            Err(InfraError::InvalidConfig { key, value })
                if key == ARTIFACT_CACHE_TTL_SECONDS_ENV && value == "soon"
        ));
    }

    #[test]
    fn env_requires_s3_bucket() {
        let result = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("s3".to_string()),
            _ => None,
        });

        assert!(matches!(
            result,
            Err(InfraError::MissingConfig(ARTIFACT_BUCKET_ENV))
        ));
    }

    #[test]
    fn env_rejects_unsupported_source() {
        let result = ArtifactSourceConfig::from_env_with(|key| match key {
            ARTIFACT_SOURCE_ENV => Some("filesystem".to_string()),
            _ => None,
        });

        assert!(matches!(
            result,
            Err(InfraError::UnsupportedSource(source)) if source == "filesystem"
        ));
    }
}
