mod bookmark;
mod bulleted_list_item;
mod code;
mod divider;
mod equation;
mod heading;
mod image;
mod link_to_page;
mod numbered_list_item;
mod paragraph;
mod quote;

use crate::error::{NotionError, Result};
use crate::models::{BlockInfo, BlockType, PageInfo};
use serde_json::Value;

pub fn extract_blocks(json: &Value) -> Result<Vec<BlockInfo>> {
    let results = json
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or(NotionError::DataError(format!(
            "No results array: {json:?}"
        )))?;
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
                "image" => BlockType::Image,
                "bulleted_list_item" => BlockType::BulletedListItem,
                "numbered_list_item" => BlockType::NumberedListItem,
                "quote" => BlockType::Quote,
                "equation" => BlockType::Equation,
                "bookmark" => BlockType::BookMark,
                "divider" => BlockType::Divider,
                _ => BlockType::Unsupported(block_type_str.to_string()),
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
pub fn to_markdown(page_info: &PageInfo, blocks: &[BlockInfo]) -> Result<String> {
    // frontmatter
    let mut markdown = format!(
        "+++\n\
      id = \"{}\"\n\
      title = \"{}\"\n\
      category = \"{}\"\n\
      group = \"{}\"\n\
      priority_level = {}\n\
      tags = [\"{}\"]\n\
      summary = \"{}\"\n\
      created_time = \"{}\"\n\
      last_edited_time = \"{}\"\n\
      status = \"{}\"\n\
      +++\n",
        page_info.id,
        page_info.title,
        page_info.category,
        page_info.group,
        page_info.priority_level,
        page_info.tags.join("\", \""),
        page_info.summary,
        page_info.created_time,
        page_info.last_edited_time,
        page_info.status
    );

    // body
    for block in blocks {
        match &block.block_type {
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
            BlockType::Image => {
                markdown.push_str(image::process(&block.content)?.as_str());
            }
            BlockType::BulletedListItem => {
                markdown.push_str(bulleted_list_item::process(&block.content)?.as_str());
            }
            BlockType::Quote => {
                markdown.push_str(quote::process(&block.content)?.as_str());
            }
            BlockType::NumberedListItem => {
                markdown.push_str(numbered_list_item::process(&block.content)?.as_str());
            }
            BlockType::Equation => {
                markdown.push_str(equation::process(&block.content)?.as_str());
            }
            BlockType::BookMark => {
                markdown.push_str(bookmark::process(&block.content)?.as_str());
            }
            BlockType::Divider => {
                markdown.push_str(divider::process(&block.content)?.as_str());
            }
            BlockType::Unsupported(type_name) => {
                println!("Unsupported block type: {type_name}");
            }
        }
    }
    Ok(markdown)
}
