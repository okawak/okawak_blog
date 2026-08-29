use crate::{ArticlePageDocument, CategoryPageDocument, HomePageDocument, StaticPageDocument};

pub fn build_home_page_title(site_name: &str) -> String {
    site_name.to_string()
}

pub fn build_home_page_description(document: &HomePageDocument) -> String {
    format!(
        "{}件の記事を{}カテゴリで公開しています。",
        document.total_articles,
        document.categories.len()
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
                "{}カテゴリの記事です。",
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
                "{}カテゴリの記事一覧です。{}件の記事があります。",
                document.category_display_name,
                document.articles.len()
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
        .unwrap_or_else(|| format!("{} ページです。", document.title))
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
                category_display_name: "技術".to_string(),
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
            category_display_name: "技術".to_string(),
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
                    category_display_name: "技術".to_string(),
                    article_count: 2,
                },
                SiteCategorySummary {
                    category: Category::Daily,
                    category_display_name: "日常".to_string(),
                    article_count: 1,
                },
            ],
            articles: vec![],
            fragment: None,
        };

        assert_eq!(
            build_home_page_title("ぶくせんの探窟メモ"),
            "ぶくせんの探窟メモ"
        );
        assert_eq!(
            build_home_page_description(&document),
            "3件の記事を2カテゴリで公開しています。"
        );
    }

    #[test]
    fn builds_article_page_metadata() {
        let document = article_page(Some("summary"));

        assert_eq!(
            build_article_page_title(&document, "ぶくせんの探窟メモ"),
            "Intro | ぶくせんの探窟メモ"
        );
        assert_eq!(build_article_page_description(&document), "summary");
    }

    #[test]
    fn article_description_falls_back_when_missing() {
        assert_eq!(
            build_article_page_description(&article_page(None)),
            "技術カテゴリの記事です。"
        );
    }

    #[test]
    fn article_description_falls_back_when_blank() {
        assert_eq!(
            build_article_page_description(&article_page(Some("   "))),
            "技術カテゴリの記事です。"
        );
    }

    #[test]
    fn builds_category_page_metadata() {
        let document = category_page(Some("Rust articles"));

        assert_eq!(
            build_category_page_title(&document, "ぶくせんの探窟メモ"),
            "Rust | ぶくせんの探窟メモ"
        );
        assert_eq!(build_category_page_description(&document), "Rust articles");
    }

    #[test]
    fn category_description_falls_back_when_missing() {
        assert_eq!(
            build_category_page_description(&category_page(None)),
            "技術カテゴリの記事一覧です。1件の記事があります。"
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
            build_static_page_title(&document, "ぶくせんの探窟メモ"),
            "About | ぶくせんの探窟メモ"
        );
        assert_eq!(build_static_page_description(&document), "About this site");
    }
}
