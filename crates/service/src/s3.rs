use crate::{Result, config::S3Config, error::ServiceError};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;

#[derive(Clone)]
pub struct S3Service {
    client: Client,
    bucket: String,
}

impl S3Service {
    pub async fn new(config: &S3Config) -> Result<Self> {
        let region_provider = RegionProviderChain::default_provider()
            .or_else(aws_types::region::Region::new(config.region.clone()));

        let aws_config = if let Some(profile) = &config.profile {
            aws_config::from_env()
                .profile_name(profile)
                .region(region_provider)
                .load()
                .await
        } else {
            aws_config::from_env().region(region_provider).load().await
        };

        let client = Client::new(&aws_config);

        Ok(Self {
            client,
            bucket: config.bucket_name.clone(),
        })
    }

    pub async fn get_object(&self, key: &str) -> Result<String> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ServiceError::S3Error(format!("Failed to get object {}: {}", key, e)))?;

        let body = response
            .body
            .collect()
            .await
            .map_err(|e| ServiceError::S3Error(format!("Failed to read object body: {}", e)))?;

        String::from_utf8(body.into_bytes().to_vec())
            .map_err(|e| ServiceError::S3Error(format!("Failed to parse UTF-8: {}", e)))
    }

    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<String>> {
        let response = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .map_err(|e| ServiceError::S3Error(format!("Failed to list objects: {}", e)))?;

        let keys = response
            .contents()
            .iter()
            .filter_map(|obj| obj.key())
            .map(|key| key.to_string())
            .collect();

        Ok(keys)
    }

    pub async fn object_exists(&self, key: &str) -> Result<bool> {
        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a "not found" error
                if e.to_string().contains("404") || e.to_string().contains("NoSuchKey") {
                    Ok(false)
                } else {
                    Err(ServiceError::S3Error(format!(
                        "Failed to check object existence: {}",
                        e
                    )))
                }
            }
        }
    }
}
