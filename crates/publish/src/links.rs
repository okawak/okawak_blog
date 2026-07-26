use regex::Regex;
use std::{collections::HashMap, sync::LazyLock};

/// Published article hrefs indexed by extensionless source paths.
#[derive(Default)]
pub(crate) struct Index {
    routes: HashMap<String, String>,
}

impl Index {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            routes: HashMap::with_capacity(capacity),
        }
    }

    pub(crate) fn insert(&mut self, source_path: String, href: String) {
        self.routes.insert(source_path, href);
    }

    pub(crate) fn resolve(&self, target: &str) -> Option<&str> {
        self.routes.get(target).map(String::as_str).or_else(|| {
            let suffix = format!("/{target}");
            self.routes.iter().find_map(|(source_path, href)| {
                source_path.ends_with(&suffix).then_some(href.as_str())
            })
        })
    }
}

/// Convert Obsidian internal links to published Markdown links.
pub(crate) fn convert(content: &str, index: &Index) -> String {
    static OBSIDIAN_LINK_REGEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("Invalid regex pattern"));

    OBSIDIAN_LINK_REGEX
        .replace_all(content, |captures: &regex::Captures| {
            let link_content = &captures[1];

            let (link_target, display_text) = if let Some(pipe_position) = link_content.find('|') {
                let (link, display) = link_content.split_at(pipe_position);
                (link.trim(), display[1..].trim())
            } else {
                (link_content.trim(), link_content.trim())
            };

            let href = index
                .resolve(link_target)
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    log::warn!("Internal link target '{link_target}' was not found");
                    format!("/{link_target}")
                });

            format!(
                "[{}]({})",
                escape_markdown_link_text(display_text),
                escape_markdown_link_destination(&href)
            )
        })
        .to_string()
}

fn escape_markdown_link_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_markdown_link_destination(destination: &str) -> String {
    destination.replace(')', "\\)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_resolves_exact_and_path_suffix_targets() {
        let mut index = Index::default();
        index.insert(
            "notes/another-note".to_string(),
            "/tech/abc123def".to_string(),
        );
        index.insert("filename".to_string(), "/daily/xyz789abc".to_string());

        assert_eq!(index.resolve("notes/another-note"), Some("/tech/abc123def"));
        assert_eq!(index.resolve("another-note"), Some("/tech/abc123def"));
        assert_eq!(index.resolve("filename"), Some("/daily/xyz789abc"));
        assert_eq!(index.resolve("missing"), None);
    }

    #[test]
    fn convert_internal_links() {
        let mut index = Index::default();
        index.insert(
            "notes/another-note".to_string(),
            "/tech/abc123def".to_string(),
        );
        index.insert("filename".to_string(), "/daily/xyz789abc".to_string());

        assert_eq!(
            convert("Check out [[another-note]] for more info.", &index),
            "Check out [another-note](/tech/abc123def) for more info."
        );
        assert_eq!(
            convert("See [[filename|Custom Display Text]] here.", &index),
            "See [Custom Display Text](/daily/xyz789abc) here."
        );
        assert_eq!(
            convert("Link to [[nonexistent]] file.", &index),
            "Link to [nonexistent](/nonexistent) file."
        );
        assert_eq!(
            convert("This is normal text with no special links.", &index),
            "This is normal text with no special links."
        );
    }

    #[test]
    fn convert_escapes_markdown_link_parts() {
        let mut index = Index::default();
        index.insert("File with <script>".to_string(), "/tech/abc123".to_string());

        assert_eq!(
            convert("[[File with <script>|Display & test]]", &index),
            "[Display & test](/tech/abc123)"
        );
        assert_eq!(
            convert("[[File \"quoted\"|Text with 'quotes']]", &index),
            "[Text with 'quotes'](/File \"quoted\")"
        );
    }
}
