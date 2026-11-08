use super::*;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region, retry::RetryConfig};
use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};
use aws_smithy_types::body::SdkBody;

fn event(key: &str, status: u16, body: impl Into<SdkBody>) -> ReplayEvent {
    ReplayEvent::new(
        http::Request::builder()
            .uri(format!(
                "https://s3.example.test/bucket/public/{key}?x-id=GetObject"
            ))
            .body(SdkBody::empty())
            .unwrap(),
        http::Response::builder()
            .status(status)
            .body(body.into())
            .unwrap(),
    )
}

fn missing(key: &str) -> ReplayEvent {
    event(
        key,
        404,
        "<Error><Code>NoSuchKey</Code><Message>missing</Message></Error>",
    )
}

fn reader(events: Vec<ReplayEvent>) -> (S3ArtifactReader, StaticReplayClient) {
    let transport = StaticReplayClient::new(events);
    let config = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .region(Region::new("us-east-1"))
        .credentials_provider(Credentials::new("test", "test", None, None, "fixture"))
        .endpoint_url("https://s3.example.test")
        .force_path_style(true)
        .retry_config(RetryConfig::disabled())
        .http_client(transport.clone())
        .build();
    (
        S3ArtifactReader::new(
            Client::from_conf(config),
            S3ArtifactLocation::new("bucket", Some("public")).unwrap(),
        ),
        transport,
    )
}

#[tokio::test]
async fn release_snapshot_preserves_identity_localized_text_and_shared_binary_assets() {
    let pointer = ArtifactReleasePointerDocument {
        schema_version: 1,
        release_id: "one".into(),
        artifact_prefix: "releases/one/site".into(),
        publisher_commit: "publisher".into(),
        source_commit: "source".into(),
        content_commit: None,
        generated_at: "2026-01-01T00:00:00Z".into(),
    };
    let locales = r#"{"schema_version":1,"routes":{"/":["ja","en"],"/tech/intro":["ja","en"]}}"#;
    let name = ContentAssetName::new(format!("{}.png", "a".repeat(64))).unwrap();
    let (reader, transport) = reader(vec![
        event(
            "current.json",
            200,
            serde_json::to_string(&pointer).unwrap(),
        ),
        event("releases/one/site/locales.json", 200, locales),
        event("releases/one/site/locales.json", 200, locales),
        event(
            "releases/one/site/en/tags.json",
            200,
            r#"{"rust":"Rust articles"}"#,
        ),
        event(
            "releases/one/site/en/articles/tech/intro.html",
            200,
            "<p>English 日本</p>",
        ),
        event(
            &format!("releases/one/site/content-assets/{}", name.as_str()),
            200,
            vec![0, 255, 10],
        ),
        event("releases/one/site/locales.json", 200, locales),
    ]);
    let snapshot = reader.snapshot().await.unwrap();
    assert_eq!(snapshot.cache_identity(), Some("releases/one/site"));
    assert_eq!(
        snapshot.last_modified(),
        Some(pointer.generated_at_time().unwrap())
    );
    let catalog = snapshot.read_locales().await.unwrap();
    assert_eq!(
        catalog.path("/tech/intro", Locale::En).as_deref(),
        Some("/en/tech/intro")
    );
    assert!(snapshot.localized(Locale::Ja).await.unwrap().is_some());
    let english = snapshot.localized(Locale::En).await.unwrap().unwrap();
    assert_eq!(english.cache_identity(), snapshot.cache_identity());
    assert_eq!(english.last_modified(), snapshot.last_modified());
    assert_eq!(
        english.read_tag_labels().await.unwrap(),
        [("rust".into(), "Rust articles".into())].into()
    );
    assert_eq!(
        english
            .read_article_html(&Category::Tech, &Slug::new("intro".into()).unwrap())
            .await
            .unwrap(),
        "<p>English 日本</p>"
    );
    assert_eq!(
        english.read_content_asset(&name).await.unwrap(),
        Some(vec![0, 255, 10])
    );
    assert_eq!(english.read_locales().await.unwrap(), catalog);
    transport.assert_requests_match(&[]);
}

#[tokio::test]
async fn missing_pointer_and_optional_objects_keep_legacy_japanese_behavior() {
    let name = ContentAssetName::new(format!("{}.png", "b".repeat(64))).unwrap();
    let (reader, transport) = reader(vec![
        missing("current.json"),
        missing("locales.json"),
        missing("tags.json"),
        missing(&format!("content-assets/{}", name.as_str())),
        missing("locales.json"),
    ]);
    let snapshot = reader.snapshot().await.unwrap();
    assert_eq!(snapshot.cache_identity(), None);
    assert_eq!(snapshot.last_modified(), None);
    assert_eq!(
        snapshot.read_locales().await.unwrap(),
        SiteLocalesDocument::default()
    );
    assert!(snapshot.read_tag_labels().await.unwrap().is_empty());
    assert_eq!(snapshot.read_content_asset(&name).await.unwrap(), None);
    assert!(snapshot.localized(Locale::Ja).await.unwrap().is_some());
    assert!(snapshot.localized(Locale::En).await.unwrap().is_none());
    transport.assert_requests_match(&[]);
}

#[tokio::test]
async fn pointer_errors_do_not_fall_back_to_legacy_storage() {
    for (status, body) in [
        (403, "<Error><Code>AccessDenied</Code></Error>"),
        (200, "invalid json"),
    ] {
        let (reader, transport) = reader(vec![event("current.json", status, body)]);
        assert!(reader.snapshot().await.is_err());
        transport.assert_requests_match(&[]);
    }
}

#[tokio::test]
async fn optional_objects_propagate_access_and_decoding_errors() {
    let name = ContentAssetName::new(format!("{}.png", "c".repeat(64))).unwrap();
    for key in [
        "locales.json".to_owned(),
        "tags.json".into(),
        format!("content-assets/{}", name.as_str()),
    ] {
        let (reader, transport) = reader(vec![
            missing("current.json"),
            event(&key, 403, "<Error><Code>AccessDenied</Code></Error>"),
        ]);
        let snapshot = reader.snapshot().await.unwrap();
        let failed = match key.as_str() {
            "locales.json" => snapshot.read_locales().await.is_err(),
            "tags.json" => snapshot.read_tag_labels().await.is_err(),
            _ => snapshot.read_content_asset(&name).await.is_err(),
        };
        assert!(failed, "{key}");
        transport.assert_requests_match(&[]);
    }
    for body in [b"invalid json".to_vec(), vec![255]] {
        let (reader, transport) =
            reader(vec![missing("current.json"), event("tags.json", 200, body)]);
        assert!(
            reader
                .snapshot()
                .await
                .unwrap()
                .read_tag_labels()
                .await
                .is_err()
        );
        transport.assert_requests_match(&[]);
    }
}
