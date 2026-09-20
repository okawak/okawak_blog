use domain::Slug;
use export::{ProtectedContent, TranslationReport};
use std::path::Path;

enum AcceptanceTarget<'a> {
    Article(&'a Slug),
    Tag(&'a str),
    Ui(&'a str),
}

pub(super) fn content(
    report: &TranslationReport<ProtectedContent>,
    output: &Path,
    settings: &Path,
) {
    summary("Content", report);
    for command in content_acceptance_commands(report, output, settings) {
        println!("Review the candidate, then run `{command}`.");
    }
}

pub(super) fn ui(report: &TranslationReport<String>, catalog: &Path, settings: &Path) {
    summary("UI", report);
    for key in &report.protected {
        let command = acceptance_command(AcceptanceTarget::Ui(key), catalog, settings);
        println!("Review the candidate, then run `{command}`.");
    }
}

fn summary<T>(scope: &str, report: &TranslationReport<T>) {
    println!(
        "{scope}: generated {}, reused {}, protected {}",
        report.generated,
        report.reused,
        report.protected.len()
    );
}

fn content_acceptance_commands(
    report: &TranslationReport<ProtectedContent>,
    output: &Path,
    settings: &Path,
) -> Vec<String> {
    report
        .protected
        .iter()
        .map(|item| {
            let target = match item {
                ProtectedContent::Article(id) => AcceptanceTarget::Article(id),
                ProtectedContent::Tag(id) => AcceptanceTarget::Tag(id),
            };
            acceptance_command(target, output, settings)
        })
        .collect()
}

fn acceptance_command(target: AcceptanceTarget<'_>, path: &Path, settings: &Path) -> String {
    let (target, id, path_option) = match target {
        AcceptanceTarget::Article(id) => ("article", id.as_str(), "output"),
        AcceptanceTarget::Tag(id) => ("tag", id, "output"),
        AcceptanceTarget::Ui(id) => ("ui", id, "ui-catalog"),
    };
    format!(
        "cargo run -p export -- accept {target} {} --{path_option} {} --settings {}",
        shell_argument(id),
        shell_argument(&path.to_string_lossy()),
        shell_argument(&settings.to_string_lossy())
    )
}

fn shell_argument(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::{AcceptanceTarget, acceptance_command, content_acceptance_commands};
    use domain::Slug;
    use export::{ProtectedContent, TranslationReport};
    use std::path::Path;

    #[test]
    fn acceptance_commands_preserve_typed_targets_paths_and_shell_arguments() {
        let report = TranslationReport {
            generated: 2,
            reused: 0,
            protected: vec![
                ProtectedContent::Article(Slug::new("article-1".into()).unwrap()),
                ProtectedContent::Tag("author's note".into()),
            ],
        };

        assert_eq!(
            content_acceptance_commands(
                &report,
                Path::new("public content"),
                Path::new("translation.json")
            ),
            [
                "cargo run -p export -- accept article 'article-1' --output 'public content' --settings 'translation.json'",
                "cargo run -p export -- accept tag 'author'\"'\"'s note' --output 'public content' --settings 'translation.json'",
            ]
        );
        assert_eq!(
            acceptance_command(
                AcceptanceTarget::Ui("greeting"),
                Path::new("locales/ui.json"),
                Path::new("translation.json")
            ),
            "cargo run -p export -- accept ui 'greeting' --ui-catalog 'locales/ui.json' --settings 'translation.json'"
        );
    }
}
