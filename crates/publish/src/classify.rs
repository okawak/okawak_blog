use crate::{PublishError, Result, input::Document};
use domain::{Category, ContentKind, PageKey, SectionPath, Slug};
use std::collections::HashSet;

pub(crate) struct ParsedArticleFile {
    pub(crate) category: Category,
    pub(crate) slug: Slug,
    /// Category-relative directories used to group articles in category navigation.
    pub(crate) section_path: SectionPath,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: domain::PublicContentMeta,
}

pub(crate) struct ParsedPageFile {
    pub(crate) page: PageKey,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: domain::PublicContentMeta,
}

pub(crate) struct ParsedHomeFile {
    pub(crate) markdown_body: String,
    pub(crate) front_matter: domain::PublicContentMeta,
}

pub(crate) struct ParsedCategoryFile {
    pub(crate) category: Category,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: domain::PublicContentMeta,
}

#[derive(Default)]
pub(crate) struct ClassifiedFiles {
    pub(crate) articles: Vec<ParsedArticleFile>,
    pub(crate) pages: Vec<ParsedPageFile>,
    pub(crate) home: Option<ParsedHomeFile>,
    pub(crate) categories: Vec<ParsedCategoryFile>,
}

pub(crate) fn classify(documents: Vec<Document>) -> ClassifiedFiles {
    let mut files = ClassifiedFiles::default();
    for doc in documents {
        match doc.meta.kind {
            ContentKind::Article => files.articles.push(ParsedArticleFile {
                category: doc.meta.category.expect("validated article category"),
                slug: doc.meta.id.clone(),
                section_path: doc.meta.section_path.clone(),
                markdown_body: doc.body,
                front_matter: doc.meta,
            }),
            ContentKind::Category => files.categories.push(ParsedCategoryFile {
                category: doc.meta.category.expect("validated landing category"),
                markdown_body: doc.body,
                front_matter: doc.meta,
            }),
            ContentKind::Page => files.pages.push(ParsedPageFile {
                page: doc.meta.page.clone().expect("validated page key"),
                markdown_body: doc.body,
                front_matter: doc.meta,
            }),
            ContentKind::Home => {
                files.home = Some(ParsedHomeFile {
                    markdown_body: doc.body,
                    front_matter: doc.meta,
                })
            }
        }
    }
    files
}

pub(crate) fn ensure_category_landings(
    articles: &[ParsedArticleFile],
    categories: &[ParsedCategoryFile],
) -> Result<()> {
    let landings: HashSet<_> = categories.iter().map(|d| d.category).collect();
    if let Some(article) = articles.iter().find(|d| !landings.contains(&d.category)) {
        return Err(PublishError::MissingCategoryLanding {
            category: article.category,
        });
    }
    Ok(())
}
