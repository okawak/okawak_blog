use std::{
    fs,
    path::Path,
    process::{Command, ExitStatus},
};

fn validate(root: &Path) -> ExitStatus {
    Command::new(env!("CARGO_BIN_EXE_blog-publish"))
        .arg("--validate-artifacts")
        .arg(root)
        .output()
        .unwrap()
        .status
}

fn minimal_site() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("articles")).unwrap();
    fs::create_dir(root.path().join("metadata")).unwrap();
    fs::write(
        root.path().join("locales.json"),
        r#"{"schema_version":1,"routes":{"/":["ja"]}}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("articles/index.json"),
        r#"{"articles":[]}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("metadata/site.json"),
        r#"{"total_articles":0,"categories":[]}"#,
    )
    .unwrap();
    root
}

#[test]
fn optional_home_may_be_absent_but_must_be_readable_and_valid_when_present() {
    let root = minimal_site();
    assert!(validate(root.path()).success());
    let home = root.path().join("home.json");
    fs::write(
        &home,
        r#"{"title":"Home","html":"<p>Welcome</p>","updated_at":"2026-01-01T00:00:00Z"}"#,
    )
    .unwrap();
    assert!(validate(root.path()).success());
    fs::write(&home, "invalid").unwrap();
    assert!(!validate(root.path()).success());
    fs::remove_file(&home).unwrap();
    fs::create_dir(&home).unwrap();
    assert!(!validate(root.path()).success());
}

#[test]
fn category_and_page_artifacts_are_validated_and_checked_against_their_filenames() {
    for (directory, filename, document) in [
        (
            "categories",
            "tech",
            r#"{"category":"tech","title":"Tech","html":"<p>Tech</p>","updated_at":"2026-01-01T00:00:00Z","articles":[]}"#,
        ),
        (
            "pages",
            "about",
            r#"{"page":"about","title":"About","html":"<p>About</p>","updated_at":"2026-01-01T00:00:00Z"}"#,
        ),
    ] {
        let root = minimal_site();
        let directory = root.path().join(directory);
        fs::create_dir(&directory).unwrap();
        let path = directory.join(format!("{filename}.json"));
        fs::write(&path, document).unwrap();
        fs::write(directory.join("ignored.txt"), "not json").unwrap();
        assert!(validate(root.path()).success());
        fs::rename(&path, directory.join("wrong.json")).unwrap();
        assert!(!validate(root.path()).success());
        fs::rename(directory.join("wrong.json"), &path).unwrap();
        fs::write(&path, "invalid").unwrap();
        assert!(!validate(root.path()).success());
    }
}
