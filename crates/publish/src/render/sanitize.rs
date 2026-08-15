use super::bookmark;
use pulldown_cmark::{CowStr, Event, Tag};

fn sanitize_destination(destination: CowStr<'_>) -> CowStr<'_> {
    if is_safe_destination(&destination) {
        destination
    } else {
        CowStr::Borrowed("#")
    }
}

fn is_safe_destination(destination: &str) -> bool {
    let destination = destination.trim();

    if destination.is_empty() {
        return false;
    }

    if destination.starts_with('#') {
        return true;
    }

    if destination.starts_with('/') {
        return !destination.starts_with("//");
    }

    if destination.starts_with("http://")
        || destination.starts_with("https://")
        || destination.starts_with("mailto:")
    {
        return true;
    }

    !destination.contains(':') && !destination.contains('\\') && !destination.starts_with('.')
}

fn sanitize_destination_event(event: Event<'_>) -> Event<'_> {
    match event {
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: sanitize_destination(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: sanitize_destination(dest_url),
            title,
            id,
        }),
        event => event,
    }
}

/// Sanitizes link destinations and raw HTML before HTML generation.
pub(super) fn events<'a>(parser: impl Iterator<Item = Event<'a>>) -> Vec<Event<'a>> {
    let mut result: Vec<Event<'a>> = Vec::new();
    let mut in_bookmark = false;
    let mut bookmark_buffer = String::new();

    for event in parser {
        match event {
            Event::Html(html) | Event::InlineHtml(html) => {
                if !in_bookmark && !html.trim_start().starts_with(r#"<div class="bookmark">"#) {
                    result.push(Event::Text(html));
                } else {
                    if !in_bookmark {
                        in_bookmark = true;
                    }
                    bookmark_buffer.push_str(&html);

                    if let Some(close) = bookmark_buffer.find("</div>") {
                        in_bookmark = false;
                        let safe_end = close + "</div>".len();
                        let bookmark_part = bookmark_buffer[..safe_end].to_string();
                        let rest = bookmark_buffer[safe_end..].to_string();
                        bookmark_buffer.clear();

                        match bookmark::parse_simple_bookmark(&bookmark_part) {
                            Some(bookmark) => {
                                let href = if is_safe_destination(bookmark.href()) {
                                    bookmark.href()
                                } else {
                                    "#"
                                };
                                result.push(Event::Html(bookmark.with_href(href).into()));
                            }
                            None => result.push(Event::Text(bookmark_part.into())),
                        }

                        if !rest.is_empty() {
                            result.push(Event::Text(rest.into()));
                        }
                    }
                }
            }
            other => {
                if in_bookmark {
                    in_bookmark = false;
                    let buffer = std::mem::take(&mut bookmark_buffer);
                    if !buffer.is_empty() {
                        result.push(Event::Text(buffer.into()));
                    }
                }
                result.push(sanitize_destination_event(other));
            }
        }
    }

    if !bookmark_buffer.is_empty() {
        result.push(Event::Text(bookmark_buffer.into()));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use pulldown_cmark::{Options, Parser, html};
    use rstest::rstest;

    fn sanitize(markdown: &str) -> String {
        let parser = Parser::new_ext(markdown, Options::empty());
        let mut output = String::new();
        html::push_html(&mut output, events(parser).into_iter());
        output
    }

    #[rstest]
    #[case::absolute_url("https://example.com", true)]
    #[case::root_relative_path("/articles/example", true)]
    #[case::relative_path("articles/example", true)]
    #[case::fragment("#section", true)]
    #[case::javascript("javascript:alert(1)", false)]
    #[case::protocol_relative("//example.com", false)]
    #[case::windows_path(r"articles\example", false)]
    fn test_safe_destination(#[case] destination: &str, #[case] expected: bool) {
        assert_eq!(is_safe_destination(destination), expected);
    }

    #[rstest]
    #[case::script("<script>alert('xss')</script>")]
    #[case::custom_div(r#"<div class="custom"><span>hello</span></div>"#)]
    #[case::inline_span("<span>inline</span>")]
    fn test_raw_html_is_escaped(#[case] markdown: &str) {
        let result = sanitize(markdown);

        assert!(result.contains("&lt;"), "unexpected html:\n{result}");
    }

    #[rstest]
    #[case::link("[click](javascript:alert('xss'))", "href=\"#\"")]
    #[case::image("![image](javascript:alert('xss'))", "src=\"#\"")]
    fn test_unsafe_markdown_destination_is_neutralized(
        #[case] markdown: &str,
        #[case] expected: &str,
    ) {
        let result = sanitize(markdown);

        assert!(result.contains(expected), "unexpected html:\n{result}");
        assert!(!result.contains("javascript:alert"));
    }

    #[rstest]
    #[case::single_line(r#"<div class="bookmark"><a href="https://example.com">Example</a></div>"#)]
    #[case::multiline(indoc! {r#"
        <div class="bookmark">
          <a href="https://example.com">Example</a>
        </div>
    "#})]
    fn test_simple_bookmark_is_preserved(#[case] markdown: &str) {
        let result = sanitize(markdown);

        assert!(
            result.contains(r#"<div class="bookmark">"#),
            "unexpected html:\n{result}"
        );
        assert!(!result.contains("&lt;div"));
    }

    #[test]
    fn test_unsafe_bookmark_destination_is_neutralized() {
        let markdown = r#"<div class="bookmark"><a href="javascript:alert(1)">Example</a></div>"#;

        let result = sanitize(markdown);

        assert!(result.contains(r##"<a href="#">"##));
        assert!(!result.contains("javascript:alert"));
    }

    #[rstest]
    #[case::script_sibling(r#"<div class="bookmark"><script>alert('xss')</script></div>"#)]
    #[case::extra_anchor_attribute(
        r#"<div class="bookmark"><a href="https://example.com" onmouseover="alert(1)">Example</a></div>"#
    )]
    #[case::nested_content(
        r#"<div class="bookmark"><a href="https://example.com"><strong>Example</strong></a></div>"#
    )]
    #[case::unclosed("<div class=\"bookmark\">\n<a href=\"https://example.com\">Example</a>")]
    fn test_invalid_bookmark_is_escaped(#[case] markdown: &str) {
        let result = sanitize(markdown);

        assert!(!result.contains(r#"<div class="bookmark">"#));
        assert!(result.contains("&lt;"), "unexpected html:\n{result}");
    }

    #[test]
    fn test_html_after_bookmark_is_escaped() {
        let markdown = r#"<div class="bookmark"><a href="https://example.com">Example</a></div><script>bad()</script>"#;

        let result = sanitize(markdown);

        assert!(result.contains(r#"<div class="bookmark">"#));
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_multiple_bookmarks_are_preserved() {
        let markdown = indoc! {r#"
            <div class="bookmark">
              <a href="https://example.com">Example</a>
            </div>

            Some prose.

            <div class="bookmark">
              <a href="https://rust-lang.org">Rust</a>
            </div>
        "#};

        let result = sanitize(markdown);

        assert_eq!(result.matches(r#"<div class="bookmark">"#).count(), 2);
        assert!(!result.contains("&lt;div"));
    }
}
