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
    assert_eq!(Locale::En.path("/"), "/en");
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
}

#[test]
fn unknown_frontmatter_is_not_part_of_the_public_contract() {
    let mut value = serde_json::to_value(metadata()).unwrap();
    value["private_notes"] = "secret".into();
    assert!(serde_json::from_value::<PublicContentMeta>(value).is_err());
}

#[test]
fn invalid_field_values_are_rejected_during_deserialization() {
    for (field, invalid) in [
        ("title", serde_json::json!("   ")),
        ("title", serde_json::json!("あ".repeat(201))),
        ("created", serde_json::json!("2025-01-01")),
        ("updated", serde_json::json!("2025-02-30T00:00:00+09:00")),
        ("tags", serde_json::json!(["   "])),
        ("tags", serde_json::json!(["tag\nname"])),
        ("source_hash", serde_json::json!("private/path.md")),
        ("section_path", serde_json::json!([".."])),
        ("section_path", serde_json::json!(["rust/async"])),
    ] {
        let mut value = serde_json::to_value(metadata()).unwrap();
        value[field] = invalid;
        assert!(
            serde_json::from_value::<PublicContentMeta>(value).is_err(),
            "invalid {field} was accepted"
        );
    }
}

#[test]
fn validated_fields_preserve_the_public_representation() {
    let mut value = serde_json::to_value(metadata()).unwrap();
    value["title"] = "  記事  ".into();
    value["created"] = " 2025-01-01T00:00:00+09:00 ".into();
    value["updated"] = "2025-01-02T03:04:05.123Z".into();
    value["tags"] = serde_json::json!(["Rust", "日本語", " tag "]);
    value["section_path"] = serde_json::json!(["rust", "非同期"]);
    value["source_hash"] = "A".repeat(64).into();
    let parsed: PublicContentMeta = serde_json::from_value(value.clone()).unwrap();
    parsed.validate().unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}

#[test]
fn invalid_translation_hashes_are_rejected_during_deserialization() {
    for field in ["input_hash", "generated_hash"] {
        let mut value = serde_json::to_value(metadata()).unwrap();
        value["locale"] = "en".into();
        value["translation"] = serde_json::json!({
            "input_hash": "b".repeat(64),
            "generated_hash": "c".repeat(64),
            "stale": false,
        });
        value["translation"][field] = "g".repeat(64).into();
        assert!(serde_json::from_value::<PublicContentMeta>(value).is_err());
    }
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
        let mut parsed = serde_json::from_value::<PublicContentMeta>(value.clone()).unwrap();
        parsed.validate().unwrap();
        assert_eq!(parsed.translation.as_ref().unwrap().stale, stale);
        parsed.locale = Locale::Ja;
        assert!(parsed.validate().is_err());
    }
}
