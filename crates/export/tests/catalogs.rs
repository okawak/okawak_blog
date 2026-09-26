use domain::{LabelCatalog, LabelEntry};
use export::{ProtectedContent, Texts, TranslationRequest, TranslationSettings, Translator};
use std::{cell::Cell, fs};

struct Fake(Cell<usize>);
impl Translator for Fake {
    fn translate(&self, request: &TranslationRequest) -> export::Result<Texts> {
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
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    export::translate_catalog(&path, &fake, &settings()).unwrap();
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
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    assert_eq!(
        fake.0.get(),
        2,
        "adding a key must not retranslate existing keys"
    );
    catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    catalog.entries.get_mut("count").unwrap().context = "filtered article count".into();
    save(&path, &catalog);
    let report = export::translate_catalog(&path, &fake, &settings()).unwrap();
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
    export::accept_catalog_candidate(&path, "count", &settings()).unwrap();
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    assert_eq!(fake.0.get(), 3);
}

#[test]
fn invalid_placeholder_response_leaves_catalog_unchanged() {
    struct Broken;
    impl Translator for Broken {
        fn translate(&self, _: &TranslationRequest) -> export::Result<Texts> {
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
    assert!(matches!(
        export::translate_catalog(&path, &Broken, &settings()),
        Err(export::ExportError::InvalidTranslation(_))
    ));
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn changed_source_placeholders_regenerate_or_protect_existing_translations() {
    for source in ["{count}/{total}件", "記事数", "{total}件"] {
        for mode in ["generated", "manual", "unknown"] {
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
            export::translate_catalog(&path, &fake, &settings()).unwrap();
            catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            let entry = catalog.entries.get_mut("count").unwrap();
            let translation = entry.translation.as_mut().unwrap();
            if mode == "manual" {
                translation.value = "{count} custom".into();
            }
            if mode == "unknown" {
                translation.provenance = None;
            }
            let previous = translation.value.clone();
            entry.source = source.into();
            save(&path, &catalog);
            let report = export::translate_catalog(&path, &fake, &settings()).unwrap();
            catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            catalog.validate().unwrap();
            if mode == "generated" {
                assert_eq!(report.generated, 1);
                assert_eq!(fake.0.get(), 2);
                assert_eq!(
                    catalog.entries["count"].value(domain::Locale::En),
                    format!("EN {source}")
                );
            } else {
                assert_eq!(report.protected, ["count"]);
                assert_eq!(report.generated, 1);
                assert_eq!(fake.0.get(), 2);
                let translation = catalog.entries["count"].translation.as_ref().unwrap();
                assert_eq!(translation.value, previous);
                assert!(translation.stale);
                assert_eq!(catalog.entries["count"].value(domain::Locale::En), source);
                export::translate_catalog(&path, &fake, &settings()).unwrap();
                export::accept_catalog_candidate(&path, "count", &settings()).unwrap();
            }
            export::translate_catalog(&path, &fake, &settings()).unwrap();
            assert_eq!(
                fake.0.get(),
                2,
                "unchanged input must reuse the updated translation"
            );
        }
    }
}

#[test]
fn tags_are_translated_once_per_id_and_removed_with_unpublished_sources() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join("tech")).unwrap();
    for (name, date) in [("a", "2025-01-01"), ("b", "2025-01-02")] {
        fs::write(source.path().join(format!("tech/{name}.md")), format!("---\ntitle: {name}\ncategory: tech\nis_completed: true\ntags: [統計]\ncreated: '{date}T00:00:00+09:00'\nupdated: '{date}T00:00:00+09:00'\n---\n本文\n")).unwrap();
    }
    let fake = Fake(Cell::new(0));
    export::export_content(source.path(), output.path(), &fake, &settings()).unwrap();
    assert_eq!(fake.0.get(), 3, "two articles and one shared tag");
    let path = output.path().join("tags.json");
    let mut catalog: LabelCatalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(catalog.entries["統計"].value(domain::Locale::En), "EN 統計");
    let tag = catalog.entries.get_mut("統計").unwrap();
    tag.translation.as_mut().unwrap().value = "Manual label".into();
    tag.context = "Updated tag context".into();
    save(&path, &catalog);
    let report = export::export_content(source.path(), output.path(), &fake, &settings()).unwrap();
    assert_eq!(report.protected, [ProtectedContent::Tag("統計".into())]);
    assert_eq!(fake.0.get(), 4);
    fs::remove_dir_all(source.path().join("tech")).unwrap();
    export::export_content(source.path(), output.path(), &fake, &settings()).unwrap();
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
fn blocked_tag_candidates_stop_before_translating_any_article() {
    let source = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join("tech")).unwrap();
    fs::write(source.path().join("tech/a.md"), "---\ntitle: Article\ncategory: tech\nis_completed: true\ntags: [統計]\ncreated: '2025-01-01T00:00:00+09:00'\nupdated: '2025-01-01T00:00:00+09:00'\n---\n本文\n").unwrap();
    let path = output.path().join("tags.json");
    let fake = Fake(Cell::new(0));
    export::export_content(source.path(), output.path(), &fake, &settings()).unwrap();
    let mut catalog: LabelCatalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let entry = catalog.entries.get_mut("統計").unwrap();
    entry.translation.as_mut().unwrap().value = "Manual statistics".into();
    entry.context = "second context".into();
    save(&path, &catalog);
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    let candidate = fs::read_dir(output.path().join(".export-candidates/catalog"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut reviewed: serde_json::Value =
        serde_json::from_slice(&fs::read(&candidate).unwrap()).unwrap();
    reviewed["translation"]["value"] = "Reviewed statistics".into();
    fs::write(&candidate, serde_json::to_vec_pretty(&reviewed).unwrap()).unwrap();
    catalog.entries.get_mut("統計").unwrap().context = "third context".into();
    save(&path, &catalog);
    let before = fs::read(&path).unwrap();
    let candidate_before = fs::read(&candidate).unwrap();
    let english = fs::read_dir(output.path().join("en"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let english_before = fs::read(&english).unwrap();
    let calls = fake.0.get();
    let note = source.path().join("tech/a.md");
    fs::write(
        &note,
        fs::read_to_string(&note)
            .unwrap()
            .replace("本文", "更新した本文"),
    )
    .unwrap();
    assert!(export::export_content(source.path(), output.path(), &fake, &settings()).is_err());
    assert_eq!(fake.0.get(), calls);
    assert_eq!(fs::read(english).unwrap(), english_before);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read(&candidate).unwrap(), candidate_before);
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
    let report = export::translate_catalog(&path, &fake, &settings()).unwrap();
    assert_eq!(fake.0.get(), 1);
    assert_eq!(report.generated, 1);
    assert_eq!(report.protected, ["button"]);
    let actual: LabelCatalog = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(actual, catalog);
}

#[test]
fn filename_only_catalog_updates_and_accepts_without_replacing_working_directory() {
    const CHILD: &str = "OKAWAK_CATALOG_RELATIVE_PATH_TEST";
    if std::env::var_os(CHILD).is_none() {
        let tmp = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "filename_only_catalog_updates_and_accepts_without_replacing_working_directory",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .current_dir(tmp.path())
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    use std::os::unix::fs::MetadataExt;
    let directory_id = fs::metadata(".").unwrap().ino();
    fs::create_dir_all("unrelated/empty").unwrap();
    std::os::unix::fs::symlink("not-read", "unrelated-link").unwrap();
    let path = std::path::Path::new("ui.json");
    let mut catalog = LabelCatalog {
        entries: [(
            "button".into(),
            LabelEntry {
                source: "実行".into(),
                context: "button".into(),
                translation: None,
            },
        )]
        .into(),
        ..Default::default()
    };
    save(path, &catalog);
    let fake = Fake(Cell::new(0));
    export::translate_catalog(path, &fake, &settings()).unwrap();
    catalog = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let entry = catalog.entries.get_mut("button").unwrap();
    entry.translation.as_mut().unwrap().value = "Manual translation".into();
    entry.context = "updated button context".into();
    save(path, &catalog);
    export::translate_catalog(path, &fake, &settings()).unwrap();
    export::accept_catalog_candidate(path, "button", &settings()).unwrap();
    export::translate_catalog(path, &fake, &settings()).unwrap();
    assert_eq!(fake.0.get(), 2);
    assert_eq!(fs::metadata(".").unwrap().ino(), directory_id);
    assert!(std::path::Path::new("unrelated/empty").is_dir());
    assert_eq!(
        fs::read_link("unrelated-link").unwrap(),
        std::path::Path::new("not-read")
    );
}

#[test]
fn catalog_updates_respect_the_public_tree_transaction_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let content = tmp.path().join("content");
    fs::create_dir(&content).unwrap();
    let path = content.join("tags.json");
    save(&path, &LabelCatalog::default());
    let lock = tmp.path().join(".content.export-lock");
    fs::write(&lock, "owned by another export").unwrap();
    let before = fs::read(&path).unwrap();
    let fake = Fake(Cell::new(0));
    assert!(export::translate_catalog(&path, &fake, &settings()).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(
        fs::read_to_string(&lock).unwrap(),
        "owned by another export"
    );
    assert_eq!(fake.0.get(), 0);
}

#[test]
fn reviewed_catalog_candidates_survive_retries_and_reject_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("ui.json");
    let fake = Fake(Cell::new(0));
    let mut catalog = LabelCatalog::default();
    catalog.entries.insert(
        "label".into(),
        LabelEntry {
            source: "表示".into(),
            context: "initial context".into(),
            translation: None,
        },
    );
    save(&path, &catalog);
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    catalog
        .entries
        .get_mut("label")
        .unwrap()
        .translation
        .as_mut()
        .unwrap()
        .value = "Manual".into();
    catalog.entries.get_mut("label").unwrap().context = "new context".into();
    save(&path, &catalog);
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    let candidate = fs::read_dir(tmp.path().join(".export-candidates/catalog"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut reviewed: serde_json::Value =
        serde_json::from_slice(&fs::read(&candidate).unwrap()).unwrap();
    reviewed["translation"]["value"] = "Reviewed candidate".into();
    fs::write(&candidate, serde_json::to_vec_pretty(&reviewed).unwrap()).unwrap();
    let before = fs::read(&candidate).unwrap();
    let retry = export::translate_catalog(&path, &fake, &settings()).unwrap();
    assert_eq!(fs::read(&candidate).unwrap(), before);
    assert_eq!(retry.generated, 0);
    assert_eq!(fake.0.get(), 2);
    catalog.entries.get_mut("label").unwrap().context = "third context".into();
    catalog.entries.insert(
        "first".into(),
        LabelEntry {
            source: "先行項目".into(),
            context: "must not translate before candidate checks".into(),
            translation: None,
        },
    );
    save(&path, &catalog);
    let catalog_before = fs::read(&path).unwrap();
    let error = export::translate_catalog(&path, &fake, &settings()).unwrap_err();
    assert!(
        error.to_string().contains("manually edited candidate"),
        "{error}"
    );
    assert_eq!(fs::read(&candidate).unwrap(), before);
    assert_eq!(fs::read(&path).unwrap(), catalog_before);
    assert_eq!(fake.0.get(), 2);
    for reset in [false, true] {
        let mut changed = catalog.clone();
        let translation = &mut changed.entries.get_mut("label").unwrap().translation;
        if reset {
            translation.as_mut().unwrap().value = "EN 表示".into();
        } else {
            *translation = None;
        }
        save(&path, &changed);
        let before_generation = fs::read(&path).unwrap();
        let error = export::translate_catalog(&path, &fake, &settings()).unwrap_err();
        assert!(
            error.to_string().contains("manually edited candidate"),
            "{error}"
        );
        assert_eq!(fs::read(&candidate).unwrap(), before);
        assert_eq!(fs::read(&path).unwrap(), before_generation);
        assert_eq!(fake.0.get(), 2);
    }
    fs::remove_file(&candidate).unwrap();
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    assert_eq!(fake.0.get(), 4);
}

#[test]
fn catalog_directory_alias_uses_the_real_public_tree_lock_for_update_and_acceptance() {
    let tmp = tempfile::tempdir().unwrap();
    let content = tmp.path().join("content");
    let alias = tmp.path().join("content-link");
    fs::create_dir(&content).unwrap();
    std::os::unix::fs::symlink(&content, &alias).unwrap();
    let path = content.join("tags.json");
    let linked_path = alias.join("tags.json");
    let mut catalog = LabelCatalog::default();
    catalog.entries.insert(
        "tag".into(),
        LabelEntry {
            source: "タグ".into(),
            context: "tag label".into(),
            translation: None,
        },
    );
    save(&path, &catalog);
    let fake = Fake(Cell::new(0));
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    catalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let entry = catalog.entries.get_mut("tag").unwrap();
    entry.translation.as_mut().unwrap().value = "Manual tag".into();
    entry.context = "new tag context".into();
    save(&path, &catalog);
    export::translate_catalog(&path, &fake, &settings()).unwrap();
    let lock = tmp.path().join(".content.export-lock");
    fs::write(&lock, "another export").unwrap();
    let before = fs::read(&path).unwrap();
    assert!(export::translate_catalog(&linked_path, &fake, &settings()).is_err());
    assert!(export::accept_catalog_candidate(&linked_path, "tag", &settings()).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read_to_string(&lock).unwrap(), "another export");
    assert!(!tmp.path().join(".content-link.export-lock").exists());
    fs::remove_file(lock).unwrap();
    export::accept_catalog_candidate(&linked_path, "tag", &settings()).unwrap();
    let accepted: LabelCatalog = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        accepted.entries["tag"].translation.as_ref().unwrap().value,
        "EN タグ"
    );
    assert_eq!(fake.0.get(), 2);
}
