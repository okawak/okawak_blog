#[cfg(feature = "ssr")]
use crate::components::bookmark;
use crate::error::FrontendError;
use leptos::html::Div;
use leptos::prelude::*;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use std::sync::LazyLock;
use stylance::import_style;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

import_style!(markdown_style, "markdown_renderer.module.scss");

// $$ ... $$ を \[ ... \]
static RE_DISPLAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\$\$(.+?)\$\$").expect("Failed to compile display-math regex")
});

// $ ... $ を \( ... \)
static RE_INLINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\$(.+?)\$").expect("Failed to compile inline-math regex"));

// window.renderMathInElement を呼ぶためのバインディング
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = renderMathInElement)]
    fn render_math_in_element(el: &web_sys::Element, options: JsValue);
}

#[server]
pub async fn rewrite_bookmarks(html: String) -> Result<String, ServerFnError> {
    // OGP カードに差し替え
    bookmark::rewrite_bookmarks(html)
        .await
        .map_err(|e| ServerFnError::Serialization(e.to_string()))
}

/// Markdownをレンダリングするコンポーネント
#[component]
pub fn MarkdownRenderer(
    #[prop(into)] content: String,
    #[prop(default = false)] enable_toc: bool,
) -> impl IntoView {
    // Markdownを処理
    let (class_name, initial_html) = match render_markdown(&content, enable_toc) {
        Ok(html) => (markdown_style::markdown, html),
        Err(_) => (
            markdown_style::error,
            "Markdownのレンダリングに失敗しました。".to_string(),
        ),
    };

    let container_ref: NodeRef<Div> = NodeRef::new();
    #[cfg(target_arch = "wasm32")]
    {
        use leptos_router::hooks::use_location;
        let location = use_location();

        // `location.pathname.get()` を読みにいくと、「パスが変わるたび」に以下クロージャが起動
        Effect::new(move |_| {
            let _ = location.pathname.get();

            if let Some(el) = container_ref.get() {
                let opts = serde_wasm_bindgen::to_value(&serde_json::json!({
                    "delimiters": [
                        { "left": "$$", "right": "$$", "display": true },
                        { "left": r"\[", "right": r"\]", "display": true},
                        { "left": "$", "right": "$", "display": false },
                        { "left": r"\(", "right": r"\)", "display": false},
                    ]
                }))
                .expect("failed to serialize KaTeX options");
                render_math_in_element(&el, opts);
            }
        });
    }

    let html_resource = Resource::new(
        || (),
        move |_| {
            let value = initial_html.clone();
            async move {
                // OGP カードに差し替え
                rewrite_bookmarks(value.clone())
                    .await
                    .map_err(|e| e.to_string())
            }
        },
    );

    view! {
        <div class=class_name node_ref=container_ref>
            <Suspense fallback=|| {
                view! { <div class=markdown_style::loading>"記事を読み込み中..."</div> }
            }>
                <ErrorBoundary fallback=|error| {
                    view! {
                        <div class=markdown_style::error>
                            "記事の読み込みに失敗しました: " {format!("{error:?}")}
                        </div>
                    }
                }>
                    <Show
                        when=move || {
                            matches!(html_resource.get(), Some(Ok(html)) if !html.is_empty())
                        }
                        fallback=|| {
                            view! {
                                <div class=markdown_style::no_articles>
                                    "記事がありません"
                                </div>
                            }
                        }
                    >
                        {move || {
                            html_resource
                                .get()
                                .and_then(Result::ok)
                                .map(|html| {
                                    view! {
                                        <div
                                            class=class_name
                                            node_ref=container_ref
                                            inner_html=html
                                        ></div>
                                    }
                                })
                        }}
                    </Show>
                </ErrorBoundary>
            </Suspense>
        </div>
    }
}

fn render_option() -> Options {
    // 詳しい情報は https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES); // 表を有効
    options.insert(Options::ENABLE_FOOTNOTES); // 脚注を有効
    options.insert(Options::ENABLE_STRIKETHROUGH); // 打ち消し線を有効
    options.insert(Options::ENABLE_TASKLISTS); // タスクリストを有効
    options.insert(Options::ENABLE_SMART_PUNCTUATION); // スマート引用符を有効
    options
}

fn markdown_to_html(input: &str, options: Options) -> String {
    let parser = Parser::new_ext(input, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Markdownをレンダリングする関数
fn render_markdown(markdown: &str, generate_toc: bool) -> Result<String, FrontendError> {
    // Markdownのオプション設定
    let options = render_option();

    let mut html_output = String::new();

    if generate_toc {
        // 最初に全体をパースして見出しを収集
        let parser_for_toc = Parser::new_ext(markdown, options);
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

            html_output.push_str(&format!("{indent}<li><a href=\"#{id}\">{text}</a></li>\n"));
        }

        html_output.push_str("</ul>\n</div>\n");
    }

    let html_body = markdown_to_html(markdown, options);

    // 見出しにIDを追加
    if generate_toc {
        html_output.push_str(&add_heading_ids(&html_body));
    } else {
        html_output.push_str(&html_body);
    }

    // 数式の処理
    html_output = post_process_math(&html_output);

    Ok(html_output)
}

fn post_process_math(html: &str) -> String {
    // $$ ... $$ を \[ ... \]
    let tmp = RE_DISPLAY.replace_all(html, r"\[$1\]").into_owned();
    // $ ... $ を \( ... \)
    RE_INLINE.replace_all(&tmp, r"\($1\)").into_owned()
}

/// 見出しにID属性を追加する関数
fn add_heading_ids(html: &str) -> String {
    let re = regex::Regex::new(r"<(h[1-6])>(.*?)</h[1-6]>").unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let tag = &caps[1];
        let content = &caps[2];
        let id = slugify(content);
        format!("<{tag} id=\"{id}\">{content}</{tag}>")
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_markdown_test() {
        let test_cases = [
            // text
            ("hello world", "<p>hello world</p>\n"),
            // h1
            ("# Heading 1", "<h1>Heading 1</h1>\n"),
            // h2
            ("## Heading 2", "<h2>Heading 2</h2>\n"),
            // h3
            ("### Heading 3", "<h3>Heading 3</h3>\n"),
            // bullet points
            (
                "- Item 1\n\n- Item 2",
                "<ul>\n<li>\n<p>Item 1</p>\n</li>\n<li>\n<p>Item 2</p>\n</li>\n</ul>\n",
            ),
            // number list
            (
                "1. Item 1\n\n2. Item 2",
                "<ol>\n<li>\n<p>Item 1</p>\n</li>\n<li>\n<p>Item 2</p>\n</li>\n</ol>\n",
            ),
            // inline code
            ("`inline code`", "<p><code>inline code</code></p>\n"),
            // code block
            (
                "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```",
                "<pre><code class=\"language-rust\">fn main() {\n    println!(\"Hello, world!\");\n}\n</code></pre>\n",
            ),
            // bold
            ("**bold text**", "<p><strong>bold text</strong></p>\n"),
            // table
            (
                "table\n| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |",
                "<p>table</p>\n<table><thead><tr><th>Header 1</th><th>Header 2</th></tr></thead><tbody>\n<tr><td>Cell 1</td><td>Cell 2</td></tr>\n</tbody></table>\n",
            ),
            // footnote
            (
                "This is a footnote[^1].\n\n[^1]: This is the footnote text.",
                "<p>This is a footnote<sup class=\"footnote-reference\"><a href=\"#1\">1</a></sup>.</p>\n<div class=\"footnote-definition\" id=\"1\"><sup class=\"footnote-definition-label\">1</sup>\n<p>This is the footnote text.</p>\n</div>\n",
            ),
            // strikethrough
            (
                "This is ~~strikethrough~~ text.",
                "<p>This is <del>strikethrough</del> text.</p>\n",
            ),
            // task list
            (
                "- [x] Task 1\n- [ ] Task 2",
                "<ul>\n<li><input disabled=\"\" type=\"checkbox\" checked=\"\"/>\nTask 1</li>\n<li><input disabled=\"\" type=\"checkbox\"/>\nTask 2</li>\n</ul>\n",
            ),
            // smart punctuation
            ("\"Hello\" -- world...", "<p>“Hello” – world…</p>\n"),
            // math inline (no change for KaTeX)
            ("$E = mc^2$", "<p>$E = mc^2$</p>\n"),
            // math block (no change for KaTeX)
            ("$$E = mc^2$$", "<p>$$E = mc^2$$</p>\n"),
            // bold and math inline
            (
                "**fomula is **$E = mc^2$** .**",
                "<p><strong>fomula is <strong>$E = mc^2$</strong> .</strong></p>\n",
            ),
            (
                r"**$p(x|\theta)$**",
                "<p><strong>$p(x|\\theta)$</strong></p>\n",
            ),
        ];

        let options = render_option();

        for (input, expected) in test_cases {
            let result = markdown_to_html(input, options);
            assert_eq!(result, expected);
        }
    }
}
