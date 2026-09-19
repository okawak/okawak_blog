#[test]
fn shipped_ui_catalog_has_both_languages_and_valid_interpolation() {
    let catalog: domain::LabelCatalog =
        serde_json::from_str(include_str!("../locales/ui.json")).unwrap();
    catalog.validate().unwrap();
    for (key, entry) in &catalog.entries {
        let translation = entry
            .translation
            .as_ref()
            .unwrap_or_else(|| panic!("missing English key: {key}"));
        assert!(!translation.stale, "stale English key: {key}");
    }
    for count in ["zero", "one", "many"] {
        assert!(
            catalog
                .entries
                .contains_key(&format!("count.articles.{count}"))
        );
        assert!(
            catalog
                .entries
                .contains_key(&format!("count.categories.{count}"))
        );
    }
}
