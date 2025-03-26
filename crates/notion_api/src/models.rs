use serde_json::Value;

pub struct PageInfo {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created_time: String,
    pub last_edited_time: String,
    pub status: String,
}

pub enum BlockType {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
    BulletedListItem,
    NumberedListItem,
    ToDo,
    Toggle,
    ChildPage,
    Unsupported,
}

pub struct PageBlock {
    pub block_type: BlockType,
    pub content: Value,
}
