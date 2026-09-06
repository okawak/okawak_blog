use std::{sync::Arc, time::SystemTime};

use async_trait::async_trait;
use aws_sdk_s3::Client;
use domain::{
    ArticleIndexDocument, ArtifactReleasePointerDocument, Category, CategoryArtifactDocument,
    HomeFragmentArtifactDocument, PageArtifactDocument, PageKey, SiteMetadataDocument, Slug,
};

use crate::{ArtifactReader, ArtifactSnapshot, DynArtifactSnapshot, InfraError, Result};

pub(crate) const ARTIFACT_BUCKET_ENV: &str = "OKAWAK_BLOG_ARTIFACT_BUCKET";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3ArtifactLocation {
    bucket: String,
    prefix: Option<String>,
}

impl S3ArtifactLocation {
    pub fn new(bucket: impl Into<String>, prefix: Option<impl Into<String>>) -> Result<Self> {
        let bucket = bucket.into().trim().to_string();
        if bucket.is_empty() {
            return Err(InfraError::MissingConfig(ARTIFACT_BUCKET_ENV));
        }

        let prefix = prefix
            .map(Into::into)
            .map(|value| value.trim_matches('/').to_string())
            .filter(|value| !value.is_empty());

        Ok(Self { bucket, prefix })
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn key_for(&self, relative: &str) -> String {
        let relative = relative.trim_start_matches('/');
        match &self.prefix {
            Some(prefix) => format!("{prefix}/{relative}"),
            None => relative.to_string(),
        }
    }

    fn with_relative_prefix(&self, relative: &str) -> Self {
        let prefix = self.key_for(relative);
        Self {
            bucket: self.bucket.clone(),
            prefix: Some(prefix),
        }
    }
}

#[derive(Debug, Clone)]
pub struct S3ArtifactReader {
    client: Client,
    location: S3ArtifactLocation,
}

#[derive(Debug, Clone)]
pub struct S3ArtifactSnapshot {
    client: Client,
    location: S3ArtifactLocation,
    cache_identity: Option<String>,
    last_modified: Option<SystemTime>,
}

struct ResolvedArtifactRelease {
    location: S3ArtifactLocation,
    cache_identity: String,
    last_modified: SystemTime,
}

fn resolve_artifact_release(
    location: &S3ArtifactLocation,
    pointer: ArtifactReleasePointerDocument,
) -> Result<ResolvedArtifactRelease> {
    pointer.validate()?;
    let last_modified = pointer.generated_at_time()?;
    let cache_identity = pointer.artifact_prefix.clone();
    Ok(ResolvedArtifactRelease {
        location: location.with_relative_prefix(&pointer.artifact_prefix),
        cache_identity,
        last_modified,
    })
}

impl S3ArtifactReader {
    pub fn new(client: Client, location: S3ArtifactLocation) -> Self {
        Self { client, location }
    }

    pub fn location(&self) -> &S3ArtifactLocation {
        &self.location
    }
}

impl S3ArtifactSnapshot {
    fn new(
        client: Client,
        location: S3ArtifactLocation,
        cache_identity: Option<String>,
        last_modified: Option<SystemTime>,
    ) -> Self {
        Self {
            client,
            location,
            cache_identity,
            last_modified,
        }
    }

    async fn read_text(&self, relative: &str) -> Result<String> {
        let key = self.location.key_for(relative);
        let response = self
            .client
            .get_object()
            .bucket(self.location.bucket())
            .key(&key)
            .send()
            .await
            .map_err(|source| InfraError::s3_read(self.location.bucket(), key.clone(), source))?;
        let bytes =
            response.body.collect().await.map_err(|source| {
                InfraError::s3_read(self.location.bucket(), key.clone(), source)
            })?;

        Ok(String::from_utf8(bytes.into_bytes().to_vec())?)
    }

    async fn read_json<T>(&self, relative: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let content = self.read_text(relative).await?;
        Ok(serde_json::from_str(&content)?)
    }
}

#[async_trait]
impl ArtifactReader for S3ArtifactReader {
    async fn snapshot(&self) -> Result<DynArtifactSnapshot> {
        let base = S3ArtifactSnapshot::new(self.client.clone(), self.location.clone(), None, None);
        let (location, cache_identity, last_modified) = match base
            .read_json::<ArtifactReleasePointerDocument>("current.json")
            .await
        {
            Ok(pointer) => {
                let release = resolve_artifact_release(&self.location, pointer)?;
                (
                    release.location,
                    Some(release.cache_identity),
                    Some(release.last_modified),
                )
            }
            Err(error) if error.is_not_found() => (self.location.clone(), None, None),
            Err(error) => return Err(error),
        };

        Ok(Arc::new(S3ArtifactSnapshot::new(
            self.client.clone(),
            location,
            cache_identity,
            last_modified,
        )))
    }
}

#[async_trait]
impl ArtifactSnapshot for S3ArtifactSnapshot {
    fn cache_identity(&self) -> Option<&str> {
        self.cache_identity.as_deref()
    }

    fn last_modified(&self) -> Option<SystemTime> {
        self.last_modified
    }

    async fn read_article_index(&self) -> Result<ArticleIndexDocument> {
        self.read_json("articles/index.json").await
    }

    async fn read_category_document(
        &self,
        category: &Category,
    ) -> Result<CategoryArtifactDocument> {
        self.read_json(&format!("categories/{}.json", category.as_str()))
            .await
    }

    async fn read_site_metadata(&self) -> Result<SiteMetadataDocument> {
        self.read_json("metadata/site.json").await
    }

    async fn read_article_html(&self, category: &Category, slug: &Slug) -> Result<String> {
        self.read_text(&format!(
            "articles/{}/{}.html",
            category.as_str(),
            slug.as_str()
        ))
        .await
    }

    async fn read_home_fragment(&self) -> Result<HomeFragmentArtifactDocument> {
        self.read_json("home.json").await
    }

    async fn read_page_document(&self, page: &PageKey) -> Result<PageArtifactDocument> {
        let text = self
            .read_text(&format!("pages/{}.json", page.as_str()))
            .await?;
        Ok(serde_json::from_str(&text)?)
    }
}

#[cfg(test)]
mod tests {
    use domain::{ARTIFACT_RELEASE_SCHEMA_VERSION, ArtifactReleasePointerDocument};

    use super::*;

    #[test]
    fn location_builds_prefixed_keys() {
        let location = S3ArtifactLocation::new("blog-bucket", Some("/site/")).unwrap();

        assert_eq!(location.bucket(), "blog-bucket");
        assert_eq!(
            location.key_for("articles/index.json"),
            "site/articles/index.json"
        );
        assert_eq!(
            location.key_for("/metadata/site.json"),
            "site/metadata/site.json"
        );
    }

    #[test]
    fn location_composes_release_prefix() {
        let location = S3ArtifactLocation::new("blog-bucket", Some("public")).unwrap();
        let release = location.with_relative_prefix("releases/release-123/site");

        assert_eq!(
            release.key_for("articles/index.json"),
            "public/releases/release-123/site/articles/index.json"
        );
    }

    #[test]
    fn release_pointer_resolves_snapshot_metadata() {
        let location = S3ArtifactLocation::new("blog-bucket", Some("public")).unwrap();
        let pointer = ArtifactReleasePointerDocument {
            schema_version: ARTIFACT_RELEASE_SCHEMA_VERSION,
            release_id: "release-123".to_string(),
            artifact_prefix: "releases/release-123/site".to_string(),
            publisher_commit: "publisher-sha".to_string(),
            source_commit: "source-sha".to_string(),
            generated_at: "2026-07-12T12:00:00Z".to_string(),
        };
        let expected_last_modified = pointer.generated_at_time().unwrap();

        let release = resolve_artifact_release(&location, pointer).unwrap();

        assert_eq!(release.cache_identity, "releases/release-123/site");
        assert_eq!(release.last_modified, expected_last_modified);
        assert_eq!(
            release.location.key_for("articles/index.json"),
            "public/releases/release-123/site/articles/index.json"
        );
    }
}
