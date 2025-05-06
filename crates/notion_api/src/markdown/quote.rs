use crate::error::Result;
use serde_json::Value;

pub fn process(value: &Value) -> Result<String> {
    let mut quote = String::new();
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
                quote.push_str(&format!("[{content}]({url})"));
            } else {
                quote.push_str(&content);
            }
        } else if let Some(equation) = element.get("equation") {
            let expression = equation.get("expression").unwrap().to_string();
            let annotations = element.get("annotations").unwrap();
            let is_bold = annotations.get("bold").unwrap().as_bool().unwrap();
            let is_italic = annotations.get("italic").unwrap().as_bool().unwrap();
            let is_underline = annotations.get("underline").unwrap().as_bool().unwrap();
            let is_strikethrough = annotations.get("strikethrough").unwrap().as_bool().unwrap();
            let is_code = annotations.get("code").unwrap().as_bool().unwrap();
            let _is_color = annotations.get("color").unwrap().as_str().unwrap();

            let mut content = expression.trim_matches('"').to_string();
            content = content.replace("\\\\", "\\");
            content = format!("${content}$");
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

            quote.push_str(&content);
        }
    }
    Ok(format!("> {quote}\n\n"))
}
