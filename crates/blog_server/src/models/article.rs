use crate::models::HasMetadata;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 完全な記事データを表す構造体
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Article {
    /// 記事のユニークID
    pub id: String,
    /// 記事のタイトル
    pub title: String,
    /// URLに使用されるスラッグ
    pub slug: String,
    /// 記事が所属するカテゴリー
    pub category: String,
    /// カテゴリーの中のグループの名前
    pub group: String,
    /// 記事の優先順位
    pub priority_level: i32,
    /// 記事の要約文
    pub summary: String,
    /// 記事に関連するタグのリスト
    pub tags: Vec<String>,
    /// フォーマット無しの投稿日
    pub published_date: NaiveDate,
    /// フォーマット済みの投稿日
    pub published_at: String,
    /// フォーマット済みの最終変更日
    pub updated_at: String,
    /// 記事の内容（Markdown形式）
    pub content: String,
}

impl Article {
    /// 記事のURLを生成
    pub fn url(&self) -> String {
        format!("/{}/{}", self.category, self.slug)
    }

    /// 記事の投稿日を「YYYY年MM月DD日」形式でフォーマット
    pub fn date_formatted(&self) -> String {
        self.published_date.format("%Y年%m月%d日").to_string()
    }

    /// 記事の概要を取得
    pub fn to_summary(&self) -> ArticleSummary {
        ArticleSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            slug: self.slug.clone(),
            category: self.category.clone(),
            group: self.group.clone(),
            priority_level: self.priority_level,
            summary: self.summary.clone(),
            tags: self.tags.clone(),
            published_date: self.published_date,
            published_at: self.published_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

impl HasMetadata for Article {
    fn title(&self) -> &str {
        &self.title
    }

    fn description(&self) -> &str {
        &self.summary
    }

    fn published_at(&self) -> NaiveDate {
        self.published_date
    }
}

/// 記事の概要情報を表す構造体
/// 一覧表示などに使用されます
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArticleSummary {
    /// 記事のユニークID
    pub id: String,
    /// 記事のタイトル
    pub title: String,
    /// URLに使用されるスラッグ
    pub slug: String,
    /// 記事が所属するカテゴリー
    pub category: String,
    /// カテゴリーの中のグループの名前
    pub group: String,
    /// 記事の優先順位
    pub priority_level: i32,
    /// 記事の要約文
    pub summary: String,
    /// 記事に関連するタグのリスト
    pub tags: Vec<String>,
    /// フォーマット無しの投稿日
    pub published_date: NaiveDate,
    /// フォーマット済みの投稿日
    pub published_at: String,
    /// フォーマット済みの最終変更日
    pub updated_at: String,
}

impl ArticleSummary {
    /// 記事のURLを生成
    pub fn url(&self) -> String {
        format!("/{}/{}", self.category, self.slug)
    }
}
