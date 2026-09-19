//! Presentation-only filtering of published category cards.

use domain::CategorySectionGroup;

pub(crate) const MAX_QUERY_CHARS: usize = 100;

pub(crate) fn filter_sections(
    sections: &mut Vec<CategorySectionGroup>,
    query: &str,
    labels: &domain::TagLabels,
) {
    // Signals come from the browser; enforce the input bound on the server as well.
    let query = query.chars().take(MAX_QUERY_CHARS).collect::<String>();
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return;
    }
    sections.retain_mut(|section| {
        section.articles.retain(|article| {
            article.title.as_str().to_lowercase().contains(&query)
                || article
                    .description
                    .as_deref()
                    .is_some_and(|description| description.to_lowercase().contains(&query))
                || article.tags.iter().any(|tag| {
                    labels
                        .get(tag)
                        .unwrap_or(tag)
                        .to_lowercase()
                        .contains(&query)
                })
        });
        !section.articles.is_empty()
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{SectionPath, SiteArticleCard};

    fn sections() -> Vec<CategorySectionGroup> {
        let card = |slug: &str, title: &str, description: Option<&str>, tags: &[&str]| {
            serde_json::from_value::<SiteArticleCard>(serde_json::json!({
                "slug": slug, "title": title, "category": "tech",
                "section_path": [],
                "description": description, "tags": tags, "priority": null,
                "created_at": "2026-01-01T00:00:00+09:00",
                "updated_at": "2026-01-01T00:00:00+09:00"
            }))
            .unwrap()
        };
        vec![
            CategorySectionGroup {
                section_path: SectionPath::default(),
                heading: "First".to_string(),
                articles: vec![card(
                    "rust",
                    "Rust入門",
                    Some("非同期処理"),
                    &["Programming"],
                )],
            },
            CategorySectionGroup {
                section_path: SectionPath::default(),
                heading: "Second".to_string(),
                articles: vec![card("web", "Web", None, &[])],
            },
        ]
    }

    #[test]
    fn matches_title_description_and_tags_without_case_or_surrounding_whitespace() {
        for query in [" rust ", "非同期", "PROGRAMMING"] {
            let mut actual = sections();
            filter_sections(&mut actual, query, &Default::default());
            assert_eq!(actual, vec![sections().remove(0)], "{query}");
        }
    }

    #[test]
    fn blank_query_preserves_all_cards_and_order() {
        let mut actual = sections();
        filter_sections(&mut actual, "　 \n ", &Default::default());
        assert_eq!(actual, sections());
    }

    #[test]
    fn unmatched_query_removes_empty_groups() {
        let mut actual = sections();
        filter_sections(&mut actual, "missing", &Default::default());
        assert!(actual.is_empty());
    }

    #[test]
    fn bounds_untrusted_query_by_unicode_characters() {
        let mut actual = sections();
        actual[0].articles[0].tags = vec!["あ".repeat(MAX_QUERY_CHARS)];
        filter_sections(
            &mut actual,
            &format!("{}ignored", "あ".repeat(MAX_QUERY_CHARS)),
            &Default::default(),
        );
        assert_eq!(actual.len(), 1);
    }
}
