mod bulleted_list_item;
mod code;
mod heading;
mod link_to_page;
mod numbered_list_item;
mod paragraph;

use crate::models::{BlockInfo, BlockType, PageInfo};
use serde_json::Value;
use std::error::Error;

pub fn extract_blocks(json: &Value) -> Result<Vec<BlockInfo>, Box<dyn Error>> {
    let results = json
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or("No results array")?;
    let blocks = results
        .iter()
        .map(|block| {
            let block_type_str = block
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unsupported");
            let block_type = match block_type_str {
                "paragraph" => BlockType::Paragraph,
                "link_to_page" => BlockType::LinkToPage,
                "heading_1" => BlockType::Heading1,
                "heading_2" => BlockType::Heading2,
                "heading_3" => BlockType::Heading3,
                "code" => BlockType::Code,
                "bulleted_list_item" => BlockType::BulletedListItem,
                "numbered_list_item" => BlockType::NumberedListItem,
                _ => BlockType::Unsupported,
            };
            let content = block.get(block_type_str).unwrap().clone();
            BlockInfo {
                block_type,
                content,
            }
        })
        .collect();
    Ok(blocks)
}

/// ページの子ブロックのJSONデータをMarkdown形式の文字列に変換する
pub fn to_markdown(page_info: &PageInfo, blocks: &[BlockInfo]) -> Result<String, Box<dyn Error>> {
    let mut markdown = String::new();
    // frontmatter
    markdown.push_str("+++\n");
    markdown.push_str(&format!("title = \"{}\"\n", page_info.title));
    markdown.push_str(&format!("id = \"{}\"\n", page_info.id));
    markdown.push_str(&format!("tags = [\"{}\"]\n", page_info.tags.join("\", \"")));
    markdown.push_str(&format!("created_time = \"{}\"\n", page_info.created_time));
    markdown.push_str(&format!(
        "last_edited_time = \"{}\"\n",
        page_info.last_edited_time
    ));
    markdown.push_str(&format!("status = \"{}\"\n", page_info.status));
    markdown.push_str("+++\n");

    // body
    for block in blocks {
        match block.block_type {
            BlockType::Paragraph => {
                markdown.push_str(paragraph::process(&block.content)?.as_str());
            }
            BlockType::LinkToPage => {
                markdown.push_str(link_to_page::process(&block.content)?.as_str());
            }
            BlockType::Heading1 => {
                markdown.push_str(heading::process(&block.content, 1)?.as_str());
            }
            BlockType::Heading2 => {
                markdown.push_str(heading::process(&block.content, 2)?.as_str());
            }
            BlockType::Heading3 => {
                markdown.push_str(heading::process(&block.content, 3)?.as_str());
            }
            BlockType::Code => {
                markdown.push_str(code::process(&block.content)?.as_str());
            }
            BlockType::BulletedListItem => {
                markdown.push_str(bulleted_list_item::process(&block.content)?.as_str());
            }
            BlockType::NumberedListItem => {
                markdown.push_str(numbered_list_item::process(&block.content)?.as_str());
            }
            BlockType::Unsupported => {
                println!("Unsupported block type");
            }
        }
    }
    Ok(markdown)
}
