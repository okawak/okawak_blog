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
    /// 記事の内容（Markdown形式）
    pub content: String,
    /// 記事の要約文
    pub excerpt: String,
    /// サムネイル画像のURL（オプショナル）
    pub thumbnail_url: Option<String>,
    /// 記事に関連するタグのリスト
    pub tags: Vec<String>,
    /// 投稿日時
    pub published_at: NaiveDate,
    /// 最終更新日時（オプショナル）
    pub updated_at: Option<NaiveDate>,
    /// SEO用のメタ説明
    pub description: String,
    /// OGP画像のURL（ソーシャル共有用）
    pub og_image: Option<String>,
    /// 記事を公開するかどうか
    pub published: bool,
}

impl Article {
    /// 記事のURLを生成
    pub fn url(&self) -> String {
        format!("/{}/{}", self.category, self.slug)
    }

    /// 記事の投稿日を「YYYY年MM月DD日」形式でフォーマット
    pub fn date_formatted(&self) -> String {
        self.published_at.format("%Y年%m月%d日").to_string()
    }

    /// 記事の概要を取得
    pub fn to_summary(&self) -> ArticleSummary {
        ArticleSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            slug: self.slug.clone(),
            category: self.category.clone(),
            excerpt: self.excerpt.clone(),
            thumbnail_url: self.thumbnail_url.clone(),
            tags: self.tags.clone(),
            published_at: self.published_at,
            date_formatted: self.date_formatted(),
        }
    }
}

impl HasMetadata for Article {
    fn title(&self) -> &str {
        &self.title
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn og_image(&self) -> Option<&str> {
        self.og_image.as_deref()
    }

    fn published_at(&self) -> NaiveDate {
        self.published_at
    }

    fn updated_at(&self) -> Option<NaiveDate> {
        self.updated_at
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
    /// 記事の要約文
    pub excerpt: String,
    /// サムネイル画像のURL（オプショナル）
    pub thumbnail_url: Option<String>,
    /// 記事に関連するタグのリスト
    pub tags: Vec<String>,
    /// 投稿日時
    pub published_at: NaiveDate,
    /// フォーマット済みの日付文字列
    pub date_formatted: String,
}

impl ArticleSummary {
    /// 記事のURLを生成
    pub fn url(&self) -> String {
        format!("/{}/{}", self.category, self.slug)
    }
}
