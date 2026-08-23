use crate::error::{PublishError, Result};
use crate::vault::{ContentKind, ObsidianFrontMatter, ParsedObsidianFile, parse_obsidian_file};
use domain::{Category, PageKey, SectionPath, Slug};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::error;

pub(crate) struct ParsedArticleFile {
    pub(crate) category: Category,
    pub(crate) slug: Slug,
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
    /// Category-relative directories used to group articles in category navigation.
    pub(crate) section_path: SectionPath,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedPageFile {
    pub(crate) page: PageKey,
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedHomeFile {
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

pub(crate) struct ParsedCategoryFile {
    pub(crate) category: Category,
    /// Extensionless vault-relative key used to resolve Obsidian internal links.
    pub(crate) source_key: String,
    pub(crate) markdown_body: String,
    pub(crate) front_matter: ObsidianFrontMatter,
}

enum ClassifiedFile {
    Article(ParsedArticleFile),
    Page(ParsedPageFile),
    Home(ParsedHomeFile),
    Category(ParsedCategoryFile),
}

#[derive(Default)]
pub(crate) struct ClassifiedFiles {
    pub(crate) articles: Vec<ParsedArticleFile>,
    pub(crate) pages: Vec<ParsedPageFile>,
    pub(crate) home: Option<ParsedHomeFile>,
    pub(crate) categories: Vec<ParsedCategoryFile>,
    pub(crate) skipped: usize,
    pub(crate) errors: usize,
}

impl ClassifiedFiles {
    fn add(&mut self, file: ClassifiedFile) -> Result<()> {
        match file {
            ClassifiedFile::Article(file) => self.articles.push(file),
            ClassifiedFile::Page(file) => self.pages.push(file),
            ClassifiedFile::Home(file) => {
                if self.home.is_some() {
                    return Err(PublishError::Parse(
                        "Duplicate home content detected".to_string(),
                    ));
                }
                self.home = Some(file);
            }
            ClassifiedFile::Category(file) => self.categories.push(file),
        }
        Ok(())
    }
}

pub(crate) fn classify_obsidian_files(
    markdown_files: Vec<PathBuf>,
    obsidian_dir: &Path,
) -> ClassifiedFiles {
    let mut classified_files = ClassifiedFiles::default();

    for file_path in markdown_files {
        match classify_file(&file_path, obsidian_dir) {
            Ok(Some(file)) => {
                if let Err(error) = classified_files.add(file) {
                    classified_files.errors += 1;
                    error!(file_path = %file_path.display(), %error, "failed to process file");
                }
            }
            Ok(None) => {
                classified_files.skipped += 1;
            }
            Err(error) => {
                classified_files.errors += 1;
                error!(file_path = %file_path.display(), %error, "failed to process file");
            }
        }
    }

    classified_files
}

fn classify_file(file_path: &Path, obsidian_dir: &Path) -> Result<Option<ClassifiedFile>> {
    let Some(parsed_file) = parse_obsidian_file(file_path)? else {
        return Ok(None);
    };
    if !parsed_file.front_matter.is_completed {
        return Ok(None);
    }

    let source_key = derive_source_key(file_path, obsidian_dir)?;
    let classified_file = match parsed_file.front_matter.kind {
        ContentKind::Article => ClassifiedFile::Article(process_article_file(
            file_path,
            parsed_file,
            obsidian_dir,
            source_key,
        )?),
        ContentKind::Page => {
            let page = parse_page_key(parsed_file.front_matter.page.as_deref())?;
            ClassifiedFile::Page(ParsedPageFile {
                page,
                source_key,
                markdown_body: parsed_file.markdown_body,
                front_matter: parsed_file.front_matter,
            })
        }
        ContentKind::Home => ClassifiedFile::Home(ParsedHomeFile {
            source_key,
            markdown_body: parsed_file.markdown_body,
            front_matter: parsed_file.front_matter,
        }),
        ContentKind::Category => {
            let category = parse_category(parsed_file.front_matter.category.as_deref())?;
            ClassifiedFile::Category(ParsedCategoryFile {
                category,
                source_key,
                markdown_body: parsed_file.markdown_body,
                front_matter: parsed_file.front_matter,
            })
        }
    };

    Ok(Some(classified_file))
}

fn process_article_file(
    file_path: &Path,
    parsed_file: ParsedObsidianFile,
    obsidian_dir: &Path,
    source_key: String,
) -> Result<ParsedArticleFile> {
    let relative_path = file_path.strip_prefix(obsidian_dir)?;
    let category = parse_category(parsed_file.front_matter.category.as_deref())?;
    let category_relative_path = relative_path.strip_prefix(category.as_str())?;
    let slug = crate::slug::generate_slug(
        &parsed_file.front_matter.title,
        relative_path,
        &parsed_file.front_matter.created,
    )?;
    let section_path = derive_section_path(category_relative_path);

    Ok(ParsedArticleFile {
        category,
        slug,
        source_key,
        section_path,
        markdown_body: parsed_file.markdown_body,
        front_matter: parsed_file.front_matter,
    })
}

fn derive_source_key(file_path: &Path, obsidian_dir: &Path) -> Result<String> {
    Ok(file_path
        .strip_prefix(obsidian_dir)?
        .with_extension("")
        .to_string_lossy()
        .into_owned())
}

fn derive_section_path(category_relative_path: &Path) -> SectionPath {
    let segments = category_relative_path
        .parent()
        .map(|parent| {
            parent
                .iter()
                .map(|component| component.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    SectionPath::new(segments)
}

pub(crate) fn ensure_unique_page_keys(pages: &[ParsedPageFile]) -> Result<()> {
    let mut seen = HashSet::with_capacity(pages.len());
    for parsed_page in pages {
        if !seen.insert(parsed_page.page.as_str()) {
            return Err(PublishError::Parse(format!(
                "Duplicate page key detected: {}",
                parsed_page.page.as_str()
            )));
        }
    }
    Ok(())
}

pub(crate) fn ensure_unique_category_landings(categories: &[ParsedCategoryFile]) -> Result<()> {
    let mut seen = HashSet::with_capacity(categories.len());
    for parsed_category in categories {
        if !seen.insert(parsed_category.category.as_str()) {
            return Err(PublishError::Parse(format!(
                "Duplicate category landing detected: {}",
                parsed_category.category.as_str()
            )));
        }
    }
    Ok(())
}

pub(crate) fn ensure_category_landings(
    articles: &[ParsedArticleFile],
    categories: &[ParsedCategoryFile],
) -> Result<()> {
    let category_landings: HashSet<_> = categories.iter().map(|file| file.category).collect();
    let missing = articles
        .iter()
        .map(|file| file.category)
        .find(|category| !category_landings.contains(category));

    match missing {
        Some(category) => Err(PublishError::MissingCategoryLanding { category }),
        None => Ok(()),
    }
}

fn parse_category(category: Option<&str>) -> Result<Category> {
    let category = category
        .ok_or_else(|| PublishError::Parse("Completed content requires a category".to_string()))?;
    category.parse().map_err(Into::into)
}

fn parse_page_key(page: Option<&str>) -> Result<PageKey> {
    let page =
        page.ok_or_else(|| PublishError::Parse("Completed pages require a page key".to_string()))?;
    PageKey::new(page.trim().to_string()).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const TEST_TIMESTAMP: &str = "2025-01-01T00:00:00+09:00";

    fn front_matter(kind: ContentKind) -> ObsidianFrontMatter {
        ObsidianFrontMatter {
            title: "Test".to_string(),
            kind,
            tags: None,
            summary: None,
            is_completed: true,
            priority: None,
            created: TEST_TIMESTAMP.to_string(),
            updated: TEST_TIMESTAMP.to_string(),
            category: None,
            page: None,
        }
    }

    fn parsed_article(category: Category) -> ParsedObsidianFile {
        let mut front_matter = front_matter(ContentKind::Article);
        front_matter.category = Some(category.as_str().to_string());
        ParsedObsidianFile {
            front_matter,
            markdown_body: String::new(),
        }
    }

    fn parsed_category(category: Category) -> ParsedCategoryFile {
        let mut front_matter = front_matter(ContentKind::Category);
        front_matter.category = Some(category.as_str().to_string());
        ParsedCategoryFile {
            category,
            source_key: format!("{}/index", category.as_str()),
            markdown_body: String::new(),
            front_matter,
        }
    }

    fn parsed_page(page: &str) -> ParsedPageFile {
        let mut front_matter = front_matter(ContentKind::Page);
        front_matter.page = Some(page.to_string());
        ParsedPageFile {
            page: PageKey::new(page.to_string()).unwrap(),
            source_key: format!("pages/{page}"),
            markdown_body: String::new(),
            front_matter,
        }
    }

    fn parsed_home() -> ParsedHomeFile {
        ParsedHomeFile {
            source_key: "home".to_string(),
            markdown_body: String::new(),
            front_matter: front_matter(ContentKind::Home),
        }
    }

    #[test]
    fn test_add_rejects_duplicate_home_files() {
        let mut files = ClassifiedFiles::default();
        files.add(ClassifiedFile::Home(parsed_home())).unwrap();

        let result = files.add(ClassifiedFile::Home(parsed_home()));

        assert!(
            matches!(result, Err(PublishError::Parse(message)) if message.contains("Duplicate home content"))
        );
    }

    #[test]
    fn test_ensure_unique_category_landings_rejects_duplicates() {
        let categories = vec![
            parsed_category(Category::Tech),
            parsed_category(Category::Tech),
        ];

        let result = ensure_unique_category_landings(&categories);

        assert!(
            matches!(result, Err(PublishError::Parse(message)) if message.contains("Duplicate category landing"))
        );
    }

    #[test]
    fn test_process_article_file_rejects_category_path_mismatch() {
        let obsidian_dir = Path::new("/vault");
        let parsed_file = parsed_article(Category::Tech);

        let result = process_article_file(
            Path::new("/vault/daily/article.md"),
            parsed_file,
            obsidian_dir,
            "daily/article".to_string(),
        );

        assert!(matches!(result, Err(PublishError::StripPrefix(_))));
    }

    #[test]
    fn test_derive_source_key_uses_extensionless_vault_relative_path() {
        let source_key =
            derive_source_key(Path::new("/vault/pages/about.md"), Path::new("/vault")).unwrap();

        assert_eq!(source_key, "pages/about");
    }

    #[rstest]
    #[case::single_section("block1/hoge.md", &["block1"])]
    #[case::nested_sections("rust/async/hoge.md", &["rust", "async"])]
    #[case::root_article("hoge.md", &[])]
    fn test_derive_section_path(#[case] path: &str, #[case] expected: &[&str]) {
        let section_path = derive_section_path(Path::new(path));

        assert_eq!(section_path.segments(), expected);
    }

    #[test]
    fn test_parse_page_key_success() {
        assert_eq!(parse_page_key(Some(" about ")).unwrap().as_str(), "about");
    }

    #[test]
    fn test_parse_page_key_rejects_missing_value() {
        assert!(matches!(
            parse_page_key(None),
            Err(PublishError::Parse(message)) if message.contains("require a page key")
        ));
    }

    #[test]
    fn test_parse_category_success() {
        assert_eq!(parse_category(Some("tech")).unwrap(), Category::Tech);
    }

    #[test]
    fn test_parse_category_rejects_missing_value() {
        assert!(matches!(
            parse_category(None),
            Err(PublishError::Parse(message)) if message.contains("requires a category")
        ));
    }

    #[test]
    fn test_ensure_unique_page_keys_rejects_duplicates() {
        let parsed_pages = vec![parsed_page("about"), parsed_page("about")];

        assert!(matches!(
            ensure_unique_page_keys(&parsed_pages),
            Err(PublishError::Parse(message)) if message.contains("Duplicate page key")
        ));
    }
}
