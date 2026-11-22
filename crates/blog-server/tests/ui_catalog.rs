#[test]
fn shipped_ui_catalog_has_both_languages_and_valid_interpolation() {
    let catalog: blog_domain::LabelCatalog =
        serde_json::from_str(include_str!("../locales/ui.json")).unwrap();
    catalog.validate().unwrap();
    for (key, entry) in &catalog.entries {
        for locale in blog_domain::Locale::translations() {
            assert!(
                entry.translations.get(&locale).is_some_and(|t| !t.stale),
                "missing or stale {locale} key: {key}"
            );
        }
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
