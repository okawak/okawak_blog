use jiff::{civil::DateTime, fmt::temporal::Pieces, tz::Offset};
use std::borrow::Cow;

pub(crate) fn parse(value: &str) -> Option<(DateTime, Offset)> {
    // Jiff's Temporal parser also accepts ISO 8601 dates, abbreviated times
    // and RFC 9557 annotations. Keep the public RFC 3339 input shape.
    let (datetime, tail) = value.split_at_checked(19)?;
    if !matches!(
        datetime.as_bytes(),
        [
            _,
            _,
            _,
            _,
            b'-',
            _,
            _,
            b'-',
            _,
            _,
            b'T' | b't' | b' ',
            _,
            _,
            b':',
            _,
            _,
            b':',
            _,
            _
        ]
    ) {
        return None;
    }
    let (fraction, offset) = if let Some(digits) = tail.strip_prefix('.') {
        let offset = digits.trim_start_matches(|c: char| c.is_ascii_digit());
        let fraction_len = digits.len() - offset.len();
        if fraction_len == 0 {
            return None;
        }
        (&tail[..=fraction_len.min(9)], offset)
    } else {
        ("", tail)
    };
    if !matches!(offset, "Z" | "z") {
        let digits = offset.strip_prefix(['+', '-', '−'])?;
        if !matches!(
            digits.as_bytes(),
            [b'0'..=b'9', b'0'..=b'9', b':', b'0'..=b'5', b'0'..=b'9']
        ) {
            return None;
        }
    }
    // Preserve the previous nanosecond truncation and Unicode minus support.
    let value = if let Some(digits) = offset.strip_prefix('−') {
        Cow::Owned(format!("{datetime}{fraction}-{digits}"))
    } else if datetime.len() + fraction.len() + offset.len() != value.len() {
        Cow::Owned(format!("{datetime}{fraction}{offset}"))
    } else {
        Cow::Borrowed(value)
    };
    let pieces = Pieces::parse(value.as_bytes()).ok()?;
    let offset = pieces.to_numeric_offset()?;
    if offset.seconds().unsigned_abs() >= 24 * 60 * 60 {
        return None;
    }
    Some((pieces.date().to_datetime(pieces.time()?), offset))
}
