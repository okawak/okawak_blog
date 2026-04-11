use domain::{
    ArticleIndexDocument, ArticleSummaryDocument, CategoryIndexDocument, CategoryMetadataDocument,
    SiteMetadataDocument,
};
use std::{fs, path::Path};

pub(crate) fn write_fixture_site(root: &Path) {
    fs::create_dir_all(root.join("articles")).unwrap();
    fs::create_dir_all(root.join("categories")).unwrap();
    fs::create_dir_all(root.join("metadata")).unwrap();

    fs::write(
        root.join("articles/index.json"),
        serde_json::to_string_pretty(&ArticleIndexDocument {
            articles: vec![ArticleSummaryDocument {
                slug: "sample0000001".to_string(),
                title: "Sample".to_string(),
                category: "tech".to_string(),
                section_path: vec!["block".to_string()],
                description: Some("summary".to_string()),
                tags: vec!["rust".to_string()],
                priority: Some(1),
                created_at: "2025-01-01T00:00:00+09:00".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            }],
        })
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("categories/tech.json"),
        serde_json::to_string_pretty(&CategoryIndexDocument {
            category: "tech".to_string(),
            articles: vec![ArticleSummaryDocument {
                slug: "sample0000001".to_string(),
                title: "Sample".to_string(),
                category: "tech".to_string(),
                section_path: vec!["block".to_string()],
                description: Some("summary".to_string()),
                tags: vec!["rust".to_string()],
                priority: Some(1),
                created_at: "2025-01-01T00:00:00+09:00".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            }],
        })
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("metadata/site.json"),
        serde_json::to_string_pretty(&SiteMetadataDocument {
            total_articles: 1,
            categories: vec![CategoryMetadataDocument {
                category: "tech".to_string(),
                article_count: 1,
            }],
        })
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("articles/sample0000001.html"),
        "<article><h1>Sample</h1></article>",
    )
    .unwrap();
}
