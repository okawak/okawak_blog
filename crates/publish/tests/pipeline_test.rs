use domain::{
    ArticleIndexDocument, CategoryArtifactDocument, HomeFragmentArtifactDocument,
    PageArtifactDocument, SiteMetadataDocument,
};
use indoc::{formatdoc, indoc};
use publish::{BookmarkEnricher, PublishError, publish, publish_with_bookmark_enricher};
use rstest::rstest;
use serde::de::DeserializeOwned;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::TempDir;

fn offline_bookmark_enricher() -> BookmarkEnricher {
    Arc::new(|html: String| {
        Box::pin(async move {
            const SIMPLE_BOOKMARK: &str = indoc! {r#"
                <div class="bookmark">
                  <a href="https://example.com">Fallback Bookmark</a>
                </div>
            "#};
            const RICH_BOOKMARK: &str =
                r#"<a class="bookmark-link"><span class="bookmark-domain">example.com</span></a>"#;

            if !html.contains("Fallback Bookmark") {
                return html;
            }
            assert!(html.contains(SIMPLE_BOOKMARK));
            html.replace(SIMPLE_BOOKMARK, RICH_BOOKMARK)
        })
    })
}

#[tokio::test]
async fn test_publish_with_empty_directory() {
    let fixture = PublishFixture::new();

    let result = publish(fixture.obsidian_dir(), fixture.output_dir()).await;

    assert!(matches!(result, Err(PublishError::NoArticles)));
}

#[tokio::test]
async fn test_publish_requires_about_page() {
    let fixture = PublishFixture::new();
    fixture.write_required_article();
    fixture.write_tech_category_landing();

    let result = publish(fixture.obsidian_dir(), fixture.output_dir()).await;

    assert!(matches!(result, Err(PublishError::MissingAboutPage)));
    assert!(!fixture.output_dir().exists());
}

#[tokio::test]
async fn test_publish_writes_article_index_and_metadata() {
    let fixture = PublishFixture::new();
    fixture.write_required_site();

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    let site_root = fixture.site_root();
    let html_files = collect_html_files(&site_root.join("articles"));
    assert_eq!(html_files.len(), 1);
    let html = fs::read_to_string(&html_files[0]).unwrap();
    assert!(html.contains("Required Article"));
    assert!(html.contains("This article makes the fixture deployable."));

    let article_index: ArticleIndexDocument = read_json(site_root.join("articles/index.json"));
    assert_eq!(article_index.articles.len(), 1);
    assert_eq!(article_index.articles[0].category, "tech");

    let site_metadata: SiteMetadataDocument = read_json(site_root.join("metadata/site.json"));
    assert_eq!(site_metadata.total_articles, 1);
}

#[tokio::test]
async fn test_publish_resolves_links_to_all_content_kinds() {
    let fixture = PublishFixture::new();
    fixture.write_article(
        "tech/links.md",
        "Links",
        "tech",
        true,
        indoc! {r#"
            [[about|About]]
            [[home|Home]]
            [[tech/category|Tech]]
            ![[about|About image]]
        "#},
    );
    fixture.write_about_page();
    fixture.write_home_fragment("home.md");
    fixture.write_tech_category_landing();

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    let html_path = collect_html_files(&fixture.site_root().join("articles"))
        .pop()
        .unwrap();
    let html = fs::read_to_string(html_path).unwrap();
    for expected in [
        r#"href="/about">About</a>"#,
        r#"href="/">Home</a>"#,
        r#"href="/tech">Tech</a>"#,
        r#"src="/about" alt="About image""#,
    ] {
        assert!(html.contains(expected), "missing {expected}");
    }
}

#[tokio::test]
async fn test_publish_skips_unpublishable_files() {
    let fixture = PublishFixture::new();
    fixture.write_required_site();
    fixture.write_article(
        "tech/incomplete.md",
        "Incomplete Article",
        "tech",
        false,
        "# Incomplete Article",
    );
    fixture.write(
        "notes.md",
        indoc! {r#"
            # Notes

            Markdown without frontmatter is not publishable.
        "#},
    );

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    let article_index: ArticleIndexDocument =
        read_json(fixture.site_root().join("articles/index.json"));
    assert_eq!(article_index.articles.len(), 1);
    assert_eq!(article_index.articles[0].title, "Required Article");
}

#[tokio::test]
async fn test_publish_writes_static_page() {
    let fixture = PublishFixture::new();
    fixture.write_required_site();

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    let page: PageArtifactDocument = read_json(fixture.site_root().join("pages/about.json"));
    assert_eq!(page.page.as_str(), "about");
    assert_eq!(page.title, "About");
    assert!(page.html.contains("This page is required for deployment."));
}

#[tokio::test]
async fn test_publish_writes_home_fragment_separately_from_pages() {
    let fixture = PublishFixture::new();
    fixture.write_required_site();
    fixture.write_home_fragment("home.md");

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    let home: HomeFragmentArtifactDocument = read_json(fixture.site_root().join("home.json"));
    assert_eq!(home.title, "Home");
    assert!(
        home.html
            .contains("This fragment is generated from markdown.")
    );
    assert!(!fixture.site_root().join("pages/home.json").exists());
}

#[tokio::test]
async fn test_publish_rejects_duplicate_home_files() {
    let fixture = PublishFixture::new();
    fixture.write_required_site();
    fixture.write_home_fragment("home.md");
    fixture.write_home_fragment("another-home.md");

    let result = publish(fixture.obsidian_dir(), fixture.output_dir()).await;

    assert!(matches!(
        result,
        Err(PublishError::ContentErrors { count: 1 })
    ));
    assert!(!fixture.output_dir().exists());
}

#[tokio::test]
async fn test_publish_writes_category_landing() {
    let fixture = PublishFixture::new();
    fixture.write_required_article();
    fixture.write_about_page();
    fixture.write(
        "tech/category.md",
        indoc! {r#"
            ---
            title: "Tech"
            kind: category
            category: tech
            summary: "  Technology landing  "
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            is_completed: true
            ---

            # Tech

            Welcome to the category landing page.
        "#},
    );

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    let category: CategoryArtifactDocument =
        read_json(fixture.site_root().join("categories/tech.json"));
    assert_eq!(category.category, "tech");
    assert_eq!(category.title, "Tech");
    assert_eq!(
        category.description.as_deref(),
        Some("  Technology landing  ")
    );
    assert!(
        category
            .html
            .contains("Welcome to the category landing page.")
    );
}

#[tokio::test]
async fn test_publish_rejects_missing_category_landing_before_writing() {
    let fixture = PublishFixture::new();
    fixture.write_required_article();
    fixture.write_about_page();

    let result = publish(fixture.obsidian_dir(), fixture.output_dir()).await;

    assert!(matches!(
        result,
        Err(PublishError::MissingCategoryLanding {
            category: domain::Category::Tech
        })
    ));
    assert!(!fixture.output_dir().exists());
}

#[rstest]
#[case::blank_title(
    "   ",
    indoc! {r#"
        # Tech

        Category description.
    "#}
)]
#[case::blank_body("Tech", "   ")]
#[tokio::test]
async fn test_publish_rejects_incomplete_category_landing(#[case] title: &str, #[case] body: &str) {
    let fixture = PublishFixture::new();
    fixture.write_required_article();
    fixture.write_about_page();
    fixture.write(
        "tech/category.md",
        &formatdoc! {r#"
            ---
            title: "{title}"
            kind: category
            category: tech
            created: "2025-01-01T00:00:00+09:00"
            updated: "2025-01-01T00:00:00+09:00"
            is_completed: true
            ---

            {body}
        "#},
    );

    let result = publish(fixture.obsidian_dir(), fixture.output_dir()).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_publish_rejects_non_existent_obsidian_directory() {
    let fixture = PublishFixture::new();
    let missing_dir = fixture.obsidian_dir().join("missing");

    let result = publish(&missing_dir, fixture.output_dir()).await;

    assert!(matches!(
        result,
        Err(PublishError::InvalidSourceDirectory(_))
    ));
    assert!(!fixture.output_dir().exists());
}

#[tokio::test]
async fn test_publish_uses_injected_bookmark_enricher() {
    let fixture = PublishFixture::new();
    fixture.write_about_page();
    fixture.write_tech_category_landing();
    fixture.write_article(
        "tech/bookmark.md",
        "Bookmark Article",
        "tech",
        true,
        indoc! {r#"
            Here is a bookmark:

            <div class="bookmark">
              <a href="https://example.com">Fallback Bookmark</a>
            </div>
        "#},
    );

    publish_with_bookmark_enricher(
        fixture.obsidian_dir(),
        fixture.output_dir(),
        offline_bookmark_enricher(),
    )
    .await
    .unwrap();

    let html_path = collect_html_files(&fixture.site_root().join("articles"))
        .pop()
        .unwrap();
    let html = fs::read_to_string(html_path).unwrap();
    assert!(!html.contains("&lt;div class=&quot;bookmark&quot;&gt;"));
    assert!(html.contains(r#"class="bookmark-link""#));
    assert!(html.contains(r#"class="bookmark-domain""#));
}

#[tokio::test]
async fn test_publish_processes_a_realistic_vault() {
    let fixture = PublishFixture::new();
    fixture.write_about_page();
    fixture.write_tech_category_landing();
    fixture.write_article(
        "tech/rust-performance.md",
        "Rustでのパフォーマンス最適化",
        "tech",
        true,
        indoc! {r#"
            # Rustでのパフォーマンス最適化

            ```rust
            fn fibonacci() {}
            ```
        "#},
    );
    fixture.write_article(
        "tech/basic-rust-concepts.md",
        "基本的なRust概念",
        "tech",
        true,
        indoc! {r#"
            # 基本的なRust概念

            Rustの**所有権システム**について学びます。
        "#},
    );
    fixture.write_article(
        "tech/memory-best-practices.md",
        "メモリ管理のベストプラクティス",
        "tech",
        true,
        indoc! {r#"
            # メモリ管理のベストプラクティス

            $$x = 1$$

            $O(n)$
        "#},
    );
    fixture.write_article(
        "blog/development-diary.md",
        "開発日記: ブログシステムを作ってみた",
        "blog",
        false,
        indoc! {r#"
            # 開発日記

            まだ作成中です。
        "#},
    );

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    let site_root = fixture.site_root();
    let articles_dir = site_root.join("articles");
    let files = collect_html_files(&articles_dir);
    assert_eq!(files.len(), 3);

    let combined_html = files
        .iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<String>();
    for expected in [
        "Rustでのパフォーマンス最適化",
        "fibonacci",
        "メモリ管理のベストプラクティス",
        r#"<span class="math math-display">"#,
        r#"<span class="math math-inline">"#,
    ] {
        assert!(combined_html.contains(expected), "missing {expected}");
    }
    assert!(!combined_html.contains("開発日記: ブログシステムを作ってみた"));

    let article_index: ArticleIndexDocument = read_json(articles_dir.join("index.json"));
    for expected_title in [
        "Rustでのパフォーマンス最適化",
        "基本的なRust概念",
        "メモリ管理のベストプラクティス",
    ] {
        assert!(
            article_index
                .articles
                .iter()
                .any(|article| article.title == expected_title),
            "article index should contain {expected_title}"
        );
    }
    assert!(
        article_index
            .articles
            .iter()
            .all(|article| article.title != "開発日記: ブログシステムを作ってみた")
    );
    for article in &article_index.articles {
        assert!(
            articles_dir
                .join(&article.category)
                .join(format!("{}.html", article.slug))
                .is_file()
        );
    }

    let tech_category: CategoryArtifactDocument = read_json(site_root.join("categories/tech.json"));
    assert_eq!(tech_category.category, "tech");
    assert_eq!(tech_category.articles.len(), 3);

    let site_metadata: SiteMetadataDocument = read_json(site_root.join("metadata/site.json"));
    assert_eq!(site_metadata.total_articles, 3);
}

#[tokio::test]
async fn test_publish_handles_many_articles() {
    let fixture = PublishFixture::new();
    fixture.write_about_page();
    fixture.write_tech_category_landing();

    for index in 0..100 {
        fixture.write_article(
            &format!("tech/article-{index:03}.md"),
            &format!("Test Article {index}"),
            "tech",
            true,
            &formatdoc! {r#"
                # Test Article {index}

                Test content {index}.
            "#},
        );
    }

    publish(fixture.obsidian_dir(), fixture.output_dir())
        .await
        .unwrap();

    assert_eq!(
        collect_html_files(&fixture.site_root().join("articles")).len(),
        100
    );
}

#[tokio::test]
async fn test_publish_rejects_content_errors_without_writing_artifacts() {
    let fixture = PublishFixture::new();
    fixture.write(
        "tech/invalid.md",
        indoc! {r#"
            ---
            title: "Invalid YAML"
            tags: [invalid yaml structure
            is_completed: true
            category: tech
            ---
        "#},
    );

    let result = publish(fixture.obsidian_dir(), fixture.output_dir()).await;

    assert!(matches!(
        result,
        Err(PublishError::ContentErrors { count: 1 })
    ));
    assert!(!fixture.output_dir().exists());
}

struct PublishFixture {
    _temp_dir: TempDir,
    obsidian_dir: PathBuf,
    output_dir: PathBuf,
}

impl PublishFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let obsidian_dir = temp_dir.path().join("obsidian");
        let output_dir = temp_dir.path().join("dist");
        fs::create_dir_all(&obsidian_dir).unwrap();

        Self {
            _temp_dir: temp_dir,
            obsidian_dir,
            output_dir,
        }
    }

    fn obsidian_dir(&self) -> &Path {
        &self.obsidian_dir
    }

    fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    fn site_root(&self) -> PathBuf {
        self.output_dir.join("site")
    }

    fn write(&self, relative_path: impl AsRef<Path>, content: &str) {
        let path = self.obsidian_dir.join(relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn write_article(
        &self,
        relative_path: &str,
        title: &str,
        category: &str,
        is_completed: bool,
        body: &str,
    ) {
        self.write(
            relative_path,
            &formatdoc! {r#"
                ---
                title: "{title}"
                created: "2025-01-01T00:00:00+09:00"
                updated: "2025-01-01T00:00:00+09:00"
                is_completed: {is_completed}
                category: "{category}"
                ---

                {body}
            "#},
        );
    }

    fn write_required_article(&self) {
        self.write_article(
            "tech/required-article.md",
            "Required Article",
            "tech",
            true,
            indoc! {r#"
                # Required Article

                This article makes the fixture deployable.
            "#},
        );
    }

    fn write_about_page(&self) {
        self.write(
            "about.md",
            indoc! {r#"
                ---
                title: "About"
                kind: page
                page: about
                summary: "About this site"
                created: "2025-01-01T00:00:00+09:00"
                updated: "2025-01-01T00:00:00+09:00"
                is_completed: true
                ---

                # About

                This page is required for deployment.
            "#},
        );
    }

    fn write_home_fragment(&self, relative_path: &str) {
        self.write(
            relative_path,
            indoc! {r#"
                ---
                title: "Home"
                kind: home
                summary: "Home intro"
                created: "2025-01-01T00:00:00+09:00"
                updated: "2025-01-01T00:00:00+09:00"
                is_completed: true
                ---

                # Welcome

                This fragment is generated from markdown.
            "#},
        );
    }

    fn write_tech_category_landing(&self) {
        self.write(
            "tech/category.md",
            indoc! {r#"
                ---
                title: "Tech"
                kind: category
                category: tech
                summary: "Technology articles"
                created: "2025-01-01T00:00:00+09:00"
                updated: "2025-01-01T00:00:00+09:00"
                is_completed: true
                ---

                # Tech

                Technology articles.
            "#},
        );
    }

    fn write_required_site(&self) {
        self.write_required_article();
        self.write_about_page();
        self.write_tech_category_landing();
    }
}

fn collect_html_files(root: &Path) -> Vec<PathBuf> {
    let mut html_files = Vec::new();

    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            html_files.extend(collect_html_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "html") {
            html_files.push(path);
        }
    }

    html_files.sort();
    html_files
}

fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_reader(File::open(path).unwrap()).unwrap()
}
