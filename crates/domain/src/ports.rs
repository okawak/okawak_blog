//! Ports - 外部サービス用トレイト（Dependency Inversion）
//!
//! Rustのトレイトシステムを活用した抽象化

use crate::{Article, ArticleId, ArticleSummary, Category, CoreError, Result, Slug};
use async_trait::async_trait;

// =============================================================================
// Repository Traits - データアクセス抽象化
// =============================================================================

/// 記事リポジトリトレイト
#[async_trait]
pub trait ArticleRepository: Send + Sync {
    /// IDで記事を取得
    async fn find_by_id(&self, id: &ArticleId) -> Result<Option<Article>>;

    /// スラッグで記事を取得
    async fn find_by_slug(&self, category: &Category, slug: &Slug) -> Result<Option<Article>>;

    /// カテゴリ別記事一覧を取得
    async fn find_by_category(
        &self,
        category: &Category,
        limit: Option<usize>,
    ) -> Result<Vec<ArticleSummary>>;

    /// 最新記事一覧を取得
    async fn find_latest(&self, limit: usize) -> Result<Vec<ArticleSummary>>;

    /// 記事を保存
    async fn save(&self, article: &Article) -> Result<()>;

    /// 記事を削除
    async fn delete(&self, id: &ArticleId) -> Result<()>;

    /// 検索
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<ArticleSummary>>;
}

// =============================================================================
// External Service Traits - 外部サービス抽象化
// =============================================================================

/// ファイルストレージサービス（S3等）
#[async_trait]
pub trait FileStorage: Send + Sync {
    /// HTMLファイルを取得
    async fn get_html(&self, key: &str) -> Result<String>;

    /// HTMLファイルを保存
    async fn save_html(&self, key: &str, content: &str) -> Result<()>;

    /// ファイルを削除
    async fn delete(&self, key: &str) -> Result<()>;

    /// ファイル一覧を取得
    async fn list_files(&self, prefix: &str) -> Result<Vec<String>>;
}

/// 検索サービス
#[async_trait]
pub trait SearchService: Send + Sync {
    /// インデックスを更新
    async fn index_article(&self, article: &Article) -> Result<()>;

    /// インデックスから削除
    async fn remove_article(&self, id: &ArticleId) -> Result<()>;

    /// 記事を検索
    async fn search(&self, query: &str, limit: Option<usize>) -> Result<Vec<ArticleId>>;
}

/// 通知サービス（将来的な拡張用）
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// 記事公開通知
    async fn notify_article_published(&self, article: &Article) -> Result<()>;
}

// =============================================================================
// Aggregate Services - 複合サービス
// =============================================================================

/// 記事に関連するすべてのサービスを束ねるトレイト
#[async_trait]
pub trait ArticleServices: Send + Sync {
    type Repository: ArticleRepository;
    type Storage: FileStorage;
    type Search: SearchService;

    fn repository(&self) -> &Self::Repository;
    fn storage(&self) -> &Self::Storage;
    fn search(&self) -> &Self::Search;
}

// =============================================================================
// Event Types - ドメインイベント（将来的な拡張用）
// =============================================================================

/// ドメインイベント
#[derive(Debug, Clone)]
pub enum DomainEvent {
    ArticleCreated { article_id: ArticleId },
    ArticlePublished { article_id: ArticleId },
    ArticleUpdated { article_id: ArticleId },
    ArticleDeleted { article_id: ArticleId },
}

/// イベントハンドラートレイト
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: DomainEvent) -> Result<()>;
}
