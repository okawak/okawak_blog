use domain::{LabelCatalog, LabelEntry};
use export::{Texts, TranslationRequest, TranslationSettings, Translator};
use std::{cell::Cell, fs};

struct Fake(Cell<usize>);
impl Translator for Fake {
    fn translate(&self, request: &TranslationRequest) -> anyhow::Result<Texts> {
        self.0.set(self.0.get() + 1);
        Ok(request
            .texts
            .iter()
            .map(|(k, v)| (k.clone(), format!("EN {v}")))
            .collect())
    }
}
fn settings() -> TranslationSettings {
    TranslationSettings {
        model: "fake".into(),
        instruction: "translate".into(),
        glossary: Texts::new(),
    }
}
fn save(path: &std::path::Path, catalog: &LabelCatalog) {
    fs::write(path, serde_json::to_vec_pretty(catalog).unwrap()).unwrap();
}
#[test]
fn catalogs_reuse_each_entry_and_protect_manual_edits_until_candidate_acceptance() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("ui.json");
    let mut catalog = LabelCatalog::default();
    catalog.entries.insert(
        "count".into(),
        LabelEntry {
            source: "{count}件".into(),
            context: "article count".into(),
            translation: None,
        },
    );
    save(&path, &catalog);
    let fake = Fake(Cell::new(0));
    export::translate_catalog(&path, &fake, &settings(), false).unwrap();
    export::translate_catalog(&path, &fake, &settings(), false).unwrap();
    assert_eq!(fake.0.get(), 1);
    catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    catalog
        .entries
        .get_mut("count")
        .unwrap()
        .translation
        .as_mut()
        .unwrap()
        .value = "{count} manually edited".into();
    catalog.entries.insert(
        "new".into(),
        LabelEntry {
            source: "新規".into(),
            context: "button".into(),
            translation: None,
        },
    );
    save(&path, &catalog);
    export::translate_catalog(&path, &fake, &settings(), false).unwrap();
    assert_eq!(
        fake.0.get(),
        2,
        "adding a key must not retranslate existing keys"
    );
    catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    catalog.entries.get_mut("count").unwrap().context = "filtered article count".into();
    save(&path, &catalog);
    let report = export::translate_catalog(&path, &fake, &settings(), true).unwrap();
    assert_eq!(report.protected, ["count"]);
    let protected: LabelCatalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        protected.entries["count"]
            .translation
            .as_ref()
            .unwrap()
            .value,
        "{count} manually edited"
    );
    assert!(
        protected.entries["count"]
            .translation
            .as_ref()
            .unwrap()
            .stale
    );
    export::accept_catalog_translation(&path, "count", &settings()).unwrap();
    export::translate_catalog(&path, &fake, &settings(), false).unwrap();
    assert_eq!(fake.0.get(), 3);
}

#[test]
fn invalid_placeholder_response_leaves_catalog_unchanged() {
    struct Broken;
    impl Translator for Broken {
        fn translate(&self, _: &TranslationRequest) -> anyhow::Result<Texts> {
            Ok(Texts::from([("value".into(), "lost placeholder".into())]))
        }
    }
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("ui.json");
    let catalog = LabelCatalog {
        entries: [(
            "count".into(),
            LabelEntry {
                source: "{count}件".into(),
                context: "count".into(),
                translation: None,
            },
        )]
        .into(),
        ..Default::default()
    };
    save(&path, &catalog);
    let before = fs::read(&path).unwrap();
    assert!(export::translate_catalog(&path, &Broken, &settings(), false).is_err());
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn tags_are_translated_once_per_id_and_removed_with_unpublished_sources() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join("tech")).unwrap();
    for (name, date) in [("a", "2025-01-01"), ("b", "2025-01-02")] {
        fs::write(source.path().join(format!("tech/{name}.md")), format!("---\ntitle: {name}\ncategory: tech\nis_completed: true\ntags: [統計]\ncreated: '{date}T00:00:00+09:00'\nupdated: '{date}T00:00:00+09:00'\n---\n本文\n")).unwrap();
    }
    export::export_japanese(source.path(), output.path()).unwrap();
    let fake = Fake(Cell::new(0));
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    assert_eq!(fake.0.get(), 3, "two articles and one shared tag");
    let path = output.path().join("tags.json");
    let catalog: LabelCatalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(catalog.entries["統計"].value(domain::Locale::En), "EN 統計");
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    assert_eq!(fake.0.get(), 3);
    fs::remove_dir_all(source.path().join("tech")).unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let catalog: LabelCatalog = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert!(catalog.entries.is_empty());
    assert!(
        fs::read_dir(output.path().join(".export-archive"))
            .unwrap()
            .any(|file| file
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("tags-"))
    );
}

#[test]
fn authored_labels_without_generation_history_are_preserved_and_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("ui.json");
    let catalog = LabelCatalog {
        entries: [(
            "button".into(),
            LabelEntry {
                source: "実行".into(),
                context: "button".into(),
                translation: Some(domain::LabelTranslation {
                    value: "Run".into(),
                    provenance: None,
                    stale: false,
                }),
            },
        )]
        .into(),
        ..Default::default()
    };
    save(&path, &catalog);
    let fake = Fake(Cell::new(0));
    let report = export::translate_catalog(&path, &fake, &settings(), false).unwrap();
    assert_eq!(fake.0.get(), 0);
    assert_eq!(report.protected, ["button"]);
    let actual: LabelCatalog = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(actual, catalog);
}
