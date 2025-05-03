use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    let mut list_item = String::new();
    list_item.push_str("- ");
    for element in value.get("rich_text").unwrap().as_array().unwrap() {
        if let Some(text) = element.get("text") {
            // 装飾の取得
            let annotations = element.get("annotations").unwrap();
            let is_bold = annotations.get("bold").unwrap().as_bool().unwrap();
            let is_italic = annotations.get("italic").unwrap().as_bool().unwrap();
            let is_underline = annotations.get("underline").unwrap().as_bool().unwrap();
            let is_strikethrough = annotations.get("strikethrough").unwrap().as_bool().unwrap();
            let is_code = annotations.get("code").unwrap().as_bool().unwrap();
            // 色はまだ対応していない
            let _is_color = annotations.get("color").unwrap().as_str().unwrap();

            let mut content = text.get("content").unwrap().to_string();
            content = content.trim_matches('"').to_string();
            // 装飾を適用
            if is_bold {
                content = format!(" **{content}** ");
            }
            if is_italic {
                content = format!(" _{content}_ ");
            }
            if is_underline {
                content = format!("<u>{content}</u>");
            }
            if is_strikethrough {
                content = format!(" ~~{content}~~ ");
            }
            if is_code {
                content = format!("`{content}`");
            }

            let link = text.get("link").unwrap();
            if let Some(raw_url) = link.get("url") {
                let tmp_url = raw_url.to_string();
                let url = tmp_url.trim_matches('"');
                list_item.push_str(&format!("[{content}]({url})"));
            } else {
                list_item.push_str(&content);
            }
        } else if let Some(equation) = element.get("equation") {
            let expression = equation.get("expression").unwrap().to_string();
            // 装飾も可能だが、数式には適用しない
            // let annotations = element.get("annotations").unwrap();
            // let is_bold = annotations.get("bold").unwrap().as_bool().unwrap();
            // let is_italic = annotations.get("italic").unwrap().as_bool().unwrap();
            // let is_underline = annotations.get("underline").unwrap().as_bool().unwrap();
            // let is_strikethrough = annotations.get("strikethrough").unwrap().as_bool().unwrap();
            // let is_code = annotations.get("code").unwrap().as_bool().unwrap();
            // let _is_color = annotations.get("color").unwrap().as_str().unwrap();

            let mut content = expression.trim_matches('"').to_string();
            content = content.replace("\\\\", "\\");
            content = format!("${content}$");
            // 装飾を適用
            // if is_bold {
            //     content = format!("**{}**", content);
            // }
            // if is_italic {
            //     content = format!("_{}_", content);
            // }
            // if is_underline {
            //     content = format!("<u>{}</u>", content);
            // }
            // if is_strikethrough {
            //     content = format!("~~{}~~", content);
            // }
            // if is_code {
            //     content = format!("`{}`", content);
            // }

            list_item.push_str(&content);
        }
    }
    Ok(format!("{list_item}\n\n").to_string())
}
