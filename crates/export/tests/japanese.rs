use std::{fs, path::Path};
use tempfile::TempDir;

fn note(root: &Path, path: &str, title: &str, completed: bool, body: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("---\ntitle: {title}\ncategory: tech\nis_completed: {completed}\ncreated: '2025-01-01T00:00:00+09:00'\nupdated: '2025-01-01T00:00:00+09:00'\nprivate_notes: SECRET\n---\n{body}\n")).unwrap();
}

fn outputs(root: &Path) -> Vec<String> {
    let mut paths: Vec<_> = fs::read_dir(root.join("ja"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| fs::read_to_string(p).unwrap())
        .collect()
}

#[test]
fn exports_only_completed_notes_allowlists_metadata_and_preserves_legacy_slug() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "Public body",
    );
    note(
        source.path(),
        "tech/draft.md",
        "Draft",
        false,
        "PRIVATE BODY",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    let actual = outputs(output.path());
    assert_eq!(actual.len(), 1);
    assert!(actual[0].contains("Public body"));
    assert!(!actual[0].contains("SECRET"));
    assert!(!actual[0].contains("PRIVATE"));
    assert!(!actual[0].contains("tech/article.md"));
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(b"Article/tech/article.md/2025-01-01T00:00:00+09:00")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert!(
        output
            .path()
            .join(format!("ja/{}.md", &digest[..12]))
            .exists()
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    assert_eq!(outputs(output.path()), actual);
}

#[test]
fn failed_reference_validation_does_not_change_existing_output() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "Public body",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    let before = outputs(output.path());
    note(
        source.path(),
        "tech/draft.md",
        "Draft",
        false,
        "PRIVATE BODY",
    );
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "[[draft]]",
    );
    assert!(export::export_japanese(source.path(), output.path()).is_err());
    assert_eq!(outputs(output.path()), before);
}

#[test]
fn rename_and_title_update_keep_id_and_unpublish_removes_managed_output() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "Public body",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    let old_name = fs::read_dir(output.path().join("ja"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name();
    fs::remove_file(source.path().join("tech/article.md")).unwrap();
    note(
        source.path(),
        "tech/renamed.md",
        "Renamed",
        true,
        "Public body",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    assert!(output.path().join("ja").join(old_name).exists());
    fs::write(output.path().join("README.md"), "Keep this").unwrap();
    note(
        source.path(),
        "tech/renamed.md",
        "Renamed",
        false,
        "Public body",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    assert!(outputs(output.path()).is_empty());
    assert_eq!(
        fs::read_to_string(output.path().join("README.md")).unwrap(),
        "Keep this"
    );
}

#[test]
fn resolves_public_wikilinks_and_copies_only_referenced_images() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "[[target|See target]]\n![[image.png]]\n`[[private]]`",
    );
    note(source.path(), "tech/target.md", "Target", true, "# Target");
    fs::write(source.path().join("image.png"), b"public image").unwrap();
    fs::write(source.path().join("private.png"), b"private image").unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let actual = outputs(output.path()).join("\n");
    assert!(actual.contains("[See target](content:"));
    assert!(actual.contains("/content-assets/"));
    assert!(actual.contains("`[[private]]`"));
    assert_eq!(
        fs::read_dir(output.path().join("assets")).unwrap().count(),
        1
    );
}

#[test]
fn symlink_source_is_rejected() {
    let source = TempDir::new().unwrap();
    let private = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(private.path(), "tech/secret.md", "Secret", true, "SECRET");
    std::os::unix::fs::symlink(
        private.path().join("tech/secret.md"),
        source.path().join("leak.md"),
    )
    .unwrap();
    assert!(export::export_japanese(source.path(), output.path()).is_err());
}

#[test]
fn preserves_hidden_user_files_and_deleted_translations_in_archive() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "Public body",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    fs::write(output.path().join(".keep"), "user data").unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        false,
        "Public body",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    assert_eq!(
        fs::read_to_string(output.path().join(".keep")).unwrap(),
        "user data"
    );
    assert_eq!(
        fs::read_dir(output.path().join(".export-archive"))
            .unwrap()
            .count(),
        1
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    assert_eq!(
        fs::read_dir(output.path().join(".export-archive"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn failed_export_preserves_unrelated_files_and_rejects_output_inside_source() {
    let source = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "Public body",
    );
    assert!(export::export_japanese(source.path(), &source.path().join("output")).is_err());
}

#[test]
fn ambiguous_rename_requires_explicit_id() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(source.path(), "tech/a.md", "A", true, "A");
    note(source.path(), "tech/b.md", "B", true, "B");
    export::export_japanese(source.path(), output.path()).unwrap();
    fs::remove_file(source.path().join("tech/a.md")).unwrap();
    fs::remove_file(source.path().join("tech/b.md")).unwrap();
    note(source.path(), "tech/c.md", "C", true, "C");
    assert!(
        export::export_japanese(source.path(), output.path())
            .unwrap_err()
            .to_string()
            .contains("ambiguous")
    );
    assert_eq!(outputs(output.path()).len(), 2);
}

#[test]
fn heading_references_are_stable_and_missing_anchors_stop_export() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/a.md",
        "A",
        true,
        "# 見出し\n\n[[#見出し|見出しへ]]",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    let result = outputs(output.path()).join("\n");
    assert!(result.contains("<a id=\"section-"));
    assert!(result.contains("#section-"));
    note(
        source.path(),
        "tech/a.md",
        "A",
        true,
        "# 見出し\n\n[[#存在しない|リンク]]",
    );
    assert!(export::export_japanese(source.path(), output.path()).is_err());
    assert_eq!(outputs(output.path()).join("\n"), result);
}

#[test]
fn public_bookmark_html_is_allowed_but_local_html_images_are_rejected() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/a.md",
        "A",
        true,
        "<div class=\"bookmark\"><a href=\"https://example.com\">Public</a></div>",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    note(
        source.path(),
        "tech/a.md",
        "A",
        true,
        "<IMG SRC = 'file:///private/secret.png'>",
    );
    assert!(export::export_japanese(source.path(), output.path()).is_err());
}
