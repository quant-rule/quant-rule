//! Explicit UTC date windows for source requests.
//!
//! The shell accepts two date-only strings and converts them to a half-open
//! second interval: the start is inclusive and the end is exclusive and is
//! exactly the midnight after the supplied end date. No clock is read; the
//! dates are the entire definition of the window.

use std::fmt;

use quantrule_data::{TimestampError, parse_utc_timestamp};

/// One UTC day in seconds.
const SECONDS_PER_DAY: i64 = 86_400;

/// A validated inclusive date range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DateRange {
    start: String,
    end: String,
    start_seconds: i64,
    end_exclusive_seconds: i64,
}

impl DateRange {
    /// Parses two `YYYY-MM-DD` dates into an inclusive range.
    ///
    /// Both dates are UTC and the end date covers its whole day. The range is
    /// valid when the start is not later than the end; a single-day range is
    /// accepted.
    ///
    /// # Errors
    ///
    /// Returns a [`DateRangeError`] when either date is not a real calendar
    /// date, when the start is after the end, or when the exclusive end would
    /// overflow.
    pub(crate) fn parse(start: &str, end: &str) -> Result<Self, DateRangeError> {
        let start_seconds = parse_date(start)?;
        let end_seconds = parse_date(end)?;

        if start_seconds > end_seconds {
            return Err(DateRangeError::StartAfterEnd {
                start: start.to_owned(),
                end: end.to_owned(),
            });
        }

        let end_exclusive_seconds = end_seconds
            .checked_add(SECONDS_PER_DAY)
            .ok_or(DateRangeError::Overflow)?;

        Ok(Self {
            start: start.to_owned(),
            end: end.to_owned(),
            start_seconds,
            end_exclusive_seconds,
        })
    }

    /// The first UTC date, as supplied, in `YYYY-MM-DD` form.
    pub(crate) fn start_label(&self) -> &str {
        &self.start
    }

    /// The last UTC date, as supplied, in `YYYY-MM-DD` form.
    pub(crate) fn end_label(&self) -> &str {
        &self.end
    }

    /// The inclusive start as UTC Unix seconds.
    pub(crate) const fn start_seconds(&self) -> i64 {
        self.start_seconds
    }

    /// The exclusive end as UTC Unix seconds.
    pub(crate) const fn end_exclusive_seconds(&self) -> i64 {
        self.end_exclusive_seconds
    }

    /// The `start_end` suffix used in output file names.
    pub(crate) fn file_suffix(&self) -> String {
        format!("{}_{}", self.start, self.end)
    }
}

/// A date range could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DateRangeError {
    /// A date is not a valid `YYYY-MM-DD` UTC date.
    InvalidDate {
        /// The supplied text.
        text: String,
        /// The timestamp parser's reason.
        source: TimestampError,
    },
    /// The start date is later than the end date.
    StartAfterEnd {
        /// The supplied start date.
        start: String,
        /// The supplied end date.
        end: String,
    },
    /// The exclusive end overflowed the supported Unix-second range.
    Overflow,
}

impl fmt::Display for DateRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDate { text, source } => {
                write!(formatter, "invalid UTC date '{text}': {source}")
            }
            Self::StartAfterEnd { start, end } => {
                write!(formatter, "start date {start} is after end date {end}")
            }
            Self::Overflow => formatter.write_str("date range is outside the supported range"),
        }
    }
}

impl std::error::Error for DateRangeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidDate { source, .. } => Some(source),
            Self::StartAfterEnd { .. } | Self::Overflow => None,
        }
    }
}

fn parse_date(text: &str) -> Result<i64, DateRangeError> {
    if text.len() != 10
        || text.as_bytes().get(4) != Some(&b'-')
        || text.as_bytes().get(7) != Some(&b'-')
    {
        return Err(DateRangeError::InvalidDate {
            text: text.to_owned(),
            source: TimestampError::InvalidFormat,
        });
    }

    let timestamp = format!("{text} 00:00:00+00:00");
    parse_utc_timestamp(&timestamp).map_err(|source| DateRangeError::InvalidDate {
        text: text.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::{DateRange, DateRangeError};
    use quantrule_data::TimestampError;

    #[test]
    fn parses_an_inclusive_range() {
        let range =
            DateRange::parse("2025-01-01", "2025-12-31").expect("the dates are valid and ordered");

        assert_eq!(range.start_label(), "2025-01-01");
        assert_eq!(range.end_label(), "2025-12-31");
        assert_eq!(range.start_seconds(), 1_735_689_600);
        assert_eq!(range.end_exclusive_seconds(), 1_767_225_600);
        assert_eq!(range.file_suffix(), "2025-01-01_2025-12-31");
    }

    #[test]
    fn a_single_day_covers_the_whole_day() {
        let range =
            DateRange::parse("2025-01-01", "2025-01-01").expect("a single day is a valid range");

        assert_eq!(
            range.end_exclusive_seconds() - range.start_seconds(),
            86_400
        );
    }

    #[test]
    fn accepts_a_leap_day() {
        let range = DateRange::parse("2024-02-29", "2024-02-29")
            .expect("2024 is a leap year and the range is valid");

        assert_eq!(range.start_label(), "2024-02-29");
    }

    #[test]
    fn rejects_a_non_leap_day() {
        assert!(matches!(
            DateRange::parse("2023-02-29", "2023-03-01"),
            Err(DateRangeError::InvalidDate {
                source: TimestampError::InvalidDate,
                ..
            })
        ));
    }

    #[test]
    fn rejects_a_date_with_a_time() {
        assert!(matches!(
            DateRange::parse("2025-01-01T00:00", "2025-01-02"),
            Err(DateRangeError::InvalidDate { .. })
        ));
    }

    #[test]
    fn rejects_a_reversed_range() {
        assert_eq!(
            DateRange::parse("2025-01-02", "2025-01-01"),
            Err(DateRangeError::StartAfterEnd {
                start: String::from("2025-01-02"),
                end: String::from("2025-01-01"),
            })
        );
    }

    #[test]
    fn rejects_a_single_digit_month() {
        assert!(matches!(
            DateRange::parse("2025-1-01", "2025-01-02"),
            Err(DateRangeError::InvalidDate { .. })
        ));
    }
}
