use blogcore::Locale;
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime};
use topcoat::l10n::{DateTime as LocalizedDateTime, DateTimeStyle, Number, locale};

/// Keep framework locale formatting at the server boundary.
fn formatting_locale(locale: Locale) -> topcoat::l10n::Locale {
    match locale {
        Locale::Ja => locale!("ja"),
        Locale::En => locale!("en"),
    }
}

pub(crate) fn format_count(count: usize, locale: Locale) -> String {
    Number::new(count as f64).resolve_in(&formatting_locale(locale))
}

/// Formats an artifact date for human-readable presentation.
pub(crate) fn format_display_date(value: &str, locale: Locale) -> String {
    let date = DateTime::parse_from_rfc3339(value)
        .map(|date_time| date_time.date_naive())
        .or_else(|_| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").map(|date_time| date_time.date())
        })
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|date_time| date_time.date())
        });

    match date {
        Ok(date) => {
            // Preserve the date in the artifact's offset; do not convert it to UTC.
            LocalizedDateTime::date(date.year(), date.month() as u8, date.day() as u8)
                .expect("chrono date is a valid ISO date")
                .date_style(if locale == Locale::Ja {
                    DateTimeStyle::Long
                } else {
                    DateTimeStyle::Medium
                })
                .resolve_in(&formatting_locale(locale))
        }
        Err(_) => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::format_display_date;
    use blogcore::Locale;

    #[test]
    fn formats_rfc3339_without_exposing_time_or_offset() {
        assert_eq!(
            format_display_date("2026-01-02T00:00:00+09:00", Locale::Ja),
            "2026年1月2日"
        );
    }

    #[test]
    fn formats_date_only_values() {
        assert_eq!(
            format_display_date("2026-01-02", Locale::Ja),
            "2026年1月2日"
        );
    }

    #[test]
    fn formats_english_dates_and_keeps_the_artifact_calendar_day() {
        for value in ["2026-01-02", "2026-01-02T00:30:00+09:00"] {
            assert_eq!(format_display_date(value, Locale::En), "Jan 2, 2026");
        }
        assert_eq!(
            format_display_date("2024-02-29", Locale::Ja),
            "2024年2月29日"
        );
        assert_eq!(format_display_date("2025-02-29", Locale::En), "2025-02-29");
    }

    #[test]
    fn formats_local_datetime_values() {
        for value in ["2025-05-04T16:50", "2025-05-04T16:50:30"] {
            assert_eq!(format_display_date(value, Locale::Ja), "2025年5月4日");
        }
    }

    #[test]
    fn preserves_unknown_values() {
        assert_eq!(format_display_date("unknown", Locale::Ja), "unknown");
    }
}
