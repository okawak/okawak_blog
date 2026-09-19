use crate::i18n::{Message, article_count, category_count, category_name, interpolate, t};
use domain::{
    ArticlePageDocument, CategoryPageDocument, HomePageDocument, Locale, StaticPageDocument,
};

pub(crate) fn build_home_page_title(locale: Locale) -> String {
    t(locale, Message::SiteName).to_string()
}

pub(crate) fn build_home_page_description(document: &HomePageDocument, locale: Locale) -> String {
    interpolate(
        locale,
        Message::MetaHome,
        &[
            ("articles", article_count(locale, document.total_articles)),
            (
                "categories",
                category_count(locale, document.categories.len()),
            ),
        ],
    )
}

pub(crate) fn build_article_page_title(document: &ArticlePageDocument, locale: Locale) -> String {
    format!(
        "{} | {}",
        document.article.title.as_str(),
        t(locale, Message::SiteName)
    )
}

pub(crate) fn build_article_page_description(
    document: &ArticlePageDocument,
    locale: Locale,
) -> String {
    document
        .article
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            interpolate(
                locale,
                Message::MetaArticle,
                &[(
                    "category",
                    category_name(locale, document.article.category).into(),
                )],
            )
        })
}

pub(crate) fn build_category_page_title(document: &CategoryPageDocument, locale: Locale) -> String {
    format!("{} | {}", document.title, t(locale, Message::SiteName))
}

pub(crate) fn build_category_page_description(
    document: &CategoryPageDocument,
    locale: Locale,
) -> String {
    document
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            interpolate(
                locale,
                Message::MetaCategory,
                &[
                    ("articles", article_count(locale, document.articles.len())),
                    ("category", category_name(locale, document.category).into()),
                ],
            )
        })
}

pub(crate) fn build_static_page_title(document: &StaticPageDocument, locale: Locale) -> String {
    format!("{} | {}", document.title, t(locale, Message::SiteName))
}

pub(crate) fn build_static_page_description(
    document: &StaticPageDocument,
    locale: Locale,
) -> String {
    document
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            interpolate(
                locale,
                Message::MetaPage,
                &[("title", document.title.clone())],
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{
        Category, PageKey, SectionPath, SiteArticleCard, SiteCategorySummary, Slug, Title,
    };

    fn article_page(description: Option<&str>) -> ArticlePageDocument {
        ArticlePageDocument {
            article: SiteArticleCard {
                slug: Slug::new("intro00000001".to_string()).unwrap(),
                title: Title::new("Intro".to_string()).unwrap(),
                category: Category::Tech,
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
                    article_count: 2,
                },
                SiteCategorySummary {
                    category: Category::Daily,
                    article_count: 1,
                },
            ],
            articles: vec![],
            fragment: None,
        };

        assert_eq!(
            build_home_page_title(Locale::En),
            t(Locale::En, Message::SiteName)
        );
        assert_eq!(
            build_home_page_description(&document, Locale::En),
            "3 articles published across 2 categories."
        );
    }

    #[test]
    fn builds_article_page_metadata() {
        let document = article_page(Some("summary"));

        assert_eq!(
            build_article_page_title(&document, Locale::En),
            format!("Intro | {}", t(Locale::En, Message::SiteName))
        );
        assert_eq!(
            build_article_page_description(&document, Locale::En),
            "summary"
        );
    }

    #[test]
    fn article_description_falls_back_when_missing() {
        assert_eq!(
            build_article_page_description(&article_page(None), Locale::En),
            "An article in the Technology category."
        );
    }

    #[test]
    fn article_description_falls_back_when_blank() {
        assert_eq!(
            build_article_page_description(&article_page(Some("   ")), Locale::En),
            "An article in the Technology category."
        );
    }

    #[test]
    fn builds_category_page_metadata() {
        let document = category_page(Some("Rust articles"));

        assert_eq!(
            build_category_page_title(&document, Locale::En),
            format!("Rust | {}", t(Locale::En, Message::SiteName))
        );
        assert_eq!(
            build_category_page_description(&document, Locale::En),
            "Rust articles"
        );
    }

    #[test]
    fn category_description_falls_back_when_missing() {
        assert_eq!(
            build_category_page_description(&category_page(None), Locale::En),
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
            build_static_page_title(&document, Locale::En),
            format!("About | {}", t(Locale::En, Message::SiteName))
        );
        assert_eq!(
            build_static_page_description(&document, Locale::En),
            "About this site"
        );
    }
}
