use jiff::{civil::DateTime, fmt::temporal::Pieces, tz::Offset};

pub(crate) fn parse(value: &str) -> Option<(DateTime, Offset)> {
    let (date, time) = value.split_once(['T', 't', ' '])?;
    let (clock, tail) = time.split_at_checked(8)?;
    // Pieces also accepts basic dates and abbreviated times.
    let [_, _, _, _, b'-', _, _, b'-', _, _] = date.as_bytes() else {
        return None;
    };
    let [_, _, b':', _, _, b':', _, _] = clock.as_bytes() else {
        return None;
    };

    let offset = if let Some(fraction) = tail.strip_prefix('.') {
        let offset = fraction.trim_start_matches(|c: char| c.is_ascii_digit());
        if fraction.len() == offset.len() {
            return None;
        }
        offset
    } else {
        tail
    };
    // Restrict numeric offsets to ±HH:MM; Pieces validates the digits and ranges.
    if !matches!(offset, "Z" | "z") {
        let hm = offset.strip_prefix(['+', '-', '−'])?;
        if hm.len() != 5 || hm.as_bytes()[2] != b':' {
            return None;
        }
    }

    // Older artifacts may use a Unicode minus or more than nine fractional digits.
    const MAX_LOCAL_LEN: usize = "0000-00-00T00:00:00.000000000".len();
    let normalized;
    let pieces = match Pieces::parse(value) {
        Ok(pieces) => pieces,
        Err(_) => {
            let local = value.strip_suffix(offset)?;
            let local = local.get(..MAX_LOCAL_LEN).unwrap_or(local);
            normalized = format!("{local}{}", offset.replacen('−', "-", 1));
            Pieces::parse(&normalized).ok()?
        }
    };
    let offset = pieces.to_numeric_offset()?;
    (offset.seconds().unsigned_abs() < 24 * 60 * 60)
        .then_some((pieces.date().to_datetime(pieces.time()?), offset))
}
