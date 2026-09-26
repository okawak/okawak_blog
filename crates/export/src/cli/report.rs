use domain::Slug;
use export::{ProtectedContent, TranslationReport};

enum AcceptanceTarget<'a> {
    Article(&'a Slug),
    Tag(&'a str),
    Ui(&'a str),
}

pub(super) fn content(report: &TranslationReport<ProtectedContent>) {
    summary("Content", report);
    for command in content_acceptance_commands(report) {
        tracing::warn!(%command, "translation candidate needs review");
    }
}

pub(super) fn ui(report: &TranslationReport<String>) {
    summary("UI", report);
    for key in &report.protected {
        let command = acceptance_command(AcceptanceTarget::Ui(key));
        tracing::warn!(%command, "translation candidate needs review");
    }
}

fn summary<T>(scope: &'static str, report: &TranslationReport<T>) {
    tracing::info!(
        scope,
        generated = report.generated,
        reused = report.reused,
        protected = report.protected.len(),
        "translation completed"
    );
}

fn content_acceptance_commands(report: &TranslationReport<ProtectedContent>) -> Vec<String> {
    report
        .protected
        .iter()
        .map(|item| {
            let target = match item {
                ProtectedContent::Article(id) => AcceptanceTarget::Article(id),
                ProtectedContent::Tag(id) => AcceptanceTarget::Tag(id),
            };
            acceptance_command(target)
        })
        .collect()
}

fn acceptance_command(target: AcceptanceTarget<'_>) -> String {
    let (target, id) = match target {
        AcceptanceTarget::Article(id) => ("article", id.as_str()),
        AcceptanceTarget::Tag(id) => ("tag", id),
        AcceptanceTarget::Ui(id) => ("ui", id),
    };
    format!(
        "cargo run -p export -- accept-{target} {}",
        shell_argument(id),
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

    #[test]
    fn acceptance_commands_preserve_typed_targets_and_shell_arguments() {
        let report = TranslationReport {
            generated: 2,
            reused: 0,
            protected: vec![
                ProtectedContent::Article(Slug::new("article-1".into()).unwrap()),
                ProtectedContent::Tag("author's note".into()),
            ],
        };

        assert_eq!(
            content_acceptance_commands(&report),
            [
                "cargo run -p export -- accept-article 'article-1'",
                "cargo run -p export -- accept-tag 'author'\"'\"'s note'",
            ]
        );
        assert_eq!(
            acceptance_command(AcceptanceTarget::Ui("greeting")),
            "cargo run -p export -- accept-ui 'greeting'"
        );
    }
}
