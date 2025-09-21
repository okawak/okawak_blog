//! Infrastructure - 旧アーキテクチャ互換性スタブ
//! 将来的に削除予定

use crate::ports::{ArticleRepository, FileStorage};
use async_trait::async_trait;
use domain::{Article, ArticleSummary};

pub struct MemoryArticleRepository;

impl MemoryArticleRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn seed_data(&self) {
        // スタブ実装
    }
}

#[async_trait]
impl ArticleRepository for MemoryArticleRepository {
    async fn find_by_id(&self, _id: &str) -> crate::ports::Result<Option<Article>> {
        // スタブ実装
        Ok(None)
    }

    async fn find_by_slug(&self, _slug: &str) -> crate::ports::Result<Option<Article>> {
        // スタブ実装
        Ok(None)
    }

    async fn list_all(&self) -> crate::ports::Result<Vec<ArticleSummary>> {
        // スタブ実装
        Ok(vec![])
    }
}

pub struct S3Storage;

impl S3Storage {
    pub fn new(_s3_client: aws_sdk_s3::Client, _bucket: String) -> Self {
        Self
    }
}

#[async_trait]
impl FileStorage for S3Storage {
    async fn get_content(&self, _path: &str) -> crate::ports::Result<String> {
        // スタブ実装
        Ok(String::new())
    }

    async fn put_content(&self, _path: &str, _content: &str) -> crate::ports::Result<()> {
        // スタブ実装
        Ok(())
    }
}
