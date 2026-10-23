//! UTC timestamp parsing and formatting.
//!
//! The source format is `YYYY-MM-DD HH:MM:SS+00:00`. The shell renders the
//! engine's Unix seconds as `YYYY-MM-DDTHH:MM:SSZ`. Both formats are UTC; no
//! local timezone or clock is involved.

use std::fmt;

const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;
const SOURCE_TIMESTAMP_LEN: usize = 25;

/// A timestamp could not be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampError {
    /// The input does not have the fixed source length.
    InvalidLength,
    /// A separator or digit is in the wrong place.
    InvalidFormat,
    /// The date is not a valid calendar date.
    InvalidDate,
    /// The time is not a valid UTC time.
    InvalidTime,
    /// The year is outside the supported range.
    UnsupportedYear,
    /// The offset is not `+00:00`.
    UnsupportedOffset,
}

impl fmt::Display for TimestampError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLength => "timestamp has the wrong length",
            Self::InvalidFormat => "timestamp has an invalid format",
            Self::InvalidDate => "timestamp has an invalid date",
            Self::InvalidTime => "timestamp has an invalid time",
            Self::UnsupportedYear => "timestamp year is outside 1970..=9999",
            Self::UnsupportedOffset => "timestamp offset is not +00:00",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TimestampError {}

/// Parses a source timestamp into UTC Unix seconds.
///
/// The accepted shape is exactly `YYYY-MM-DD HH:MM:SS+00:00`, with years from
/// 1970 through 9999. Seconds are whole seconds; leap seconds are not
/// represented.
///
/// # Errors
///
/// Returns a [`TimestampError`] when the input is malformed, not a real
/// calendar date, has an out-of-range time, or uses an offset other than
/// `+00:00`.
///
/// # Examples
///
/// ```
/// use quantrule_data::parse_utc_timestamp;
///
/// let seconds = parse_utc_timestamp("1970-01-01 00:00:00+00:00")
///     .expect("the epoch is a valid timestamp");
/// assert_eq!(seconds, 0);
/// ```
pub fn parse_utc_timestamp(input: &str) -> Result<i64, TimestampError> {
    let bytes = input.as_bytes();
    if bytes.len() != SOURCE_TIMESTAMP_LEN {
        return Err(TimestampError::InvalidLength);
    }

    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b' '
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err(TimestampError::InvalidFormat);
    }

    if bytes[19] != b'+' || &bytes[20..22] != b"00" || bytes[22] != b':' || &bytes[23..25] != b"00"
    {
        return Err(TimestampError::UnsupportedOffset);
    }

    let year = parse_digits(bytes, 0, 4)?;
    let month = parse_digits(bytes, 5, 2)?;
    let day = parse_digits(bytes, 8, 2)?;
    let hour = parse_digits(bytes, 11, 2)?;
    let minute = parse_digits(bytes, 14, 2)?;
    let second = parse_digits(bytes, 17, 2)?;

    if !(1970..=9999).contains(&year) {
        return Err(TimestampError::UnsupportedYear);
    }
    if !(1..=12).contains(&month) {
        return Err(TimestampError::InvalidDate);
    }
    if day == 0 || day > days_in_month(year, month) {
        return Err(TimestampError::InvalidDate);
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err(TimestampError::InvalidTime);
    }

    let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
    let seconds = days * SECONDS_PER_DAY
        + i64::from(hour) * SECONDS_PER_HOUR
        + i64::from(minute) * SECONDS_PER_MINUTE
        + i64::from(second);

    Ok(seconds)
}

/// Formats UTC Unix seconds as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// The output is the RFC 3339 form used by the shell's NDJSON. The function is
/// pure: it does not consult the system clock or a local timezone.
///
/// # Examples
///
/// ```
/// use quantrule_data::format_utc_timestamp;
///
/// assert_eq!(format_utc_timestamp(0), "1970-01-01T00:00:00Z");
/// ```
#[must_use]
pub fn format_utc_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let remainder = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / SECONDS_PER_HOUR;
    let minute = (remainder % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let second = remainder % SECONDS_PER_MINUTE;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn parse_digits(bytes: &[u8], start: usize, len: usize) -> Result<u32, TimestampError> {
    let mut value = 0_u32;
    for byte in &bytes[start..start + len] {
        if !byte.is_ascii_digit() {
            return Err(TimestampError::InvalidFormat);
        }
        value = value * 10 + u32::from(*byte - b'0');
    }
    Ok(value)
}

const fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{TimestampError, format_utc_timestamp, parse_utc_timestamp};

    #[test]
    fn parses_the_epoch() {
        assert_eq!(parse_utc_timestamp("1970-01-01 00:00:00+00:00"), Ok(0));
    }

    #[test]
    fn parses_a_known_timestamp() {
        assert_eq!(
            parse_utc_timestamp("2025-01-01 22:00:00+00:00"),
            Ok(1_735_768_800)
        );
    }

    #[test]
    fn parses_a_leap_day() {
        assert_eq!(
            parse_utc_timestamp("2024-02-29 00:00:00+00:00"),
            Ok(1_709_164_800)
        );
    }

    #[test]
    fn rejects_a_non_leap_day() {
        assert_eq!(
            parse_utc_timestamp("2023-02-29 00:00:00+00:00"),
            Err(TimestampError::InvalidDate)
        );
    }

    #[test]
    fn accepts_a_century_leap_year() {
        assert!(parse_utc_timestamp("2000-02-29 00:00:00+00:00").is_ok());
    }

    #[test]
    fn rejects_a_non_leap_century() {
        assert_eq!(
            parse_utc_timestamp("2100-02-29 00:00:00+00:00"),
            Err(TimestampError::InvalidDate)
        );
    }

    #[test]
    fn rejects_an_out_of_range_month() {
        assert_eq!(
            parse_utc_timestamp("2025-13-01 00:00:00+00:00"),
            Err(TimestampError::InvalidDate)
        );
    }

    #[test]
    fn rejects_an_out_of_range_time() {
        assert_eq!(
            parse_utc_timestamp("2025-01-01 24:00:00+00:00"),
            Err(TimestampError::InvalidTime)
        );
    }

    #[test]
    fn rejects_a_wrong_length() {
        assert_eq!(
            parse_utc_timestamp("2025-01-01 00:00:00Z"),
            Err(TimestampError::InvalidLength)
        );
    }

    #[test]
    fn rejects_a_wrong_separator() {
        assert_eq!(
            parse_utc_timestamp("2025/01/01 00:00:00+00:00"),
            Err(TimestampError::InvalidFormat)
        );
    }

    #[test]
    fn rejects_a_non_digit_field() {
        assert_eq!(
            parse_utc_timestamp("20x5-01-01 00:00:00+00:00"),
            Err(TimestampError::InvalidFormat)
        );
    }

    #[test]
    fn rejects_an_offset_other_than_utc() {
        assert_eq!(
            parse_utc_timestamp("2025-01-01 00:00:00+01:00"),
            Err(TimestampError::UnsupportedOffset)
        );
    }

    #[test]
    fn rejects_a_year_before_the_supported_range() {
        assert_eq!(
            parse_utc_timestamp("1969-12-31 23:59:59+00:00"),
            Err(TimestampError::UnsupportedYear)
        );
    }

    #[test]
    fn formats_the_epoch() {
        assert_eq!(format_utc_timestamp(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn round_trips_a_timestamp() {
        let text = "2025-12-31 23:59:59+00:00";
        let seconds = parse_utc_timestamp(text).expect("the timestamp is valid");

        assert_eq!(format_utc_timestamp(seconds), "2025-12-31T23:59:59Z");
    }

    #[test]
    fn formats_a_leap_day() {
        let seconds =
            parse_utc_timestamp("2024-02-29 12:34:56+00:00").expect("the timestamp is valid");

        assert_eq!(format_utc_timestamp(seconds), "2024-02-29T12:34:56Z");
    }
}
