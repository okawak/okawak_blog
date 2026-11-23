use indoc::indoc;
use rstest::{fixture, rstest};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn validate(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_blog-publish"))
        .arg("--validate-artifacts")
        .arg(root)
        .output()
        .unwrap()
}

#[fixture]
fn minimal_site() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("ja/_data")).unwrap();
    fs::write(
        root.path().join("locales.json"),
        r#"{"schema_version":1,"routes":{"/":["ja"]}}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("ja/_data/articles.json"),
        r#"{"articles":[]}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("ja/_data/site.json"),
        r#"{"total_articles":0,"categories":[]}"#,
    )
    .unwrap();
    root
}

#[rstest]
fn optional_home_may_be_absent_but_must_be_readable_and_valid_when_present(
    minimal_site: tempfile::TempDir,
) {
    let root = minimal_site;
    assert!(validate(root.path()).status.success());
    let home = root.path().join("ja/index.json");
    fs::write(
        &home,
        indoc! {r#"
            {
                "title": "Home",
                "html": "<p>Welcome</p>",
                "updated_at": "2026-01-01T00:00:00Z"
            }
        "#},
    )
    .unwrap();
    assert!(validate(root.path()).status.success());
    fs::write(&home, "invalid").unwrap();
    assert!(!validate(root.path()).status.success());
    fs::remove_file(&home).unwrap();
    fs::create_dir(&home).unwrap();
    let result = validate(root.path());
    assert!(!result.status.success());
    let read_error = fs::read(&home).unwrap_err().to_string();
    assert!(String::from_utf8_lossy(&result.stderr).contains(&read_error));
}

#[rstest]
#[case::category("tech", indoc! {r#"
    {
        "category": "tech",
        "title": "Tech",
        "html": "<p>Tech</p>",
        "updated_at": "2026-01-01T00:00:00Z",
        "articles": []
    }
"#})]
#[case::static_page("about", indoc! {r#"
    {
        "page": "about",
        "title": "About",
        "html": "<p>About</p>",
        "updated_at": "2026-01-01T00:00:00Z"
    }
"#})]
fn category_and_page_artifacts_are_validated_and_checked_against_their_filenames(
    minimal_site: tempfile::TempDir,
    #[case] filename: &str,
    #[case] document: &str,
) {
    let root = minimal_site;
    let directory = root.path().join("ja");
    let path = directory.join(format!("{filename}.json"));
    fs::write(&path, document).unwrap();
    fs::write(directory.join("ignored.txt"), "not json").unwrap();
    assert!(validate(root.path()).status.success());
    fs::rename(&path, directory.join("wrong.json")).unwrap();
    assert!(!validate(root.path()).status.success());
    fs::rename(directory.join("wrong.json"), &path).unwrap();
    fs::write(&path, "invalid").unwrap();
    assert!(!validate(root.path()).status.success());
}
