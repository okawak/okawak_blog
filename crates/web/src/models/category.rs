use serde::{Deserialize, Serialize};

/// カテゴリー情報を表す構造体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Category {
    /// カテゴリーのスラッグ（URLに使用）
    pub slug: String,
    /// カテゴリーの表示名
    pub name: String,
    /// カテゴリーの説明
    pub description: String,
    /// カテゴリーに含まれる記事数
    pub article_count: usize,
}

impl Category {
    /// カテゴリーのURLを生成
    pub fn url(&self) -> String {
        format!("/{}", self.slug)
    }

    /// 新しいカテゴリーインスタンスを作成
    pub fn new(slug: &str, name: &str, description: &str) -> Self {
        Self {
            slug: slug.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            article_count: 0,
        }
    }
}
