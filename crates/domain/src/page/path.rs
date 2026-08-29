use crate::{ArticlePageDocument, Category, CategoryPageDocument, Slug, StaticPageDocument};

pub fn build_home_page_canonical_path() -> &'static str {
    "/"
}

pub fn build_category_path(category: &Category) -> String {
    format!("/{}", category.as_str())
}

pub fn build_article_path(category: &Category, slug: &Slug) -> String {
    format!("{}/{}", build_category_path(category), slug.as_str())
}

pub fn build_article_page_canonical_path(document: &ArticlePageDocument) -> String {
    build_article_path(&document.article.category, &document.article.slug)
}

pub fn build_category_page_canonical_path(document: &CategoryPageDocument) -> String {
    build_category_path(&document.category)
}

pub fn build_static_page_canonical_path(document: &StaticPageDocument) -> String {
    format!("/{}", document.page.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PageKey, SectionPath, SiteArticleCard, Title};

    fn article_page() -> ArticlePageDocument {
        ArticlePageDocument {
            article: SiteArticleCard {
                slug: Slug::new("intro00000001".to_string()).unwrap(),
                title: Title::new("Intro".to_string()).unwrap(),
                category: Category::Tech,
                category_display_name: "Technology".to_string(),
                section_path: SectionPath::default(),
                description: None,
                tags: vec![],
                priority: None,
                created_at: "2025-01-01T00:00:00+09:00".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            },
            html: "<article><h1>Intro</h1></article>".to_string(),
        }
    }

    #[test]
    fn builds_home_canonical_path() {
        assert_eq!(build_home_page_canonical_path(), "/");
    }

    #[test]
    fn builds_category_and_article_paths() {
        let slug = Slug::new("intro00000001".to_string()).unwrap();

        assert_eq!(build_category_path(&Category::Tech), "/tech");
        assert_eq!(
            build_article_path(&Category::Tech, &slug),
            "/tech/intro00000001"
        );
    }

    #[test]
    fn builds_page_canonical_paths() {
        let article = article_page();
        let category = CategoryPageDocument {
            category: Category::Tech,
            title: "Tech".to_string(),
            category_display_name: "Technology".to_string(),
            description: None,
            html: "<article><h1>Tech</h1></article>".to_string(),
            sections: vec![],
            articles: vec![],
        };
        let static_page = StaticPageDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: None,
            html: "<article><h1>About</h1></article>".to_string(),
        };

        assert_eq!(
            build_article_page_canonical_path(&article),
            "/tech/intro00000001"
        );
        assert_eq!(build_category_page_canonical_path(&category), "/tech");
        assert_eq!(build_static_page_canonical_path(&static_page), "/about");
    }
}
