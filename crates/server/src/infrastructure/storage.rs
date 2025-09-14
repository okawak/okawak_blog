//! S3 Storage Implementation - ファイルストレージサービス実装

use async_trait::async_trait;
use aws_sdk_s3::Client as S3Client;
use domain::{CoreError, Result, ports::FileStorage};

/// S3-based file storage implementation
#[derive(Debug, Clone)]
pub struct S3FileStorage {
    client: S3Client,
    bucket: String,
}

impl S3FileStorage {
    pub fn new(client: S3Client, bucket: String) -> Self {
        Self { client, bucket }
    }
}

#[async_trait]
impl FileStorage for S3FileStorage {
    async fn get_html(&self, _key: &str) -> Result<String> {
        todo!("S3 HTML取得機能の実装")
    }

    async fn save_html(&self, _key: &str, _content: &str) -> Result<()> {
        todo!("S3 HTML保存機能の実装")
    }

    async fn delete(&self, _key: &str) -> Result<()> {
        todo!("S3 ファイル削除機能の実装")
    }

    async fn list_files(&self, _prefix: &str) -> Result<Vec<String>> {
        todo!("S3 ファイル一覧機能の実装")
    }
}
