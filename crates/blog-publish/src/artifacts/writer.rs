use super::builder::SiteDocuments;
use crate::error::Result;

use blog_domain::{
    ArticleArtifactDocument, Locale, build_article_path, build_category_path,
    build_static_page_path,
};
use serde::Serialize;
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

/// Prepared root directory for generated site artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteOutput {
    root: PathBuf,
    locale: Locale,
}

impl SiteOutput {
    #[cfg(test)]
    pub(crate) fn prepare(output_dir: impl AsRef<Path>) -> Result<Self> {
        Self::prepare_locale(output_dir.as_ref(), Locale::Ja)
    }

    pub(crate) fn prepare_locale(output_dir: &Path, locale: Locale) -> Result<Self> {
        let root = output_dir.join("site");
        fs::create_dir_all(root.join(locale.as_str()))?;
        Ok(Self { root, locale })
    }

    fn page(&self, route: &str) -> PathBuf {
        self.root.join(self.locale.page_file(route, "json"))
    }

    fn data(&self, filename: &str) -> PathBuf {
        self.root.join(self.locale.data_key(filename))
    }
}

pub(crate) fn write_article_page(
    site_output: &SiteOutput,
    article: &ArticleArtifactDocument,
) -> Result<PathBuf> {
    let route = build_article_path(
        &article.summary.category.parse()?,
        &article.summary.slug.parse()?,
    );
    let output_file_path = site_output.page(&route);
    write_json(&output_file_path, article)?;
    Ok(output_file_path)
}

pub(crate) fn write_site_documents(
    site_output: &SiteOutput,
    site_documents: &SiteDocuments,
) -> Result<()> {
    write_json(
        &site_output.data("articles.json"),
        &site_documents.article_index,
    )?;
    for category_document in &site_documents.category_documents {
        write_json(
            &site_output.page(&build_category_path(&category_document.category.parse()?)),
            category_document,
        )?;
    }
    for page_document in &site_documents.page_documents {
        write_json(
            &site_output.page(&build_static_page_path(&page_document.page)),
            page_document,
        )?;
    }
    if let Some(home_fragment) = &site_documents.home_fragment {
        write_json(&site_output.page("/"), home_fragment)?;
    }

    write_json(
        &site_output.data("site.json"),
        &site_documents.site_metadata,
    )?;

    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::create_dir_all(path.parent().expect("artifact directory"))?;
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, value)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::builder::build_site_documents;
    use super::*;
    use blog_domain::{
        ArticleMeta, Category, CategoryLandingMeta, HomeFragmentArtifactDocument,
        PageArtifactDocument, PageKey, SectionPath, Slug, Timestamp, Title,
    };
    use tempfile::TempDir;

    fn article_meta() -> ArticleMeta {
        ArticleMeta {
            slug: Slug::new("artifact00001".to_string()).unwrap(),
            title: Title::new("Artifact Test".to_string()).unwrap(),
            category: Category::Tech,
            section_path: SectionPath::default(),
            description: Some("Artifact summary".to_string()),
            tags: vec!["rust".parse().unwrap()],
            priority: Some(1),
            created_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
            updated_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
        }
    }

    fn category_landing() -> CategoryLandingMeta {
        CategoryLandingMeta {
            category: Category::Tech,
            title: Title::new("Tech".to_string()).unwrap(),
            description: Some("Tech landing".to_string()),
            updated_at: Timestamp::new("2025-01-01T00:00:00+09:00".to_string()).unwrap(),
        }
    }

    #[test]
    fn test_write_local_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let output = SiteOutput::prepare(temp_dir.path()).unwrap();
        let article = article_meta();
        let documents = build_site_documents(
            vec![article.clone()],
            vec![blog_domain::PublishableCategoryLanding::new(
                category_landing(),
                blog_domain::CategoryLandingBody::new("<h1>Tech</h1>".to_string()).unwrap(),
            )],
            vec![PageArtifactDocument {
                page: PageKey::new("about".to_string()).unwrap(),
                title: "About".to_string(),
                description: Some("About this site".to_string()),
                html: "<article><h1>About</h1></article>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            }],
            Some(HomeFragmentArtifactDocument {
                title: "Home".to_string(),
                description: Some("Home introduction".to_string()),
                html: "<p>Welcome</p>".to_string(),
                updated_at: "2025-01-01T00:00:00+09:00".to_string(),
            }),
        )
        .unwrap();

        let article_path = write_article_page(
            &output,
            &ArticleArtifactDocument {
                summary: (&article).into(),
                html: "<h1>Artifact Test</h1>".into(),
            },
        )
        .unwrap();
        write_site_documents(&output, &documents).unwrap();

        let stored: blog_domain::ArticleArtifactDocument =
            serde_json::from_reader(File::open(&article_path).unwrap()).unwrap();
        assert_eq!(stored.summary, documents.article_index.articles[0]);
        assert_eq!(stored.html, "<h1>Artifact Test</h1>");
        assert_eq!(article_path.extension().unwrap(), "json");
        assert!(!article_path.with_extension("html").exists());

        for path in [
            article_path,
            output.root.join("ja/_data/articles.json"),
            output.root.join("ja/tech.json"),
            output.root.join("ja/about.json"),
            output.root.join("ja/index.json"),
            output.root.join("ja/_data/site.json"),
        ] {
            assert!(path.exists(), "{} should exist", path.display());
        }

        let article_index = fs::read_to_string(output.root.join("ja/_data/articles.json")).unwrap();
        assert!(article_index.starts_with("{\"articles\":["));
    }
}
