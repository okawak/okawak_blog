use serde::Deserialize;
use serde_json::Value;

/// データベースから以下の情報を抽出するため、
/// データベースの要素にこれらが含まれていることを仮定している
/// また、他の情報が必要であればここに追記できる
#[derive(Debug, Deserialize)]
pub struct PageInfo {
    pub id: String,
    pub title: String,
    pub category: String,
    pub group: String,
    pub priority_level: i32,
    pub tags: Vec<String>,
    pub summary: String,
    pub created_time: String,
    pub last_edited_time: String,
    pub status: String,
}

/// ブロックの種類として、以下のもののみを扱っている
/// Notionでは他にもToggleなどのブロックが存在するが、
/// 現状では対応していない
pub enum BlockType {
    Paragraph,
    LinkToPage,
    Heading1,
    Heading2,
    Heading3,
    Code,
    Image,
    Quote,
    BulletedListItem,
    NumberedListItem,
    Equation,
    BookMark,
    Divider,
    Unsupported(String),
}

pub struct BlockInfo {
    pub block_type: BlockType,
    pub content: Value,
}
