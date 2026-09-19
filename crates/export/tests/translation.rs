use export::{Texts, TranslationRequest, TranslationSettings, Translator};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

struct Fake {
    calls: Cell<usize>,
    fail_after: usize,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: Cell::new(0),
            fail_after: usize::MAX,
        }
    }
}
impl Translator for Fake {
    fn translate(&self, request: &TranslationRequest) -> anyhow::Result<Texts> {
        let n = self.calls.get();
        self.calls.set(n + 1);
        if n >= self.fail_after {
            anyhow::bail!("simulated quota limit");
        }
        assert!(
            !request.texts.values().any(|v| v.contains("CODE_SECRET")
                || v.contains("HTML_SECRET")
                || v.contains("x^2"))
        );
        Ok(request
            .texts
            .iter()
            .map(|(k, v)| (k.clone(), format!("English {v}")))
            .collect())
    }
}

fn settings() -> TranslationSettings {
    TranslationSettings {
        model: "fake".into(),
        instruction: "test".into(),
        glossary: Texts::new(),
    }
}
fn write_note(source: &Path, body: &str) {
    fs::create_dir_all(source.join("tech")).unwrap();
    fs::write(source.join("tech/a.md"), format!("---\ntitle: 記事\ncategory: tech\nis_completed: true\ncreated: '2025-01-01T00:00:00+09:00'\nupdated: '2025-01-01T00:00:00+09:00'\n---\n{body}\n")).unwrap();
}
fn english(output: &Path) -> PathBuf {
    fs::read_dir(output.join("en"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
}

#[test]
fn translates_only_prose_and_reuses_unchanged_manual_edits() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let fake = Fake::new();
    write_note(
        source.path(),
        "# 見出し\n\n本文 **強調** `CODE_SECRET` $x^2$\n\n```rust\nCODE_SECRET\n```",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    let path = english(output.path());
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("`CODE_SECRET` $x^2$"));
    assert!(text.contains("```rust\nCODE_SECRET\n```"));
    let manual = text.replace("English 記事", "Manually edited title");
    fs::write(&path, &manual).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    assert_eq!(fake.calls.get(), 1);
    assert_eq!(fs::read_to_string(path).unwrap(), manual);
}

#[test]
fn changed_source_protects_manual_edits_and_candidate_requires_current_source() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let fake = Fake::new();
    write_note(source.path(), "本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    let path = english(output.path());
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("English 記事", "Manual title"),
    )
    .unwrap();
    write_note(source.path(), "更新した本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    let report = export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    assert_eq!(report.protected.len(), 1);
    assert_eq!(fake.calls.get(), 1);
    assert!(fs::read_to_string(&path).unwrap().contains("Manual title"));
    assert!(fs::read_to_string(&path).unwrap().contains("stale: true"));
    export::translate_public(output.path(), &fake, &settings(), true).unwrap();
    let id = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
    export::accept_translation(output.path(), &id, &settings()).unwrap();
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("English 更新した本文")
    );
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    assert_eq!(fake.calls.get(), 2);
}

#[test]
fn code_only_update_reassembles_from_cached_translation_without_ai() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let fake = Fake::new();
    write_note(source.path(), "本文 `old-code`");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    write_note(source.path(), "本文 `new-code`");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    assert_eq!(fake.calls.get(), 1);
    assert!(
        fs::read_to_string(english(output.path()))
            .unwrap()
            .contains("`new-code`")
    );
}

#[test]
fn reviewed_candidates_survive_retries_and_block_overwrites_after_source_changes() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let fake = Fake::new();
    write_note(source.path(), "本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    let path = english(output.path());
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("English 記事", "Manual title"),
    )
    .unwrap();
    write_note(source.path(), "新しい本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), true).unwrap();
    let candidate = output
        .path()
        .join(".export-candidates")
        .join(path.file_name().unwrap());
    let reviewed = fs::read_to_string(&candidate)
        .unwrap()
        .replace("English 記事", "Reviewed title");
    fs::write(&candidate, &reviewed).unwrap();
    let retry = export::translate_public(output.path(), &fake, &settings(), true).unwrap();
    assert_eq!(fs::read_to_string(&candidate).unwrap(), reviewed);
    assert_eq!(retry.generated, 0);
    assert_eq!(fake.calls.get(), 2);
    write_note(source.path(), "さらに新しい本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    let before = fs::read(&path).unwrap();
    let error = export::translate_public(output.path(), &fake, &settings(), true).unwrap_err();
    assert!(
        error.to_string().contains("manually edited candidate"),
        "{error}"
    );
    assert_eq!(fs::read_to_string(&candidate).unwrap(), reviewed);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fake.calls.get(), 2);
    fs::remove_file(&candidate).unwrap();
    export::translate_public(output.path(), &fake, &settings(), true).unwrap();
    assert!(
        fs::read_to_string(&candidate)
            .unwrap()
            .contains("English さらに新しい本文")
    );
}

#[test]
fn accepting_a_candidate_refreshes_management_metadata_and_keeps_reviewed_prose() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let fake = Fake::new();
    write_note(source.path(), "本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    let path = english(output.path());
    fs::write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace("English 記事", "Manual title"),
    )
    .unwrap();
    write_note(source.path(), "更新した本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), true).unwrap();
    let candidate_path = output
        .path()
        .join(".export-candidates")
        .join(path.file_name().unwrap());
    fs::write(
        &candidate_path,
        fs::read_to_string(&candidate_path)
            .unwrap()
            .replace("English 記事", "Reviewed candidate title"),
    )
    .unwrap();
    let moved = fs::read_to_string(source.path().join("tech/a.md"))
        .unwrap()
        .replace(
            "category: tech",
            "category: daily\ntags: [NewTag]\npriority: 42",
        )
        .replace(
            "updated: '2025-01-01T00:00:00+09:00'",
            "updated: '2025-02-01T00:00:00+09:00'",
        );
    fs::remove_file(source.path().join("tech/a.md")).unwrap();
    fs::create_dir_all(source.path().join("daily/new")).unwrap();
    fs::write(source.path().join("daily/new/a.md"), moved).unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let id = path.file_stem().unwrap().to_str().unwrap().parse().unwrap();
    export::accept_translation(output.path(), &id, &settings()).unwrap();
    fn meta(path: &Path) -> domain::PublicContentMeta {
        let text = fs::read_to_string(path).unwrap();
        serde_yaml::from_str(
            text.strip_prefix("---\n")
                .unwrap()
                .split_once("\n---\n")
                .unwrap()
                .0,
        )
        .unwrap()
    }
    let actual = meta(&path);
    let mut expected = meta(&output.path().join("ja").join(path.file_name().unwrap()));
    expected.locale = domain::Locale::En;
    expected.title = "Reviewed candidate title".into();
    expected.translation = actual.translation.clone();
    assert_eq!(actual, expected);
    assert_eq!(actual.path(), format!("/en/daily/{id}"));
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("English 更新した本文")
    );
    assert!(!candidate_path.exists());
    let report = export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    // The article is reused; translating the newly added tag is independent.
    assert_eq!(report.reused, 1);
    assert_eq!(meta(&path).title, "Reviewed candidate title");
}

#[test]
fn translated_plain_text_cannot_introduce_markdown_structure() {
    struct Plain<'a>(&'a str);
    impl Translator for Plain<'_> {
        fn translate(&self, request: &TranslationRequest) -> anyhow::Result<Texts> {
            Ok(request
                .texts
                .keys()
                .map(|key| {
                    (
                        key.clone(),
                        if key == "title" { "Title" } else { self.0 }.into(),
                    )
                })
                .collect())
        }
    }
    for prose in [
        "1. First",
        "- item",
        "+ item",
        "1) item",
        "~~removed~~",
        "===",
        "---",
    ] {
        let source = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        write_note(source.path(), "本文");
        export::export_japanese(source.path(), output.path()).unwrap();
        export::translate_public(output.path(), &Plain(prose), &settings(), false).unwrap();
        let text = fs::read_to_string(english(output.path())).unwrap();
        let body = text
            .strip_prefix("---\n")
            .unwrap()
            .split_once("\n---\n")
            .unwrap()
            .1;
        let mut html = String::new();
        pulldown_cmark::html::push_html(
            &mut html,
            pulldown_cmark::Parser::new_ext(body, pulldown_cmark::Options::ENABLE_STRIKETHROUGH),
        );
        assert_eq!(html, format!("<p>{prose}</p>\n"), "{prose}");
    }
}

#[test]
fn inline_html_text_is_preserved_while_surrounding_prose_is_translated() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let fake = Fake::new();
    write_note(
        source.path(),
        "前文 <span data-label=\"raw > value\">HTML_SECRET <b>HTML_SECRET</b></span> 後文<br> 続き <!-- <span> --> 最後\n\n<kbd>HTML_SECRET</kbd>",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    let translated = fs::read_to_string(english(output.path())).unwrap();
    assert!(
        translated
            .contains("<span data-label=\"raw > value\">HTML_SECRET <b>HTML_SECRET</b></span>")
    );
    assert!(translated.contains("<kbd>HTML_SECRET</kbd>"));
    assert!(translated.contains("English 前文"));
    assert!(translated.contains(" English 後文"));
    assert!(translated.contains(" English 続き"));
    assert!(translated.contains(" English 最後"));
}

#[test]
fn autolinks_are_untouched_and_self_closing_html_does_not_hide_prose() {
    struct Check;
    impl Translator for Check {
        fn translate(&self, request: &TranslationRequest) -> anyhow::Result<Texts> {
            assert!(
                !request
                    .texts
                    .values()
                    .any(|text| text.contains("https://") || text.contains("me@example.com"))
            );
            Ok(request
                .texts
                .iter()
                .map(|(key, value)| (key.clone(), format!("EN {}", value.trim())))
                .collect())
        }
    }
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    write_note(
        source.path(),
        "<https://example.com/a> <me@example.com> 文 <span /> 続き <x/> 最後",
    );
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &Check, &settings(), false).unwrap();
    let text = fs::read_to_string(english(output.path())).unwrap();
    assert!(text.contains("<https://example.com/a> <me@example.com>"));
    assert!(text.contains("EN 文"));
    assert!(text.contains("EN 続き"));
    assert!(text.contains("EN 最後"));
}

#[test]
fn trimmed_translations_keep_the_original_fragment_boundary_whitespace() {
    struct Trim;
    impl Translator for Trim {
        fn translate(&self, request: &TranslationRequest) -> anyhow::Result<Texts> {
            Ok(request
                .texts
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        match value.trim() {
                            "前" => "Before",
                            "強調" => "emphasis",
                            "後" => "After",
                            _ => "Title",
                        }
                        .into(),
                    )
                })
                .collect())
        }
    }
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    write_note(source.path(), "前 **強調** 後");
    export::export_japanese(source.path(), output.path()).unwrap();
    export::translate_public(output.path(), &Trim, &settings(), false).unwrap();
    assert!(
        fs::read_to_string(english(output.path()))
            .unwrap()
            .contains("Before **emphasis** After")
    );
}

#[test]
fn invalid_article_response_is_not_cached_and_retry_can_succeed() {
    struct Retry(Cell<usize>);
    impl Translator for Retry {
        fn translate(&self, request: &TranslationRequest) -> anyhow::Result<Texts> {
            self.0.set(self.0.get() + 1);
            Ok(request
                .texts
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        if self.0.get() == 1 && key.starts_with("text_") {
                            "bad\nfragment".into()
                        } else {
                            format!("EN {value}")
                        },
                    )
                })
                .collect())
        }
    }
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    write_note(source.path(), "本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    let retry = Retry(Cell::new(0));
    assert!(export::translate_public(output.path(), &retry, &settings(), false).is_err());
    export::translate_public(output.path(), &retry, &settings(), false).unwrap();
    assert_eq!(retry.0.get(), 2);
    assert!(
        fs::read_to_string(english(output.path()))
            .unwrap()
            .contains("EN 本文")
    );
}

#[test]
fn missing_provenance_is_protected() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let fake = Fake::new();
    write_note(source.path(), "本文");
    export::export_japanese(source.path(), output.path()).unwrap();
    let path = fs::read_dir(output.path().join("ja"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::create_dir_all(output.path().join("en")).unwrap();
    fs::write(
        output.path().join("en").join(path.file_name().unwrap()),
        fs::read_to_string(path)
            .unwrap()
            .replace("locale: ja", "locale: en"),
    )
    .unwrap();
    let report = export::translate_public(output.path(), &fake, &settings(), false).unwrap();
    assert_eq!(report.protected.len(), 1);
    assert_eq!(fake.calls.get(), 0);
}

#[test]
fn failure_rolls_back_outputs_and_retry_reuses_completed_units() {
    let source = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    write_note(source.path(), "本文");
    fs::write(
        source.path().join("tech/b.md"),
        fs::read_to_string(source.path().join("tech/a.md"))
            .unwrap()
            .replace("title: 記事", "title: 別の記事"),
    )
    .unwrap();
    export::export_japanese(source.path(), output.path()).unwrap();
    let failing = Fake {
        calls: Cell::new(0),
        fail_after: 1,
    };
    assert!(export::translate_public(output.path(), &failing, &settings(), false).is_err());
    assert!(!output.path().join("en").exists());
    let retry = Fake::new();
    export::translate_public(output.path(), &retry, &settings(), false).unwrap();
    assert_eq!(retry.calls.get(), 1);
    assert_eq!(fs::read_dir(output.path().join("en")).unwrap().count(), 2);
}

#[test]
fn fake_codex_checks_restricted_arguments_and_structured_output() {
    use std::os::unix::fs::PermissionsExt;
    let temp = TempDir::new().unwrap();
    let program = temp.path().join("codex");
    fs::write(
        &program,
        r#"#!/bin/sh
set -eu
printf '%s\n' "$@" > "$(dirname "$0")/args"
cat > "$(dirname "$0")/prompt"
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--output-last-message' ]; then
    shift
    printf '%s' '{"title":"Translated"}' > "$1"
  fi
  shift
done
"#,
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    let translator = export::CodexTranslator {
        executable: program,
        timeout: std::time::Duration::from_secs(5),
    };
    let result = translator
        .translate(&TranslationRequest {
            texts: Texts::from([("title".into(), "記事".into())]),
            context: "title".into(),
            model: "fake".into(),
            instruction: "Use British English for spelling.".into(),
            glossary: Texts::new(),
        })
        .unwrap();
    assert_eq!(result["title"], "Translated");
    let prompt = fs::read_to_string(temp.path().join("prompt")).unwrap();
    let (instructions, source_json) = prompt.split_once("\nUNTRUSTED_SOURCE_JSON\n").unwrap();
    assert!(instructions.contains("Use British English for spelling."));
    let source: serde_json::Value = serde_json::from_str(source_json).unwrap();
    assert_eq!(source["texts"]["title"], "記事");
    assert!(source.get("instruction").is_none());
    let args = fs::read_to_string(temp.path().join("args")).unwrap();
    for required in [
        "--ignore-user-config",
        "--ignore-rules",
        "--ephemeral",
        "forced_login_method=\"chatgpt\"",
        "default_permissions=\"translation\"",
        "\":workspace_roots\"=\"read\"",
        "features.shell_tool=false",
        "features.apps=false",
        "features.plugins=false",
    ] {
        assert!(args.contains(required), "missing {required}");
    }
}
