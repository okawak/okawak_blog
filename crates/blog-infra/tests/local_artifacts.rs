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

#[tokio::test]
async fn indexed_article_lookups_preserve_first_match_and_isolate_locales_and_categories() {
    let root = TempDir::new().unwrap();
    for directory in ["articles", "en/articles"] {
        fs::create_dir_all(root.path().join(directory)).unwrap();
    }
    fs::write(
        root.path().join("locales.json"),
        r#"{"schema_version":1,"routes":{"/":["ja","en"]}}"#,
    )
    .unwrap();
    fs::write(root.path().join("articles/index.json"), serde_json::to_vec(&serde_json::json!({
        "articles": [summary("tech", "same", "First"), summary("daily", "same", "Daily"), summary("tech", "same", "Duplicate")]
    })).unwrap()).unwrap();
    fs::write(
        root.path().join("en/articles/index.json"),
        serde_json::to_vec(&serde_json::json!({
            "articles": [summary("tech", "same", "English")]
        }))
        .unwrap(),
    )
    .unwrap();
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
        (&ja, Category::Tech, "First"),
        (&ja, Category::Daily, "Daily"),
        (&en, Category::Tech, "English"),
    ] {
        let (first, second) = tokio::join!(
            snapshot.read_article_summary(&category, &slug),
            snapshot.read_article_summary(&category, &slug)
        );
        assert_eq!(first.unwrap().unwrap().title, expected);
        assert_eq!(second.unwrap().unwrap().title, expected);
    }
    assert!(
        ja.read_article_summary(&Category::Physics, &slug)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        ja.read_article_summary(&Category::Tech, &"missing".parse().unwrap())
            .await
            .unwrap()
            .is_none()
    );
    fs::remove_file(root.path().join("articles/index.json")).unwrap();
    assert_eq!(
        ja.read_article_summary(&Category::Tech, &slug)
            .await
            .unwrap()
            .unwrap()
            .title,
        "First"
    );
}

#[tokio::test]
async fn failed_index_load_is_retried_before_building_the_lookup() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("articles")).unwrap();
    let path = root.path().join("articles/index.json");
    fs::write(&path, "invalid").unwrap();
    let reader = CachingArtifactReader::new(
        Arc::new(LocalArtifactReader::new(root.path())),
        Duration::from_secs(60),
    );
    let snapshot = reader.snapshot().await.unwrap();
    let slug: Slug = "intro".parse().unwrap();
    assert!(
        snapshot
            .read_article_summary(&Category::Tech, &slug)
            .await
            .is_err()
    );
    fs::write(
        path,
        serde_json::to_vec(
            &serde_json::json!({"articles": [summary("tech", "intro", "Recovered")]}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        snapshot
            .read_article_summary(&Category::Tech, &slug)
            .await
            .unwrap()
            .unwrap()
            .title,
        "Recovered"
    );
}

#[tokio::test]
async fn cached_html_labels_and_assets_share_their_storage() {
    let root = TempDir::new().unwrap();
    for directory in ["articles/tech", "articles/daily", "content-assets"] {
        fs::create_dir_all(root.path().join(directory)).unwrap();
    }
    for category in ["tech", "daily"] {
        fs::write(
            root.path().join(format!("articles/{category}/same.html")),
            category,
        )
        .unwrap();
    }
    fs::write(root.path().join("tags.json"), r#"{"rust":"Rust"}"#).unwrap();
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
    let slug: Slug = "same".parse().unwrap();
    let first = snapshot
        .read_article_html(&Category::Tech, &slug)
        .await
        .unwrap();
    let second = snapshot
        .read_article_html(&Category::Tech, &slug)
        .await
        .unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(
        snapshot
            .read_article_html(&Category::Daily, &slug)
            .await
            .unwrap()
            .as_str(),
        "daily"
    );
    assert!(Arc::ptr_eq(
        &snapshot.read_tag_labels().await.unwrap(),
        &snapshot.read_tag_labels().await.unwrap()
    ));
    assert!(Arc::ptr_eq(
        &snapshot.read_locales().await.unwrap(),
        &snapshot.read_locales().await.unwrap()
    ));
    let first = snapshot.read_content_asset(&name).await.unwrap().unwrap();
    let second = snapshot.read_content_asset(&name).await.unwrap().unwrap();
    assert_eq!(first.as_ptr(), second.as_ptr());
}

#[tokio::test]
async fn caches_nonempty_locale_catalogs_and_article_html() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("articles/tech")).unwrap();
    let catalog = r#"{"schema_version":1,"routes":{"/":["ja","en"]}}"#;
    fs::write(root.path().join("locales.json"), catalog).unwrap();
    fs::write(root.path().join("articles/tech/intro.html"), "<p>Intro</p>").unwrap();
    let reader = CachingArtifactReader::new(
        Arc::new(LocalArtifactReader::new(root.path())),
        Duration::from_secs(60),
    );
    let snapshot = reader.snapshot().await.unwrap();
    let slug = Slug::new("intro".into()).unwrap();
    for _ in 0..2 {
        assert_eq!(
            snapshot.read_locales().await.unwrap(),
            serde_json::from_str::<SiteLocalesDocument>(catalog)
                .unwrap()
                .into()
        );
        assert_eq!(
            snapshot
                .read_article_html(&Category::Tech, &slug)
                .await
                .unwrap()
                .as_str(),
            "<p>Intro</p>"
        );
        fs::write(root.path().join("locales.json"), "corrupt after first read").unwrap();
        fs::write(root.path().join("articles/tech/intro.html"), "changed").unwrap();
    }
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
