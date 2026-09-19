use domain::{ContentKind, Locale, PublicContentMeta};

fn metadata() -> PublicContentMeta {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1, "id": "legacy123456", "locale": "ja",
        "kind": "article", "title": "記事", "category": "tech",
        "created": "2025-01-01T00:00:00+09:00",
        "updated": "2025-01-01T00:00:00+09:00",
        "source_hash": "a".repeat(64)
    }))
    .unwrap()
}

#[test]
fn locale_preserves_japanese_routes_and_separates_english_keys() {
    assert_eq!(Locale::Ja.path("/tech/legacy123456"), "/tech/legacy123456");
    assert_eq!(
        Locale::En.path("/tech/legacy123456"),
        "/en/tech/legacy123456"
    );
    assert_eq!(Locale::En.path("/"), "/en/");
    assert_eq!(
        Locale::En.artifact_key("articles/index.json"),
        "en/articles/index.json"
    );
    assert!("fr".parse::<Locale>().is_err());
}

#[test]
fn public_contract_checks_schema_kind_and_provenance() {
    let mut meta = metadata();
    meta.validate().unwrap();
    assert_eq!(meta.path(), "/tech/legacy123456");
    meta.schema_version = 2;
    assert!(meta.validate().is_err());
    meta.schema_version = 1;
    meta.category = None;
    assert!(meta.validate().is_err());
    meta.kind = ContentKind::Home;
    meta.validate().unwrap();
    meta.source_hash = "private/path.md".into();
    assert!(meta.validate().is_err());
}

#[test]
fn unknown_frontmatter_is_not_part_of_the_public_contract() {
    let mut value = serde_json::to_value(metadata()).unwrap();
    value["private_notes"] = "secret".into();
    assert!(serde_json::from_value::<PublicContentMeta>(value).is_err());
}

#[test]
fn translation_provenance_requires_an_explicit_stale_state() {
    let mut value = serde_json::to_value(metadata()).unwrap();
    value["locale"] = "en".into();
    value["translation"] = serde_json::json!({
        "input_hash": "b".repeat(64),
        "generated_hash": "c".repeat(64)
    });
    assert!(serde_json::from_value::<PublicContentMeta>(value.clone()).is_err());
    for stale in [true, false] {
        value["translation"]["stale"] = stale.into();
        let parsed = serde_json::from_value::<PublicContentMeta>(value.clone()).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed.translation.unwrap().stale, stale);
    }
}
