#![cfg(unix)]

use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
};
use tempfile::TempDir;

const ARTICLE: &str = "000000000001";

struct Project(TempDir);

impl Project {
    fn new() -> Self {
        let project = Self(TempDir::new().unwrap());
        project.write_note("本文");
        project.write_json(
            "translation.json",
            json!({
                "model": "fake", "instruction": "Translate prose", "glossary": {}
            }),
        );
        project.write_json(
            "crates/server/locales/ui.json",
            json!({
                "schema_version": 1,
                "entries": {"greeting": {"source": "こんにちは", "context": "greeting"}}
            }),
        );
        let executable = project.path("bin/codex");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(
            &executable,
            r#"#!/bin/sh
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then
    shift
    export FAKE_CODEX_OUTPUT="$1"
  fi
  shift
done
exec "$FAKE_CODEX_TEST" --exact fake_codex_process --nocapture
"#,
        )
        .unwrap();
        fs::set_permissions(executable, fs::Permissions::from_mode(0o700)).unwrap();
        project
    }

    fn path(&self, path: &str) -> PathBuf {
        self.0.path().join(path)
    }

    fn write_note(&self, body: &str) {
        let path = self.path("obsidian/Publish/tech/article.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("---\npublish_id: '{ARTICLE}'\ntitle: 記事\ncategory: tech\ntags: [Rust]\nis_completed: true\ncreated: '2025-01-01T00:00:00+09:00'\nupdated: '2025-01-01T00:00:00+09:00'\n---\n{body}\n")).unwrap();
    }

    fn write_json(&self, path: &str, value: Value) {
        let path = self.path(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }

    fn read_json(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.path(path)).unwrap()).unwrap()
    }

    fn invoke(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_export"))
            .current_dir(self.0.path())
            .args(args)
            .env("PATH", self.path("bin"))
            .env("RUST_LOG", "info")
            .env("FAKE_CODEX_TEST", std::env::current_exe().unwrap())
            .env("FAKE_CODEX_LOG", self.path("calls"))
            .output()
            .unwrap()
    }

    fn run(&self, args: &[&str]) -> Output {
        let result = self.invoke(args);
        assert!(
            result.status.success(),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&result.stderr)
        );
        result
    }

    fn calls(&self) -> usize {
        fs::read_to_string(self.path("calls"))
            .unwrap_or_default()
            .lines()
            .count()
    }

    fn prepare_review_candidates(&self) -> Output {
        self.run(&[]);
        let english = self.path(&format!("content/en/{ARTICLE}.md"));
        let translated = fs::read_to_string(&english).unwrap();
        fs::write(
            &english,
            translated.replace("English 本文", "Hand edited prose"),
        )
        .unwrap();
        self.write_note("更新した本文");

        let mut tags = self.read_json("content/tags.json");
        tags["entries"]["Rust"]["translation"]["value"] = json!("Hand edited tag");
        tags["entries"]["Rust"]["translation"]
            .as_object_mut()
            .unwrap()
            .remove("provenance");
        self.write_json("content/tags.json", tags);

        let mut ui = self.read_json("crates/server/locales/ui.json");
        ui["entries"]["greeting"]["translation"]["value"] = json!("Hand edited greeting");
        ui["entries"]["greeting"]["source"] = json!("新しい挨拶");
        self.write_json("crates/server/locales/ui.json", ui);

        self.run(&[])
    }
}

// Runs only inside the fake Codex child process, without network or real AI.
#[test]
fn fake_codex_process() {
    let Some(output) = std::env::var_os("FAKE_CODEX_OUTPUT") else {
        return;
    };
    let mut prompt = String::new();
    std::io::stdin().read_to_string(&mut prompt).unwrap();
    let source: Value =
        serde_json::from_str(prompt.split_once("\nUNTRUSTED_SOURCE_JSON\n").unwrap().1).unwrap();
    let result: serde_json::Map<String, Value> = source["texts"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                json!(format!("English {}", value.as_str().unwrap())),
            )
        })
        .collect();
    fs::write(output, serde_json::to_vec(&result).unwrap()).unwrap();
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::var_os("FAKE_CODEX_LOG").unwrap())
        .unwrap();
    writeln!(log, "translated").unwrap();
}

#[test]
fn default_export_translates_every_scope_and_reuses_unchanged_results() {
    let project = Project::new();
    let first_run = project.run(&[]);
    let logs = String::from_utf8(first_run.stderr).unwrap();
    for message in [
        "content export started",
        "translation item started",
        "ai_requests",
        "current",
        "action",
        "waiting for AI translation",
        "AI translation completed",
        "UI translation started",
        "export completed",
    ] {
        assert!(logs.contains(message), "missing progress log: {message}");
    }
    assert!(!logs.contains("本文"), "source prose must not be logged");
    assert!(
        !logs.contains("こんにちは"),
        "UI source text must not be logged"
    );
    let english = project.path(&format!("content/en/{ARTICLE}.md"));
    assert!(
        fs::read_to_string(&english)
            .unwrap()
            .contains("English 本文")
    );
    assert_eq!(
        project.calls(),
        3,
        "article, tag, and UI are translated by default"
    );
    assert_eq!(
        project.read_json("content/tags.json")["entries"]["Rust"]["translation"]["value"],
        "English Rust"
    );
    assert_eq!(
        project.read_json("crates/server/locales/ui.json")["entries"]["greeting"]["translation"]["value"],
        "English こんにちは"
    );

    let first = fs::read(&english).unwrap();
    let calls = project.calls();
    project.run(&[]);
    assert_eq!(project.calls(), calls, "unchanged content must not call AI");
    assert_eq!(fs::read(&english).unwrap(), first);
}

#[test]
fn default_export_translates_only_the_changed_article() {
    let project = Project::new();
    project.run(&[]);
    let calls = project.calls();
    project.write_note("更新した本文");
    project.run(&[]);
    assert_eq!(project.calls(), calls + 1);
    assert!(
        fs::read_to_string(project.path(&format!("content/en/{ARTICLE}.md")))
            .unwrap()
            .contains("English 更新した本文")
    );
}

#[test]
fn manual_edits_are_preserved_and_matching_candidates_are_reused() {
    let project = Project::new();
    let result = project.prepare_review_candidates();
    let english = project.path(&format!("content/en/{ARTICLE}.md"));
    assert!(
        fs::read_to_string(&english)
            .unwrap()
            .contains("Hand edited prose")
    );
    assert_eq!(
        project.read_json("content/tags.json")["entries"]["Rust"]["translation"]["value"],
        "Hand edited tag"
    );
    assert_eq!(
        project.read_json("crates/server/locales/ui.json")["entries"]["greeting"]["translation"]["value"],
        "Hand edited greeting"
    );
    let candidate = project.path(&format!("content/.export-candidates/{ARTICLE}.md"));
    assert!(candidate.exists());
    let logs = String::from_utf8(result.stderr).unwrap();
    for command in [
        format!("cargo run -p export -- accept-article '{ARTICLE}'"),
        "cargo run -p export -- accept-tag 'Rust'".into(),
        "cargo run -p export -- accept-ui 'greeting'".into(),
    ] {
        assert!(
            logs.contains(&command),
            "missing command: {command}\nlogs:\n{logs}"
        );
    }

    let calls = project.calls();
    project.run(&[]);
    assert_eq!(project.calls(), calls, "matching candidates must be reused");
}

#[test]
fn accepting_candidates_needs_neither_private_input_nor_ai() {
    let project = Project::new();
    project.prepare_review_candidates();
    let calls = project.calls();
    let english = project.path(&format!("content/en/{ARTICLE}.md"));
    let candidate = project.path(&format!("content/.export-candidates/{ARTICLE}.md"));
    let source = project.path("obsidian");
    let away = project.path("unavailable-vault");
    fs::rename(&source, &away).unwrap();
    project.run(&["accept-article", ARTICLE]);
    assert!(!candidate.exists());
    assert!(
        fs::read_to_string(&english)
            .unwrap()
            .contains("English 更新した本文")
    );
    project.run(&["accept-tag", "Rust"]);
    assert_eq!(
        project.read_json("content/tags.json")["entries"]["Rust"]["translation"]["value"],
        "English Rust"
    );
    project.run(&["accept-ui", "greeting"]);
    assert_eq!(
        project.read_json("crates/server/locales/ui.json")["entries"]["greeting"]["translation"]["value"],
        "English 新しい挨拶"
    );
    assert_eq!(project.calls(), calls);
    fs::rename(away, source).unwrap();
    project.run(&[]);
    assert_eq!(
        project.calls(),
        calls,
        "accepted translations must be reused"
    );
}

#[test]
fn invalid_arguments_fail_before_any_export_or_translation() {
    let project = Project::new();
    for args in [
        vec!["accept"],
        vec!["accept-article"],
        vec!["accept-tag"],
        vec!["accept-ui"],
        vec!["accept-article", "../invalid"],
        vec!["--source", "somewhere"],
        vec!["--output", "elsewhere"],
        vec!["--ui-catalog", "ui.json"],
        vec!["--settings", "custom.json"],
        vec!["accept-article", ARTICLE, "--output", "elsewhere"],
        vec!["accept-tag", "Rust", "--settings", "custom.json"],
        vec!["accept-ui", "greeting", "--ui-catalog", "ui.json"],
        vec!["--translate", "--ui-only"],
        vec!["--candidates"],
    ] {
        let result = project.invoke(&args);
        assert_eq!(
            result.status.code(),
            Some(2),
            "{:?}: {}",
            args,
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!project.path("content").exists());
        assert_eq!(project.calls(), 0);
    }
    project.run(&["--help"]);
    project.run(&["--version"]);
    for command in ["accept-article", "accept-tag", "accept-ui"] {
        project.run(&[command, "--help"]);
    }
    assert!(!project.path("content").exists());
    assert_eq!(project.calls(), 0);
}

#[test]
fn codex_failure_rolls_back_japanese_and_english_and_stops_before_ui_translation() {
    let project = Project::new();
    project.run(&[]);
    let paths = [
        format!("content/ja/{ARTICLE}.md"),
        format!("content/en/{ARTICLE}.md"),
        "content/tags.json".into(),
        "crates/server/locales/ui.json".into(),
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|path| fs::read(project.path(path)).unwrap())
        .collect();
    project.write_note("更新した本文");
    fs::write(project.path("bin/codex"), "#!/bin/sh\nexit 23\n").unwrap();

    let result = project.invoke(&[]);
    assert!(!result.status.success());
    for (path, expected) in paths.iter().zip(before) {
        assert_eq!(fs::read(project.path(path)).unwrap(), expected, "{path}");
    }
    let logs = String::from_utf8(result.stderr).unwrap();
    assert!(logs.contains("translation provider failed"));
    assert!(!logs.contains("UI translation started"));
}

#[test]
fn missing_private_input_does_not_fall_back_to_public_only_translation() {
    let project = Project::new();
    fs::rename(project.path("obsidian"), project.path("unavailable-vault")).unwrap();
    let result = project.invoke(&[]);
    assert!(!result.status.success());
    assert!(!project.path("content").exists());
    assert_eq!(project.calls(), 0);
}
