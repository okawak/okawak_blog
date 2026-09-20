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
        let path = self.path("crates/publish/obsidian/Publish/tech/article.md");
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
    project.run(&[]);
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
    let stdout = String::from_utf8(result.stdout).unwrap();
    for command in [
        format!("cargo run -p export -- accept article '{ARTICLE}' --output 'content' --settings 'translation.json'"),
        "cargo run -p export -- accept tag 'Rust' --output 'content' --settings 'translation.json'".into(),
        "cargo run -p export -- accept ui 'greeting' --ui-catalog 'crates/server/locales/ui.json' --settings 'translation.json'".into(),
    ] {
        assert!(stdout.contains(&command), "missing command: {command}");
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
    let source = project.path("crates/publish/obsidian");
    let away = project.path("unavailable-vault");
    fs::rename(&source, &away).unwrap();
    project.run(&["accept", "article", ARTICLE]);
    assert!(!candidate.exists());
    assert!(
        fs::read_to_string(&english)
            .unwrap()
            .contains("English 更新した本文")
    );
    project.run(&["accept", "tag", "Rust"]);
    assert_eq!(
        project.read_json("content/tags.json")["entries"]["Rust"]["translation"]["value"],
        "English Rust"
    );
    project.run(&["accept", "ui", "greeting"]);
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
fn invalid_or_ambiguous_arguments_fail_before_any_export_or_translation() {
    let project = Project::new();
    for args in [
        vec!["accept"],
        vec!["accept", "article"],
        vec!["accept", "article", "../invalid"],
        vec!["--settings"],
        vec!["--translate", "--ui-only"],
        vec!["--candidates"],
        vec!["--source", "somewhere", "accept", "article", ARTICLE],
        vec!["accept", "article", ARTICLE, "--ui-catalog", "ui.json"],
        vec!["accept", "ui", "greeting", "--output", "elsewhere"],
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
    project.run(&["accept", "--help"]);
    assert!(!project.path("content").exists());
    assert_eq!(project.calls(), 0);
}

#[test]
fn missing_private_input_does_not_fall_back_to_public_only_translation() {
    let project = Project::new();
    fs::rename(
        project.path("crates/publish/obsidian"),
        project.path("unavailable-vault"),
    )
    .unwrap();
    let result = project.invoke(&[]);
    assert!(!result.status.success());
    assert!(!project.path("content").exists());
    assert_eq!(project.calls(), 0);
}

#[test]
fn explicit_paths_are_used_for_export_and_candidate_acceptance() {
    let project = Project::new();
    fs::rename(
        project.path("crates/publish/obsidian/Publish"),
        project.path("custom vault"),
    )
    .unwrap();
    fs::rename(
        project.path("translation.json"),
        project.path("custom settings.json"),
    )
    .unwrap();
    fs::rename(
        project.path("crates/server/locales/ui.json"),
        project.path("custom ui.json"),
    )
    .unwrap();
    let args = [
        "--source",
        "custom vault",
        "--output",
        "custom content",
        "--ui-catalog",
        "custom ui.json",
        "--settings",
        "custom settings.json",
    ];
    project.run(&args);
    assert!(
        project
            .path(&format!("custom content/en/{ARTICLE}.md"))
            .exists()
    );
    assert!(!project.path("content").exists());

    let mut ui = project.read_json("custom ui.json");
    ui["entries"]["greeting"]["translation"]["value"] = json!("Hand edited greeting");
    ui["entries"]["greeting"]["source"] = json!("新しい挨拶");
    project.write_json("custom ui.json", ui);
    project.run(&args);
    assert_eq!(
        project.read_json("custom ui.json")["entries"]["greeting"]["translation"]["value"],
        "Hand edited greeting"
    );
    let calls = project.calls();
    project.run(&[
        "accept",
        "ui",
        "greeting",
        "--ui-catalog",
        "custom ui.json",
        "--settings",
        "custom settings.json",
    ]);
    assert_eq!(
        project.read_json("custom ui.json")["entries"]["greeting"]["translation"]["value"],
        "English 新しい挨拶"
    );
    assert_eq!(project.calls(), calls);
}
