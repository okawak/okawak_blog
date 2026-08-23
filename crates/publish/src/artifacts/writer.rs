use super::builder::SiteDocuments;
use crate::error::Result;

use domain::{Category, Slug};
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
}

impl SiteOutput {
    pub(crate) fn prepare(output_dir: impl AsRef<Path>) -> Result<Self> {
        let root = output_dir.as_ref().join("site");
        for directory in ["articles", "categories", "metadata", "pages"] {
            fs::create_dir_all(root.join(directory))?;
        }

        Ok(Self { root })
    }
}

pub(crate) fn write_article_page(
    site_output: &SiteOutput,
    category: Category,
    slug: &Slug,
    html: &str,
) -> Result<PathBuf> {
    let article_dir = site_output.root.join("articles").join(category.as_str());
    fs::create_dir_all(&article_dir)?;
    let output_file_path = article_dir.join(format!("{}.html", slug.as_str()));
    fs::write(&output_file_path, html)?;
    Ok(output_file_path)
}

pub(crate) fn write_site_documents(
    site_output: &SiteOutput,
    site_documents: &SiteDocuments,
) -> Result<()> {
    write_json(
        &site_output.root.join("articles/index.json"),
        &site_documents.article_index,
    )?;
    for category_document in &site_documents.category_documents {
        write_json(
            &site_output
                .root
                .join(format!("categories/{}.json", category_document.category)),
            category_document,
        )?;
    }
    for page_document in &site_documents.page_documents {
        write_json(
            &site_output
                .root
                .join(format!("pages/{}.json", page_document.page)),
            page_document,
        )?;
    }
    if let Some(home_fragment) = &site_documents.home_fragment {
        write_json(&site_output.root.join("home.json"), home_fragment)?;
    }

    write_json(
        &site_output.root.join("metadata/site.json"),
        &site_documents.site_metadata,
    )?;

    Ok(())
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
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
    use domain::{
        ArticleMeta, CategoryLandingMeta, HomeFragmentArtifactDocument, PageArtifactDocument,
        PageKey, SectionPath, Timestamp, Title,
    };
    use tempfile::TempDir;

    fn article_meta() -> ArticleMeta {
        ArticleMeta {
            slug: Slug::new("artifact00001".to_string()).unwrap(),
            title: Title::new("Artifact Test".to_string()).unwrap(),
            category: Category::Tech,
            section_path: SectionPath::default(),
            description: Some("Artifact summary".to_string()),
            tags: vec!["rust".to_string()],
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
            vec![domain::PublishableCategoryLanding::new(
                category_landing(),
                domain::CategoryLandingBody::new("<h1>Tech</h1>".to_string()).unwrap(),
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
            article.category,
            &article.slug,
            "<h1>Artifact Test</h1>",
        )
        .unwrap();
        write_site_documents(&output, &documents).unwrap();

        for path in [
            article_path,
            output.root.join("articles/index.json"),
            output.root.join("categories/tech.json"),
            output.root.join("pages/about.json"),
            output.root.join("home.json"),
            output.root.join("metadata/site.json"),
        ] {
            assert!(path.exists(), "{} should exist", path.display());
        }

        let article_index = fs::read_to_string(output.root.join("articles/index.json")).unwrap();
        assert!(article_index.starts_with("{\"articles\":["));
    }
}
