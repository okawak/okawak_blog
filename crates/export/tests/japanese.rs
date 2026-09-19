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

#[test]
fn markdown_links_resolve_the_sibling_before_a_root_note_with_the_same_name() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/sub/article.md",
        "Article",
        true,
        "[Target](target.md)",
    );
    note(
        source.path(),
        "tech/sub/target.md",
        "Sibling",
        true,
        "Sibling body",
    );
    note(source.path(), "target.md", "Root", true, "Root body");
    let root_note = source.path().join("target.md");
    let raw = fs::read_to_string(&root_note)
        .unwrap()
        .replace("category: tech", "kind: home");
    fs::write(root_note, raw).unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let exported = outputs(output.path());
    let sibling = exported
        .iter()
        .find(|text| text.contains("title: Sibling\n"))
        .unwrap();
    let id = sibling
        .lines()
        .find_map(|line| line.strip_prefix("id: "))
        .unwrap();
    let article = exported
        .iter()
        .find(|text| text.contains("title: Article\n"))
        .unwrap();
    assert!(article.contains(&format!("[Target](content:{id})")));

    // A private sibling must not silently redirect to a different public note.
    note(
        source.path(),
        "tech/sub/target.md",
        "Sibling",
        false,
        "PRIVATE",
    );
    assert!(export::export_japanese(source.path(), output.path()).is_err());
    assert_eq!(outputs(output.path()), exported);
}

#[test]
fn markdown_images_use_the_relative_file_despite_duplicate_basenames() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/sub/article.md",
        "Article",
        true,
        "![x](image.png)",
    );
    fs::write(source.path().join("tech/sub/image.png"), b"sibling image").unwrap();
    fs::write(source.path().join("image.png"), b"other image").unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let assets: Vec<_> = fs::read_dir(output.path().join("assets"))
        .unwrap()
        .collect();
    assert_eq!(assets.len(), 1);
    assert_eq!(
        fs::read(assets[0].as_ref().unwrap().path()).unwrap(),
        b"sibling image"
    );

    note(
        source.path(),
        "tech/sub/article.md",
        "Article",
        true,
        "![[image.png]]",
    );
    assert!(export::export_japanese(source.path(), output.path()).is_err());
}

#[test]
fn markdown_url_paths_and_heading_fragments_are_decoded_before_resolution() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "[Target](My%20Note.md#%E8%A6%8B%E5%87%BA%E3%81%97)\n![Photo](my%20image.png)",
    );
    note(source.path(), "tech/My Note.md", "Target", true, "# 見出し");
    fs::write(source.path().join("tech/my image.png"), b"public photo").unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let before = outputs(output.path());
    let article = before
        .iter()
        .find(|s| s.contains("title: Article\n"))
        .unwrap();
    assert!(article.contains("#section-"));
    assert!(article.contains("![Photo](/content-assets/"));
    assert!(!article.contains("%20"));
    for path in [
        "%2Fprivate%2Fsecret.md",
        "..%2F..%2Fsecret.md",
        "target%00.md",
        "%FF.md",
    ] {
        note(
            source.path(),
            "tech/article.md",
            "Article",
            true,
            &format!("[Bad]({path})"),
        );
        assert!(
            export::export_japanese(source.path(), output.path()).is_err(),
            "accepted {path}"
        );
        assert_eq!(outputs(output.path()), before);
    }
}

#[test]
fn reference_definitions_do_not_leave_vault_paths_or_unused_private_titles() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "[Target][ref]\n![Photo][photo]\n[External][web]\n\n[ref]: target.md \"PRIVATE TITLE\"\n[ref]: unused.md \"PRIVATE DUPLICATE\"\n[photo]: image.png \"PRIVATE IMAGE\"\n[web]: https://example.com \"Public tooltip\"\n\n```md\n[example]: literal.md\n```",
    );
    note(source.path(), "tech/target.md", "Target", true, "Body");
    fs::write(source.path().join("tech/image.png"), b"public photo").unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let exported = outputs(output.path());
    let article = exported
        .iter()
        .find(|s| s.contains("title: Article\n"))
        .unwrap();
    assert!(article.contains("[Target](content:"));
    assert!(article.contains("![Photo](/content-assets/"));
    assert!(article.contains("https://example.com"));
    assert!(article.contains("Public tooltip"));
    assert!(!article.contains("PRIVATE"));
    assert!(!article.contains("target.md"));
    assert!(!article.contains("image.png"));
    assert!(article.contains("```md\n[example]: literal.md\n```"));
}

#[test]
fn email_and_web_autolinks_remain_external_references() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let body = "<me@example.com> <https://example.com/a>";
    note(source.path(), "tech/article.md", "Article", true, body);
    export::export_japanese(source.path(), output.path()).unwrap();
    assert!(outputs(output.path())[0].contains(body));
}

#[test]
fn references_resolve_markdown_extensions_case_insensitively() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "[Target](target.MD) [[target.MD|Wiki]]",
    );
    note(source.path(), "tech/target.MD", "Target", true, "Body");
    export::export_japanese(source.path(), output.path()).unwrap();
    let article = outputs(output.path())
        .into_iter()
        .find(|s| s.contains("title: Article\n"))
        .unwrap();
    assert!(article.contains("[Target](content:"));
    assert!(article.contains("[Wiki](content:"));
}

#[test]
fn markdown_link_labels_preserve_escaped_nested_and_opaque_brackets() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let labels = [
        r"see \] escaped",
        "see [nested]",
        "see `]` code",
        "see <span title=\"]\">HTML</span>",
    ];
    let body = labels
        .iter()
        .map(|label| format!("[{label}](target.md)"))
        .collect::<Vec<_>>()
        .join("\n");
    note(source.path(), "tech/article.md", "Article", true, &body);
    note(source.path(), "tech/target.md", "Target", true, "Body");
    export::export_japanese(source.path(), output.path()).unwrap();
    let article = outputs(output.path())
        .into_iter()
        .find(|s| s.contains("title: Article\n"))
        .unwrap();
    for label in labels {
        assert!(
            article.contains(&format!("[{label}](content:")),
            "lost label: {label}\n{article}"
        );
    }
}

#[test]
fn wikilink_aliases_keep_existing_escapes_and_literal_brackets() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        r"[[target|see \[literal\]]] and [[target|a [bracket] label]] and [[target|two \\ slashes]] and [[tail]]",
    );
    note(source.path(), "tech/target.md", "Target", true, "Body");
    note(source.path(), "tech/tail.md", r"trailing\", true, "Body");
    export::export_japanese(source.path(), output.path()).unwrap();
    let article = outputs(output.path())
        .into_iter()
        .find(|s| s.contains("title: Article\n"))
        .unwrap();
    let body = article
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---\n")
        .unwrap()
        .1;
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(body));
    for label in [
        "see [literal]",
        "a [bracket] label",
        r"two \ slashes",
        r"trailing\",
    ] {
        assert!(html.contains(&format!(">{label}</a>")), "{html}");
    }
}

#[test]
fn markdown_image_prefers_the_asset_over_a_note_with_the_same_stem() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    note(
        source.path(),
        "tech/article.md",
        "Article",
        true,
        "![photo](image.png)\n![[image.png.md|Note embed]]",
    );
    note(
        source.path(),
        "tech/image.png.md",
        "Image note",
        true,
        "Body",
    );
    fs::write(source.path().join("tech/image.png"), b"public image").unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let article = outputs(output.path())
        .into_iter()
        .find(|s| s.contains("title: Article\n"))
        .unwrap();
    assert!(article.contains("![photo](/content-assets/"), "{article}");
    assert!(article.contains("[Note embed](content:"), "{article}");
    let assets: Vec<_> = fs::read_dir(output.path().join("assets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(assets.len(), 1);
    assert_eq!(fs::read(&assets[0]).unwrap(), b"public image");
}
