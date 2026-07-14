use chrono::{DateTime, Datelike, NaiveDate};

/// Formats an artifact date for human-readable presentation.
pub fn format_display_date(value: &str) -> String {
    let date = DateTime::parse_from_rfc3339(value)
        .map(|date_time| date_time.date_naive())
        .or_else(|_| NaiveDate::parse_from_str(value, "%Y-%m-%d"));

    match date {
        Ok(date) => format!("{}年{}月{}日", date.year(), date.month(), date.day()),
        Err(_) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::format_display_date;

    #[test]
    fn formats_rfc3339_without_exposing_time_or_offset() {
        assert_eq!(
            format_display_date("2026-01-02T00:00:00+09:00"),
            "2026年1月2日"
        );
    }

    #[test]
    fn formats_date_only_values() {
        assert_eq!(format_display_date("2026-01-02"), "2026年1月2日");
    }

    #[test]
    fn preserves_unknown_values() {
        assert_eq!(format_display_date("unknown"), "unknown");
    }
}
