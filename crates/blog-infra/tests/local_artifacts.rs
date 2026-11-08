use blog_domain::{Category, SiteLocalesDocument, Slug};
use blog_infra::{ArtifactReader, ArtifactSnapshot, CachingArtifactReader, LocalArtifactReader};
use rstest::rstest;
use std::{fs, sync::Arc, time::Duration};
use tempfile::TempDir;

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
            serde_json::from_str::<SiteLocalesDocument>(catalog).unwrap()
        );
        assert_eq!(
            snapshot
                .read_article_html(&Category::Tech, &slug)
                .await
                .unwrap(),
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
        SiteLocalesDocument::default()
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
