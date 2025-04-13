use serde::{Deserialize, Serialize};

/// タグ情報を表す構造体
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    /// タグ名（URLに使用するスラッグも兼ねる）
    pub name: String,
    /// タグが付けられている記事数
    pub article_count: usize,
}

impl Tag {
    /// タグのURLを生成
    pub fn url(&self) -> String {
        format!("/tag/{}", self.name)
    }

    /// 新しいタグインスタンスを作成
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            article_count: 0,
        }
    }
}
