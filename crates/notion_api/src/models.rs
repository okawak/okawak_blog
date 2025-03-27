use serde_json::Value;

/// データベースから以下の情報を抽出するため、
/// データベースの要素にこれらが含まれていることを仮定している
/// また、他の情報が必要であればここに追記できる
pub struct PageInfo {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
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
    BulletedListItem,
    NumberedListItem,
    Unsupported,
}

pub struct BlockInfo {
    pub block_type: BlockType,
    pub content: Value,
}
