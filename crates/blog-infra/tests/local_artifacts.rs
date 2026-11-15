use blog_domain::{Category, SiteLocalesDocument, Slug};
use blog_infra::{ArtifactReader, ArtifactSnapshot, CachingArtifactReader, LocalArtifactReader};
use rstest::rstest;
use std::{fs, sync::Arc, time::Duration};
use tempfile::TempDir;

fn summary(category: &str, slug: &str, title: &str) -> serde_json::Value {
    serde_json::json!({
        "category": category, "slug": slug, "title": title, "section_path": [],
        "description": null, "tags": [], "priority": null,
        "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
    })
}

fn write_article(root: &std::path::Path, locale: &str, category: &str, slug: &str, title: &str) {
    let path = root
        .join(locale)
        .join(format!("articles/{category}/{slug}.json"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        serde_json::to_vec(&serde_json::json!({
            "summary": summary(category, slug, title), "html": format!("<p>{title}</p>")
        }))
        .unwrap(),
    )
    .unwrap();
}

#[tokio::test]
async fn direct_article_reads_need_no_index_and_isolate_locales_and_categories() {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("locales.json"),
        r#"{"schema_version":1,"routes":{"/":["ja","en"]}}"#,
    )
    .unwrap();
    write_article(root.path(), "", "tech", "same", "Japanese");
    write_article(root.path(), "", "daily", "same", "Daily");
    write_article(root.path(), "en", "tech", "same", "English");
    let reader = CachingArtifactReader::new(
        Arc::new(LocalArtifactReader::new(root.path())),
        Duration::from_secs(60),
    );
    let ja = reader.snapshot().await.unwrap();
    let en = ja
        .localized(blog_domain::Locale::En)
        .await
        .unwrap()
        .unwrap();
    let slug: Slug = "same".parse().unwrap();
    for (snapshot, category, expected) in [
        (&ja, Category::Tech, "Japanese"),
        (&ja, Category::Daily, "Daily"),
        (&en, Category::Tech, "English"),
    ] {
        let (first, second) = tokio::join!(
            snapshot.read_article(&category, &slug),
            snapshot.read_article(&category, &slug)
        );
        let first = first.unwrap().unwrap();
        let second = second.unwrap().unwrap();
        assert_eq!(first.summary.title, expected);
        assert_eq!(first.html, format!("<p>{expected}</p>"));
        assert!(Arc::ptr_eq(&first, &second));
    }
}

#[tokio::test]
async fn failed_article_load_is_retried_without_caching_the_error() {
    let root = TempDir::new().unwrap();
    write_article(root.path(), "", "tech", "intro", "Initial");
    let path = root.path().join("articles/tech/intro.json");
    fs::write(&path, "invalid").unwrap();
    let reader = CachingArtifactReader::new(
        Arc::new(LocalArtifactReader::new(root.path())),
        Duration::from_secs(60),
    );
    let snapshot = reader.snapshot().await.unwrap();
    let slug: Slug = "intro".parse().unwrap();
    assert!(snapshot.read_article(&Category::Tech, &slug).await.is_err());
    write_article(root.path(), "", "tech", "intro", "Recovered");
    assert_eq!(
        snapshot
            .read_article(&Category::Tech, &slug)
            .await
            .unwrap()
            .unwrap()
            .summary
            .title,
        "Recovered"
    );
}

#[tokio::test]
async fn missing_article_json_returns_none_without_reading_index_or_html() {
    let root = TempDir::new().unwrap();
    let reader = LocalArtifactReader::new(root.path());
    let slug = "intro".parse().unwrap();
    assert!(
        reader
            .read_article(&Category::Tech, &slug)
            .await
            .unwrap()
            .is_none()
    );
    fs::create_dir_all(root.path().join("articles/tech")).unwrap();
    fs::write(
        root.path().join("articles/index.json"),
        serde_json::to_vec(&serde_json::json!({
            "articles": [summary("tech", "intro", "Intro")]
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        root.path().join("articles/tech/intro.html"),
        "<p>Unsupported format</p>",
    )
    .unwrap();
    assert!(
        reader
            .read_article(&Category::Tech, &slug)
            .await
            .unwrap()
            .is_none()
    );
}

#[rstest]
#[case::malformed_json(serde_json::json!(null))]
#[case::wrong_category(serde_json::json!({"summary": summary("daily", "intro", "Other"), "html": "<p>Other</p>"}))]
#[case::wrong_slug(serde_json::json!({"summary": summary("tech", "other", "Other"), "html": "<p>Other</p>"}))]
#[case::blank_html(serde_json::json!({"summary": summary("tech", "intro", "Empty"), "html": "  "}))]
#[case::invalid_metadata(serde_json::json!({"summary": summary("tech", "intro", ""), "html": "<p>Other</p>"}))]
#[tokio::test]
async fn invalid_article_json_is_an_error(#[case] document: serde_json::Value) {
    let root = TempDir::new().unwrap();
    fs::create_dir_all(root.path().join("articles/tech")).unwrap();
    fs::write(
        root.path().join("articles/tech/intro.json"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    let reader = LocalArtifactReader::new(root.path());
    assert!(
        reader
            .read_article(&Category::Tech, &"intro".parse().unwrap())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn cached_articles_catalogs_labels_and_assets_share_their_storage() {
    let root = TempDir::new().unwrap();
    write_article(root.path(), "", "tech", "intro", "Intro");
    fs::create_dir_all(root.path().join("content-assets")).unwrap();
    fs::write(root.path().join("tags.json"), r#"{"rust":"Rust"}"#).unwrap();
    fs::write(
        root.path().join("locales.json"),
        r#"{"schema_version":1,"routes":{"/":["ja","en"]}}"#,
    )
    .unwrap();
    let name = blog_domain::ContentAssetName::new(format!("{}.png", "a".repeat(64))).unwrap();
    fs::write(
        root.path().join("content-assets").join(name.as_str()),
        b"image",
    )
    .unwrap();
    let reader = CachingArtifactReader::new(
        Arc::new(LocalArtifactReader::new(root.path())),
        Duration::from_secs(60),
    );
    let snapshot = reader.snapshot().await.unwrap();
    let slug: Slug = "intro".parse().unwrap();
    let first = snapshot
        .read_article(&Category::Tech, &slug)
        .await
        .unwrap()
        .unwrap();
    let locales = snapshot.read_locales().await.unwrap();
    fs::write(root.path().join("articles/tech/intro.json"), "changed").unwrap();
    fs::write(root.path().join("locales.json"), "changed").unwrap();
    let second = snapshot
        .read_article(&Category::Tech, &slug)
        .await
        .unwrap()
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(Arc::ptr_eq(
        &locales,
        &snapshot.read_locales().await.unwrap()
    ));
    assert!(Arc::ptr_eq(
        &snapshot.read_tag_labels().await.unwrap(),
        &snapshot.read_tag_labels().await.unwrap()
    ));
    let first = snapshot.read_content_asset(&name).await.unwrap().unwrap();
    let second = snapshot.read_content_asset(&name).await.unwrap().unwrap();
    assert_eq!(first.as_ptr(), second.as_ptr());
}

#[tokio::test]
async fn optional_catalogs_default_only_when_missing() {
    let root = TempDir::new().unwrap();
    let reader = LocalArtifactReader::new(root.path());
    assert_eq!(
        reader.read_locales().await.unwrap(),
        SiteLocalesDocument::default().into()
    );
    assert!(reader.read_tag_labels().await.unwrap().is_empty());
    fs::write(root.path().join("tags.json"), r#"{"rust":"Rust articles"}"#).unwrap();
    assert_eq!(
        reader
            .read_tag_labels()
            .await
            .unwrap()
            .get("rust")
            .map(String::as_str),
        Some("Rust articles")
    );
}

#[rstest]
#[case::malformed_json(b"invalid json")]
#[case::invalid_utf8(&[255])]
#[tokio::test]
async fn optional_catalogs_reject_invalid_content(#[case] content: &[u8]) {
    let root = TempDir::new().unwrap();
    let reader = LocalArtifactReader::new(root.path());
    fs::write(root.path().join("locales.json"), content).unwrap();
    fs::write(root.path().join("tags.json"), content).unwrap();
    assert!(reader.read_locales().await.is_err());
    assert!(reader.read_tag_labels().await.is_err());
}
