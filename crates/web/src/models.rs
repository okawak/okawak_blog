//! ブログのデータモデル
//!
//! このモジュールはブログで使用される主要なデータ構造を定義します。
//! 記事、カテゴリー、タグなどの基本モデルが含まれています。

use chrono::NaiveDate;

// サブモジュールを公開
pub mod article;
pub mod category;
pub mod tag;

// 頻繁に使用されるモデルを再エクスポート
pub use article::{Article, ArticleSummary};
pub use category::Category;
pub use tag::Tag;

/// 共通のメタデータ特性
/// 記事やその他のコンテンツに共通する情報を定義します
pub trait HasMetadata {
    /// コンテンツのタイトルを取得します
    fn title(&self) -> &str;

    /// SEO用の説明文を取得します
    fn description(&self) -> &str;

    /// 投稿日時を取得します
    fn published_at(&self) -> NaiveDate;
}

/// ページネーション情報を保持する構造体
#[derive(Clone, Debug)]
pub struct Pagination {
    /// 現在のページ番号（1から始まる）
    pub current_page: usize,
    /// 1ページあたりのアイテム数
    pub items_per_page: usize,
    /// 総アイテム数
    pub total_items: usize,
    /// 総ページ数
    pub total_pages: usize,
}

impl Pagination {
    /// 新しいページネーションを作成
    pub fn new(current_page: usize, items_per_page: usize, total_items: usize) -> Self {
        let total_pages = (total_items as f64 / items_per_page as f64).ceil() as usize;
        let current_page = if current_page < 1 {
            1
        } else if current_page > total_pages && total_pages > 0 {
            total_pages
        } else {
            current_page
        };

        Self {
            current_page,
            items_per_page,
            total_items,
            total_pages,
        }
    }

    /// 前のページがあるかどうか
    pub fn has_prev(&self) -> bool {
        self.current_page > 1
    }

    /// 次のページがあるかどうか
    pub fn has_next(&self) -> bool {
        self.current_page < self.total_pages
    }

    /// 前のページ番号を取得
    pub fn prev_page(&self) -> Option<usize> {
        if self.has_prev() {
            Some(self.current_page - 1)
        } else {
            None
        }
    }

    /// 次のページ番号を取得
    pub fn next_page(&self) -> Option<usize> {
        if self.has_next() {
            Some(self.current_page + 1)
        } else {
            None
        }
    }

    /// ページネーションの範囲を取得
    /// 現在のページの前後に表示するページ数を指定できます
    pub fn page_range(&self, surround: usize) -> Vec<usize> {
        let start = self.current_page.saturating_sub(surround);
        let end = (self.current_page + surround).min(self.total_pages);

        (start..=end).collect()
    }
}
