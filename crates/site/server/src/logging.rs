//! Process-wide tracing subscriber configuration.

use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_FILTER: &str = "info";
const RUST_LOG_ENV: &str = "RUST_LOG";

pub(crate) fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let configured_filter = std::env::var(RUST_LOG_ENV).ok();
    let filter = build_filter(configured_filter.as_deref())?;

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()?;

    Ok(())
}

fn build_filter(value: Option<&str>) -> Result<EnvFilter, tracing_subscriber::filter::ParseError> {
    let filter = value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_LOG_FILTER);

    EnvFilter::try_new(filter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_filter_defaults_to_info() {
        assert_eq!(build_filter(None).unwrap().to_string(), "info");
    }

    #[test]
    fn blank_filter_defaults_to_info() {
        assert_eq!(build_filter(Some("  ")).unwrap().to_string(), "info");
    }

    #[test]
    fn valid_target_filters_are_preserved() {
        let filter = build_filter(Some("server=debug,topcoat=warn"))
            .unwrap()
            .to_string();

        assert!(
            filter
                .split(',')
                .any(|directive| directive == "server=debug")
        );
        assert!(
            filter
                .split(',')
                .any(|directive| directive == "topcoat=warn")
        );
    }

    #[test]
    fn invalid_filter_is_rejected() {
        assert!(build_filter(Some("server==debug")).is_err());
    }
}
