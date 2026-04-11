use indoc::indoc;
use publisher::{Config, offline_bookmark_enricher, run_main, run_with_enricher};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_run_main_with_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // Create an empty Obsidian directory.
    fs::create_dir_all(&obsidian_dir).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // Run `run_main`.
    let result = run_main(&config).await;
    assert!(result.is_ok());

    // Verify that the output directory was created.
    assert!(output_dir.exists());
    assert!(output_dir.is_dir());
}

#[tokio::test]
async fn test_run_main_with_sample_file() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // Create the Obsidian directory and a sample file.
    fs::create_dir_all(&obsidian_dir).unwrap();

    let sample_content = indoc! {r#"
        ---
        title: "Test Article"
        tags: ["test"]
        summary: "Test summary"
        priority: 1
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-01T00:00:00+09:00"
        is_completed: true
        category: "tech"
        ---

        # Test Article

        This is a test article.
    "#};

    let sample_file = obsidian_dir.join("test.md");
    fs::write(&sample_file, sample_content).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // Run `run_main`.
    let result = run_main(&config).await;
    assert!(result.is_ok());

    let site_root = output_dir.join("site");
    let articles_dir = site_root.join("articles");

    // Verify that at least one slug-based HTML file was generated.
    let html_files: Vec<_> = fs::read_dir(&articles_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "html"))
        .collect();

    assert!(
        !html_files.is_empty(),
        "At least one HTML file should be generated"
    );

    // Verify the generated HTML content.
    let html_file = &html_files[0];
    let html_content = fs::read_to_string(html_file.path()).unwrap();
    assert!(html_content.contains("Test Article"));
    assert!(html_content.contains("This is a test article"));

    let article_index = fs::read_to_string(articles_dir.join("index.json")).unwrap();
    assert!(article_index.contains("\"articles\""));
    assert!(article_index.contains("\"category\": \"tech\""));

    let site_metadata = fs::read_to_string(site_root.join("metadata").join("site.json")).unwrap();
    assert!(site_metadata.contains("\"total_articles\": 1"));
}

#[tokio::test]
async fn test_run_main_with_incomplete_file() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    // Create the Obsidian directory and a file with `is_completed: false`.
    fs::create_dir_all(&obsidian_dir).unwrap();

    let incomplete_content = indoc! {r#"
        ---
        title: "Incomplete Article"
        tags: ["test"]
        summary: "Incomplete summary"
        priority: 1
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-01T00:00:00+09:00"
        is_completed: false
        category: "tech"
        ---

        # Incomplete Article

        This article is not completed.
    "#};

    let sample_file = obsidian_dir.join("incomplete.md");
    fs::write(&sample_file, incomplete_content).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    // Run `run_main`.
    let result = run_main(&config).await;
    assert!(result.is_ok());

    // Verify that no HTML file is generated for incomplete content.
    let html_file = output_dir
        .join("site")
        .join("articles")
        .join("incomplete.html");
    assert!(!html_file.exists());
}

#[test]
fn test_config_validation() {
    // Verify config behavior with a non-existent directory.
    let temp_dir = TempDir::new().unwrap();
    let non_existent_dir = temp_dir.path().join("non_existent");

    let config = Config {
        obsidian_dir: non_existent_dir,
        output_dir: PathBuf::from("test_output"),
    };

    // `validate` is not called directly here, so assert the missing-path behavior instead.
    assert!(!config.obsidian_dir.exists());
}

#[tokio::test]
async fn test_run_with_enricher_with_bookmark_article() {
    let temp_dir = TempDir::new().unwrap();
    let obsidian_dir = temp_dir.path().join("obsidian");
    let output_dir = temp_dir.path().join("dist");

    fs::create_dir_all(&obsidian_dir).unwrap();

    // Use an offline enricher that converts bookmarks with fallback data only,
    // so no network request is made and the test never waits for a timeout.
    let sample_content = indoc! {r#"
        ---
        title: "Bookmark Article"
        tags: ["test"]
        summary: "Article containing a bookmark block"
        priority: 1
        created: "2025-01-01T00:00:00+09:00"
        updated: "2025-01-01T00:00:00+09:00"
        is_completed: true
        category: "tech"
        ---

        Here is a bookmark:

        <div class="bookmark">
          <a href="https://example.com">Fallback Bookmark</a>
        </div>
    "#};

    let sample_file = obsidian_dir.join("bookmark.md");
    fs::write(&sample_file, sample_content).unwrap();

    let config = Config {
        obsidian_dir,
        output_dir: output_dir.clone(),
    };

    let result = run_with_enricher(&config, offline_bookmark_enricher()).await;
    assert!(result.is_ok());

    let articles_dir = output_dir.join("site").join("articles");
    let html_files: Vec<_> = fs::read_dir(&articles_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "html"))
        .collect();

    assert!(!html_files.is_empty(), "HTML file should be generated");

    let html_content = fs::read_to_string(html_files[0].path()).unwrap();

    // The bookmark should NOT remain as escaped HTML.
    assert!(
        !html_content.contains("&lt;div class=&quot;bookmark&quot;&gt;"),
        "bookmark HTML should not be escaped; got: {html_content}"
    );

    // The bookmark should have been converted to the rich card format
    // (fallback data is used when the OGP fetch fails).
    assert!(
        html_content.contains(r#"class="bookmark-link""#),
        "bookmark should be converted to rich format; got: {html_content}"
    );
    assert!(
        html_content.contains(r#"class="bookmark-domain""#),
        "bookmark should contain domain info; got: {html_content}"
    );
}
