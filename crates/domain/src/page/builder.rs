use super::{
    ArticlePageDocument, CategoryPageDocument, CategorySectionGroup, HomeFragmentDocument,
    HomePageDocument, SiteArticleCard, SiteCategorySummary, StaticPageDocument,
};
use crate::{
    ArticleIndexDocument, ArticleSummaryDocument, Category, CategoryArtifactDocument, DomainError,
    HomeFragmentArtifactDocument, PageArtifactDocument, PublishedArticleSummary, Result,
    SectionPath, SiteMetadataDocument, Slug,
};
use std::str::FromStr;

impl TryFrom<&ArticleSummaryDocument> for SiteArticleCard {
    type Error = DomainError;

    fn try_from(document: &ArticleSummaryDocument) -> Result<Self> {
        let summary = PublishedArticleSummary::try_from(document)?;
        let category = summary.category;

        Ok(Self {
            slug: summary.slug,
            title: summary.title,
            category,
            category_display_name: category.display_name().to_string(),
            section_path: summary.section_path,
            description: summary.description,
            tags: summary.tags,
            priority: summary.priority,
            created_at: summary.created_at.to_string(),
            updated_at: summary.updated_at.to_string(),
        })
    }
}

pub fn build_home_page_document(
    article_index: &ArticleIndexDocument,
    site_metadata: &SiteMetadataDocument,
    home_fragment: Option<&HomeFragmentArtifactDocument>,
) -> Result<HomePageDocument> {
    let articles = article_index
        .articles
        .iter()
        .map(SiteArticleCard::try_from)
        .collect::<Result<Vec<_>>>()?;
    let site_metadata = crate::SiteMetadata::try_from(site_metadata)?;
    let categories = site_metadata
        .categories
        .into_iter()
        .map(|category| {
            let category_name = category.category;
            SiteCategorySummary {
                category: category_name,
                category_display_name: category_name.display_name().to_string(),
                article_count: category.article_count,
            }
        })
        .collect();

    Ok(HomePageDocument {
        total_articles: site_metadata.total_articles,
        categories,
        articles,
        fragment: home_fragment
            .map(build_home_fragment_document)
            .transpose()?,
    })
}

fn build_home_fragment_document(
    artifact: &HomeFragmentArtifactDocument,
) -> Result<HomeFragmentDocument> {
    artifact.validate()?;

    Ok(HomeFragmentDocument {
        title: artifact.title.trim().to_string(),
        description: artifact.description.clone(),
        html: artifact.html.trim().to_string(),
    })
}

pub fn build_article_page_document(
    summary: &ArticleSummaryDocument,
    html: &str,
) -> Result<ArticlePageDocument> {
    if html.trim().is_empty() {
        return Err(DomainError::validation("html"));
    }

    Ok(ArticlePageDocument {
        article: SiteArticleCard::try_from(summary)?,
        html: html.to_string(),
    })
}

pub fn build_category_page_document(
    artifact: &CategoryArtifactDocument,
) -> Result<CategoryPageDocument> {
    artifact.validate_landing()?;
    let category = Category::from_str(&artifact.category)?;

    let articles = artifact
        .articles
        .iter()
        .map(SiteArticleCard::try_from)
        .collect::<Result<Vec<_>>>()?;
    let sections = build_category_section_groups(&articles);

    Ok(CategoryPageDocument {
        category,
        title: artifact.title.trim().to_string(),
        category_display_name: category.display_name().to_string(),
        description: artifact.description.clone(),
        html: artifact.html.clone(),
        sections,
        articles,
    })
}

pub fn build_static_page_document(artifact: &PageArtifactDocument) -> Result<StaticPageDocument> {
    artifact.validate()?;

    Ok(StaticPageDocument {
        page: artifact.page.clone(),
        title: artifact.title.trim().to_string(),
        description: artifact.description.clone(),
        html: artifact.html.clone(),
    })
}

pub fn find_article_summary<'a>(
    article_index: &'a ArticleIndexDocument,
    category: &Category,
    slug: &Slug,
) -> Option<&'a ArticleSummaryDocument> {
    article_index
        .articles
        .iter()
        .find(|article| article.slug == slug.as_str() && article.category == category.as_str())
}

fn build_category_section_groups(articles: &[SiteArticleCard]) -> Vec<CategorySectionGroup> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<SectionPath, Vec<SiteArticleCard>> = BTreeMap::new();
    for article in articles {
        grouped
            .entry(article.section_path.clone())
            .or_default()
            .push(article.clone());
    }

    grouped
        .into_iter()
        .map(|(section_path, articles)| CategorySectionGroup {
            heading: build_section_heading(&section_path),
            section_path,
            articles,
        })
        .collect()
}

fn build_section_heading(section_path: &SectionPath) -> String {
    if section_path.is_empty() {
        "General".to_string()
    } else {
        section_path.segments().join(" / ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CategoryMetadataDocument, PageKey};

    fn sample_summary() -> ArticleSummaryDocument {
        ArticleSummaryDocument {
            slug: "intro00000001".to_string(),
            title: "Intro".to_string(),
            category: "tech".to_string(),
            section_path: SectionPath::new(vec!["block".to_string()]),
            description: Some("summary".to_string()),
            tags: vec!["rust".to_string()],
            priority: Some(10),
            created_at: "2025-01-01T00:00:00+09:00".to_string(),
            updated_at: "2025-01-02T00:00:00+09:00".to_string(),
        }
    }

    fn category_artifact() -> CategoryArtifactDocument {
        CategoryArtifactDocument {
            category: "tech".to_string(),
            title: "Tech".to_string(),
            description: None,
            html: "<article><h1>Tech</h1></article>".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            articles: vec![sample_summary()],
        }
    }

    #[test]
    fn builds_site_article_card() {
        let card = SiteArticleCard::try_from(&sample_summary()).unwrap();

        assert_eq!(card.slug.as_str(), "intro00000001");
        assert_eq!(card.title.as_str(), "Intro");
        assert_eq!(card.category, Category::Tech);
        assert_eq!(card.category_display_name, "Technology");
    }

    #[test]
    fn builds_home_page_document() {
        let document = build_home_page_document(
            &ArticleIndexDocument {
                articles: vec![sample_summary()],
            },
            &SiteMetadataDocument {
                total_articles: 1,
                categories: vec![CategoryMetadataDocument {
                    category: "tech".to_string(),
                    article_count: 1,
                }],
            },
            None,
        )
        .unwrap();

        assert_eq!(document.total_articles, 1);
        assert_eq!(document.categories.len(), 1);
        assert_eq!(document.categories[0].category_display_name, "Technology");
        assert_eq!(document.articles[0].title.as_str(), "Intro");
        assert_eq!(document.fragment, None);
    }

    #[test]
    fn builds_home_page_document_with_fragment() {
        let fragment = HomeFragmentArtifactDocument {
            title: "Home".to_string(),
            description: Some("Home fragment".to_string()),
            html: "<p>Welcome</p>".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
        };
        let document = build_home_page_document(
            &ArticleIndexDocument {
                articles: vec![sample_summary()],
            },
            &SiteMetadataDocument {
                total_articles: 1,
                categories: vec![CategoryMetadataDocument {
                    category: "tech".to_string(),
                    article_count: 1,
                }],
            },
            Some(&fragment),
        )
        .unwrap();

        assert_eq!(document.fragment.as_ref().unwrap().title, "Home");
        assert!(document.fragment.as_ref().unwrap().html.contains("Welcome"));
    }

    #[test]
    fn builds_article_page_document() {
        let document =
            build_article_page_document(&sample_summary(), "<article><h1>Intro</h1></article>")
                .unwrap();

        assert_eq!(document.article.slug.as_str(), "intro00000001");
        assert!(document.html.contains("<h1>Intro</h1>"));
    }

    #[test]
    fn rejects_blank_article_html() {
        let result = build_article_page_document(&sample_summary(), "   ");

        assert_eq!(result, Err(DomainError::validation("html")));
    }

    #[test]
    fn builds_category_page_document() {
        let document = build_category_page_document(&CategoryArtifactDocument {
            category: "daily".to_string(),
            title: "Daily Notes".to_string(),
            description: Some("Daily landing".to_string()),
            html: "<article><h1>Daily Notes</h1></article>".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            articles: vec![ArticleSummaryDocument {
                category: "daily".to_string(),
                ..sample_summary()
            }],
        })
        .unwrap();

        assert_eq!(document.category, Category::Daily);
        assert_eq!(document.title, "Daily Notes");
        assert_eq!(document.category_display_name, "Daily");
        assert_eq!(document.description, Some("Daily landing".to_string()));
        assert!(document.html.contains("Daily Notes"));
        assert_eq!(document.articles.len(), 1);
        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].heading, "block");
    }

    #[test]
    fn builds_static_page_document() {
        let document = build_static_page_document(&PageArtifactDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: Some("About this site".to_string()),
            html: "<article><h1>About</h1></article>".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
        })
        .unwrap();

        assert_eq!(document.page.as_str(), "about");
        assert_eq!(document.title, "About");
        assert!(document.html.contains("<h1>About</h1>"));
    }

    #[test]
    fn rejects_blank_static_page_html() {
        let result = build_static_page_document(&PageArtifactDocument {
            page: PageKey::new("about".to_string()).unwrap(),
            title: "About".to_string(),
            description: None,
            html: "   ".to_string(),
            updated_at: "2025-01-01T00:00:00+09:00".to_string(),
        });

        assert_eq!(result, Err(DomainError::validation("html")));
    }

    #[test]
    fn finds_article_summary_by_category_and_slug() {
        let index = ArticleIndexDocument {
            articles: vec![sample_summary()],
        };
        let slug = Slug::new("intro00000001".to_string()).unwrap();

        let article = find_article_summary(&index, &Category::Tech, &slug).unwrap();

        assert_eq!(article.title, "Intro");
    }

    #[test]
    fn rejects_blank_category_html() {
        let mut artifact = category_artifact();
        artifact.html = "  ".to_string();

        let result = build_category_page_document(&artifact);

        assert_eq!(result, Err(DomainError::validation("html")));
    }

    #[test]
    fn groups_category_articles_by_section_path() {
        let mut artifact = category_artifact();
        artifact.articles = vec![
            ArticleSummaryDocument {
                slug: "alpha0000001".to_string(),
                title: "Alpha".to_string(),
                section_path: SectionPath::new(vec!["rust".to_string()]),
                ..sample_summary()
            },
            ArticleSummaryDocument {
                slug: "beta00000001".to_string(),
                title: "Beta".to_string(),
                section_path: SectionPath::new(vec!["rust".to_string(), "async".to_string()]),
                ..sample_summary()
            },
            ArticleSummaryDocument {
                slug: "gamma0000001".to_string(),
                title: "Gamma".to_string(),
                section_path: SectionPath::default(),
                ..sample_summary()
            },
        ];

        let document = build_category_page_document(&artifact).unwrap();

        assert_eq!(document.sections.len(), 3);
        assert_eq!(document.sections[0].heading, "General");
        assert_eq!(document.sections[1].heading, "rust");
        assert_eq!(document.sections[2].heading, "rust / async");
    }
}
