use crate::error::ServiceError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    pub s3: S3Config,
    pub blog: BlogConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct S3Config {
    pub bucket_name: String,
    pub region: String,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlogConfig {
    pub posts_prefix: String,
    pub cache_duration_seconds: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            s3: S3Config {
                bucket_name: "okawak-blog-posts".to_string(),
                region: "ap-northeast-1".to_string(),
                profile: Some("blog-s3".to_string()),
            },
            blog: BlogConfig {
                posts_prefix: "posts/".to_string(),
                cache_duration_seconds: 300, // 5 minutes
            },
        }
    }
}

impl ServiceConfig {
    pub fn from_env() -> Result<Self, ServiceError> {
        let settings = config::Config::builder()
            .add_source(config::Environment::with_prefix("BLOG"))
            .build()
            .map_err(|e| ServiceError::ConfigError(e.to_string()))?;

        settings
            .try_deserialize()
            .or_else(|_: config::ConfigError| Ok(Self::default()))
    }
}
