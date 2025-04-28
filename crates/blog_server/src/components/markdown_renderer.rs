use crate::error::AppError;
use leptos::prelude::*;
use pulldown_cmark::{Options, Parser, html};

/// Markdownをレンダリングするコンポーネント
#[component]
pub fn MarkdownRenderer(
    #[prop(into)] content: String,
    #[prop(default = false)] enable_toc: bool,
    #[prop(default = true)] enable_mathjax: bool,
) -> impl IntoView {
    // Markdownを処理
    let html_result = render_markdown(&content, enable_toc);

    // 条件に基づいてHTML内容とクラスを設定
    let (content_html, content_class) = match &html_result {
        Ok(html) => {
            let class = if enable_mathjax {
                "markdown-content mathjax"
            } else {
                "markdown-content"
            };
            (html.clone(), class)
        }
        Err(e) => (
            format!(
                "<div class=\"markdown-error\"><p>Markdownの処理中にエラーが発生しました: {}</p></div>",
                e
            ),
            "",
        ),
    };

    view! {
        <div>
            <div class=content_class inner_html=content_html></div>

            // MathJaxは成功時かつenable_mathjaxがtrueの場合のみ表示
            <Show when=move || html_result.is_ok() && enable_mathjax>
                <script type="text/javascript" id="mathjax-config">
                    "window.MathJax = {{
                        tex: {{
                            inlineMath: [['$', '$'], ['\\\\(', '\\\\)']],
                            displayMath: [['$$', '$$'], ['\\\\[', '\\\\]']],
                            processEscapes: true,
                            processEnvironments: true
                        }},
                        svg: {{ fontCache: 'global' }}
                    }};"
                </script>
                <script
                    type="text/javascript"
                    id="mathjax-script"
                    async=true
                    src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"
                ></script>
            </Show>
        </div>
    }
}

/// Markdownをレンダリングする関数
fn render_markdown(markdown: &str, generate_toc: bool) -> Result<String, AppError> {
    // Markdownのオプション設定
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let mut html_output = String::new();

    if generate_toc {
        // 最初に全体をパースして見出しを収集
        let parser_for_toc = Parser::new_ext(markdown, options.clone());
        let events: Vec<_> = parser_for_toc.collect();

        // 目次生成
        html_output.push_str("<div class=\"table-of-contents\">\n<h3>目次</h3>\n<ul>\n");

        let mut headers = Vec::new();
        let mut current_text = String::new();
        let mut current_level = None;

        for event in &events {
            use pulldown_cmark::Event;
            use pulldown_cmark::Tag;
            use pulldown_cmark::TagEnd;

            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    current_level = Some(*level);
                    current_text.clear();
                }
                Event::Text(text) => {
                    if current_level.is_some() {
                        current_text.push_str(text);
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some(level) = current_level {
                        let id = slugify(&current_text);
                        headers.push((level, current_text.clone(), id));
                        current_level = None;
                    }
                }
                _ => {}
            }
        }

        // 目次エントリを作成
        for (level, text, id) in &headers {
            let indent = match level {
                pulldown_cmark::HeadingLevel::H1 => "",
                pulldown_cmark::HeadingLevel::H2 => "  ",
                pulldown_cmark::HeadingLevel::H3 => "    ",
                pulldown_cmark::HeadingLevel::H4 => "      ",
                pulldown_cmark::HeadingLevel::H5 => "        ",
                pulldown_cmark::HeadingLevel::H6 => "          ",
            };

            html_output.push_str(&format!(
                "{}<li><a href=\"#{}\">{}</a></li>\n",
                indent, id, text
            ));
        }

        html_output.push_str("</ul>\n</div>\n");
    }

    // 本文のパースと変換
    let parser = Parser::new_ext(markdown, options);
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    // 見出しにIDを追加
    if generate_toc {
        html_output.push_str(&add_heading_ids(&html_body));
    } else {
        html_output.push_str(&html_body);
    }

    Ok(html_output)
}

/// 見出しにID属性を追加する関数
fn add_heading_ids(html: &str) -> String {
    let re = regex::Regex::new(r"<(h[1-6])>(.*?)</h[1-6]>").unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let tag = &caps[1];
        let content = &caps[2];
        let id = slugify(content);
        format!("<{} id=\"{}\">{}</{}>", tag, id, content, tag)
    })
    .to_string()
}

/// 文字列をURLに適したスラッグに変換する関数
fn slugify(s: &str) -> String {
    let mut slug = s.to_lowercase();
    slug = slug.replace(' ', "-");

    // 英数字、ハイフン、アンダースコア以外の文字を削除
    let re = regex::Regex::new(r"[^a-z0-9\-_]").unwrap();
    slug = re.replace_all(&slug, "").to_string();

    // 連続したハイフンをひとつにまとめる
    let re = regex::Regex::new(r"-+").unwrap();
    slug = re.replace_all(&slug, "-").to_string();

    // 前後のハイフンを削除
    slug = slug
        .trim_start_matches('-')
        .trim_end_matches('-')
        .to_string();

    slug
}
