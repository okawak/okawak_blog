use domain::{Locale, SiteLocalesDocument};
use infra::{ArtifactReader, LocalArtifactReader};
use std::fs;

#[tokio::test]
async fn locale_snapshots_read_separate_indexes_and_legacy_releases_remain_readable() {
    let temp = tempfile::TempDir::new().unwrap();
    fs::create_dir_all(temp.path().join("articles")).unwrap();
    fs::write(temp.path().join("articles/index.json"), "{\"articles\":[]}").unwrap();
    let reader = LocalArtifactReader::new(temp.path());
    let snapshot = reader.snapshot().await.unwrap();
    assert!(snapshot.read_locales().await.unwrap().routes.is_empty());
    assert!(snapshot.localized(Locale::En).await.unwrap().is_none());
    let mut locales = SiteLocalesDocument::default();
    locales
        .routes
        .insert("/".into(), vec![Locale::Ja, Locale::En]);
    fs::write(
        temp.path().join("locales.json"),
        serde_json::to_vec(&locales).unwrap(),
    )
    .unwrap();
    fs::create_dir_all(temp.path().join("en/articles")).unwrap();
    fs::write(
        temp.path().join("en/articles/index.json"),
        "{\"articles\":[]}",
    )
    .unwrap();
    fs::write(temp.path().join("tags.json"), r#"{"統計":"統計"}"#).unwrap();
    fs::write(temp.path().join("en/tags.json"), r#"{"統計":"Statistics"}"#).unwrap();
    let en = snapshot.localized(Locale::En).await.unwrap().unwrap();
    assert_eq!(snapshot.read_tag_labels().await.unwrap()["統計"], "統計");
    assert_eq!(en.read_tag_labels().await.unwrap()["統計"], "Statistics");
    assert!(en.read_article_index().await.unwrap().articles.is_empty());
    fs::write(temp.path().join("en/articles/index.json"), "broken").unwrap();
    assert!(en.read_article_index().await.is_err());
    assert!(snapshot.read_article_index().await.is_ok());
}

#[tokio::test]
async fn cache_reuses_each_locale_without_mixing_indexes_or_assets() {
    use std::{sync::Arc, time::Duration};
    let temp = tempfile::TempDir::new().unwrap();
    for prefix in ["", "en/"] {
        fs::create_dir_all(temp.path().join(format!("{prefix}articles"))).unwrap();
        fs::write(
            temp.path().join(format!("{prefix}articles/index.json")),
            "{\"articles\":[]}",
        )
        .unwrap();
    }
    fs::write(
        temp.path().join("locales.json"),
        r#"{"schema_version":1,"routes":{"/":["ja","en"]}}"#,
    )
    .unwrap();
    let name = domain::ContentAssetName::new(format!("{}.png", "f".repeat(64))).unwrap();
    fs::create_dir_all(temp.path().join("content-assets")).unwrap();
    fs::write(
        temp.path().join("content-assets").join(name.as_str()),
        [0, 255, 1],
    )
    .unwrap();
    let reader = infra::CachingArtifactReader::new(
        Arc::new(LocalArtifactReader::new(temp.path())),
        Duration::from_secs(60),
    );
    let snapshot = reader.snapshot().await.unwrap();
    let ja = snapshot.localized(Locale::Ja).await.unwrap().unwrap();
    fs::write(temp.path().join("tags.json"), r#"{"統計":"統計"}"#).unwrap();
    fs::write(temp.path().join("en/tags.json"), r#"{"統計":"Statistics"}"#).unwrap();
    let en = snapshot.localized(Locale::En).await.unwrap().unwrap();
    assert_eq!(snapshot.read_tag_labels().await.unwrap()["統計"], "統計");
    assert_eq!(en.read_tag_labels().await.unwrap()["統計"], "Statistics");
    let again = snapshot.localized(Locale::En).await.unwrap().unwrap();
    assert!(!Arc::ptr_eq(&ja, &en));
    assert!(Arc::ptr_eq(&en, &again));
    en.read_article_index().await.unwrap();
    fs::write(temp.path().join("en/articles/index.json"), "broken").unwrap();
    assert!(en.read_article_index().await.is_ok());
    assert!(ja.read_article_index().await.is_ok());
    assert_eq!(
        en.read_content_asset(&name).await.unwrap(),
        Some(vec![0, 255, 1])
    );
}
