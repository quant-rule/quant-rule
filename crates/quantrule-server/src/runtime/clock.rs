//! UTC timestamps at the service boundary.
//!
//! Engines never read a clock. A service may, and formats what it read as
//! RFC 3339 UTC so the record says when it happened.

/// Format Unix seconds as an RFC 3339 UTC timestamp.
#[must_use]
pub fn rfc3339(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    let hour = rest / 3_600;
    let minute = (rest % 3_600) / 60;
    let second = rest % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// The current time as RFC 3339 UTC.
///
/// # Panics
///
/// Panics only when the system clock is before the Unix epoch, which a correct
/// host does not report.
#[must_use]
pub fn now_utc() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the system clock is after the Unix epoch")
        .as_secs();
    rfc3339(seconds)
}

/// Howard Hinnant's days-to-civil conversion, which avoids a date dependency.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_1970() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_known_instant_is_exact() {
        // 2026-11-13T00:00:00Z
        assert_eq!(rfc3339(1_794_528_000), "2026-11-13T00:00:00Z");
    }

    #[test]
    fn a_leap_day_is_correct() {
        // 2024-02-29T12:34:56Z
        assert_eq!(rfc3339(1_709_210_096), "2024-02-29T12:34:56Z");
    }

    #[test]
    fn the_time_of_day_is_padded() {
        assert_eq!(rfc3339(59), "1970-01-01T00:00:59Z");
    }

    #[test]
    fn now_is_rfc3339_shaped() {
        let now = now_utc();
        assert_eq!(now.len(), 20, "{now}");
        assert!(now.ends_with('Z'));
    }
}
