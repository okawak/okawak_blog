use crate::{ArticlePageDocument, CategoryPageDocument, HomePageDocument, StaticPageDocument};

pub fn build_home_page_title(site_name: &str) -> String {
    site_name.to_string()
}

pub fn build_home_page_description(document: &HomePageDocument) -> String {
    format!(
        "{} published across {}.",
        format_count(document.total_articles, "article", "articles"),
        format_count(document.categories.len(), "category", "categories")
    )
}

pub fn build_article_page_title(document: &ArticlePageDocument, site_name: &str) -> String {
    format!("{} | {}", document.article.title.as_str(), site_name)
}

pub fn build_article_page_description(document: &ArticlePageDocument) -> String {
    document
        .article
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!(
                "An article in the {} category.",
                document.article.category_display_name
            )
        })
}

pub fn build_category_page_title(document: &CategoryPageDocument, site_name: &str) -> String {
    format!("{} | {}", document.title, site_name)
}

pub fn build_category_page_description(document: &CategoryPageDocument) -> String {
    document
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!(
                "{} in the {} category.",
                format_count(document.articles.len(), "article", "articles"),
                document.category_display_name
            )
        })
}

pub fn build_static_page_title(document: &StaticPageDocument, site_name: &str) -> String {
    format!("{} | {}", document.title, site_name)
}

pub fn build_static_page_description(document: &StaticPageDocument) -> String {
    document
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("The {} page.", document.title))
}

fn format_count(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Category, PageKey, SectionPath, SiteArticleCard, SiteCategorySummary, Slug, Title,
    };

    fn article_page(description: Option<&str>) -> ArticlePageDocument {
        ArticlePageDocument {
            article: SiteArticleCard {
                slug: Slug::new("intro00000001".to_string()).unwrap(),
                title: Title::new("Intro".to_string()).unwrap(),
                category: Category::Tech,
                category_display_name: "Technology".to_string(),
                section_path: SectionPath::default(),
                description: description.map(str::to_string),
                tags: vec![],
                priority: None,
                created_at: "2025-01-01T00:00:00+09:00".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            },
            html: "<article><h1>Intro</h1></article>".to_string(),
        }
    }

    fn category_page(description: Option<&str>) -> CategoryPageDocument {
        CategoryPageDocument {
            category: Category::Tech,
            title: "Rust".to_string(),
            category_display_name: "Technology".to_string(),
            description: description.map(str::to_string),
            html: "<article><h1>Rust</h1></article>".to_string(),
            sections: vec![],
            articles: vec![article_page(None).article],
        }
    }

    #[test]
    fn builds_home_page_metadata() {
        let document = HomePageDocument {
            total_articles: 3,
            categories: vec![
                SiteCategorySummary {
                    category: Category::Tech,
                    category_display_name: "Technology".to_string(),
                    article_count: 2,
                },
                SiteCategorySummary {
                    category: Category::Daily,
                    category_display_name: "Daily".to_string(),
                    article_count: 1,
                },
            ],
            articles: vec![],
            fragment: None,
        };

        assert_eq!(build_home_page_title("Example Blog"), "Example Blog");
        assert_eq!(
            build_home_page_description(&document),
            "3 articles published across 2 categories."
        );
    }

    #[test]
    fn builds_article_page_metadata() {
        let document = article_page(Some("summary"));

        assert_eq!(
            build_article_page_title(&document, "Example Blog"),
            "Intro | Example Blog"
        );
        assert_eq!(build_article_page_description(&document), "summary");
    }

    #[test]
    fn article_description_falls_back_when_missing() {
        assert_eq!(
            build_article_page_description(&article_page(None)),
            "An article in the Technology category."
        );
    }

    #[test]
    fn article_description_falls_back_when_blank() {
        assert_eq!(
            build_article_page_description(&article_page(Some("   "))),
            "An article in the Technology category."
        );
    }

    #[test]
    fn builds_category_page_metadata() {
        let document = category_page(Some("Rust articles"));

        assert_eq!(
            build_category_page_title(&document, "Example Blog"),
            "Rust | Example Blog"
        );
        assert_eq!(build_category_page_description(&document), "Rust articles");
    }

    #[test]
    fn category_description_falls_back_when_missing() {
        assert_eq!(
            build_category_page_description(&category_page(None)),
            "1 article in the Technology category."
        );
    }

    #[test]
    fn builds_static_page_metadata() {
        let document = StaticPageDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: Some("About this site".to_string()),
            html: "<article><h1>About</h1></article>".to_string(),
        };

        assert_eq!(
            build_static_page_title(&document, "Example Blog"),
            "About | Example Blog"
        );
        assert_eq!(build_static_page_description(&document), "About this site");
    }
}
