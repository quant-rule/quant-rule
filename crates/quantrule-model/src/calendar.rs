//! The declared trading calendar used to compute session-aware coverage.
//!
//! A source can trade continuously or only inside a weekly window. The calendar
//! answers one question: is a bar whose open time is `t` expected to exist? It
//! is a pure value: no I/O, no clock, no timezone database. Weekdays are
//! 0 = Sunday through 6 = Saturday, and times are UTC minutes from midnight.
//!
//! A weekly window is half-open: the open minute is expected and the close
//! minute is not. A declared holiday closes the whole UTC day. Partial
//! holidays are not represented.

use std::fmt;

/// The declared trading calendar of one instrument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradingCalendar {
    /// The market trades every minute of every day.
    Always,
    /// The market trades from `open_weekday`/`open_minute` until
    /// `close_weekday`/`close_minute`, with full-day holidays.
    Weekly {
        /// 0 = Sunday through 6 = Saturday.
        open_weekday: u8,
        /// UTC minutes from midnight, inclusive.
        open_minute: u16,
        /// 0 = Sunday through 6 = Saturday.
        close_weekday: u8,
        /// UTC minutes from midnight, exclusive.
        close_minute: u16,
        /// UTC day numbers since 1970-01-01, sorted and deduplicated.
        holidays: Vec<i64>,
    },
}

/// A calendar value is outside its contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarError {
    /// The weekday is outside 0..=6.
    Weekday(u8),
    /// The minute is outside 0..=1439.
    Minute(u16),
    /// The open and close are equal; use [`TradingCalendar::Always`] instead.
    EmptyWindow,
}

impl fmt::Display for CalendarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Weekday(value) => write!(formatter, "the weekday {value} is outside 0..=6"),
            Self::Minute(value) => write!(formatter, "the minute {value} is outside 0..=1439"),
            Self::EmptyWindow => write!(formatter, "the open and close are equal"),
        }
    }
}

impl std::error::Error for CalendarError {}

impl TradingCalendar {
    /// A 24×7 calendar.
    #[must_use]
    pub const fn always() -> Self {
        Self::Always
    }

    /// Builds a weekly calendar and sorts its holidays.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarError`] when a weekday or minute is out of range, or
    /// when the open and close are equal.
    pub fn weekly(
        open_weekday: u8,
        open_minute: u16,
        close_weekday: u8,
        close_minute: u16,
        mut holidays: Vec<i64>,
    ) -> Result<Self, CalendarError> {
        if open_weekday > 6 {
            return Err(CalendarError::Weekday(open_weekday));
        }
        if close_weekday > 6 {
            return Err(CalendarError::Weekday(close_weekday));
        }
        if open_minute > 1_439 {
            return Err(CalendarError::Minute(open_minute));
        }
        if close_minute > 1_439 {
            return Err(CalendarError::Minute(close_minute));
        }
        if open_weekday == close_weekday && open_minute == close_minute {
            return Err(CalendarError::EmptyWindow);
        }
        holidays.sort_unstable();
        holidays.dedup();
        Ok(Self::Weekly {
            open_weekday,
            open_minute,
            close_weekday,
            close_minute,
            holidays,
        })
    }

    /// Whether the calendar is 24×7.
    #[must_use]
    pub const fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }

    /// Counts the bars expected in `[first_open_time, last_open_time]`.
    ///
    /// The count is inclusive of both endpoints and steps by
    /// `timeframe_secs`. A zero timeframe, or a last time before the first,
    /// returns zero.
    #[must_use]
    pub fn expected_bars(
        &self,
        first_open_time: i64,
        last_open_time: i64,
        timeframe_secs: u64,
    ) -> usize {
        if timeframe_secs == 0 || last_open_time < first_open_time {
            return 0;
        }
        let step = i64::try_from(timeframe_secs).unwrap_or(i64::MAX);
        if step == 0 {
            return 0;
        }
        match self {
            Self::Always => {
                let steps = (last_open_time - first_open_time) / step;
                usize::try_from(steps.saturating_add(1)).unwrap_or(usize::MAX)
            }
            Self::Weekly {
                open_weekday,
                open_minute,
                close_weekday,
                close_minute,
                holidays,
            } => {
                let mut count = 0_usize;
                let mut timestamp = first_open_time;
                while timestamp <= last_open_time {
                    if weekly_open(
                        timestamp,
                        *open_weekday,
                        *open_minute,
                        *close_weekday,
                        *close_minute,
                        holidays,
                    ) {
                        count = count.saturating_add(1);
                    }
                    timestamp = timestamp.saturating_add(step);
                }
                count
            }
        }
    }
}

/// Whether the UTC timestamp is inside the weekly window and not on a holiday.
fn weekly_open(
    timestamp: i64,
    open_weekday: u8,
    open_minute: u16,
    close_weekday: u8,
    close_minute: u16,
    holidays: &[i64],
) -> bool {
    let day = timestamp.div_euclid(86_400);
    if holidays.binary_search(&day).is_ok() {
        return false;
    }
    let minute = timestamp.rem_euclid(86_400) / 60;
    let week_minute = u32::from(weekday(day)) * 1_440 + u32::try_from(minute).unwrap_or(0);
    let open = u32::from(open_weekday) * 1_440 + u32::from(open_minute);
    let close = u32::from(close_weekday) * 1_440 + u32::from(close_minute);
    if open <= close {
        week_minute >= open && week_minute < close
    } else {
        week_minute >= open || week_minute < close
    }
}

/// The UTC weekday, 0 = Sunday through 6 = Saturday.
fn weekday(day: i64) -> u8 {
    u8::try_from((day + 4).rem_euclid(7)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{CalendarError, TradingCalendar};

    fn weekly() -> TradingCalendar {
        TradingCalendar::weekly(0, 22 * 60, 5, 21 * 60, Vec::new()).expect("a valid calendar")
    }

    #[test]
    fn always_counts_the_inclusive_span() {
        let calendar = TradingCalendar::always();
        assert_eq!(calendar.expected_bars(0, 900, 900), 2);
        assert_eq!(calendar.expected_bars(0, 0, 900), 1);
        assert_eq!(calendar.expected_bars(900, 0, 900), 0);
        assert_eq!(calendar.expected_bars(0, 900, 0), 0);
    }

    #[test]
    fn a_weekly_window_opens_sunday_and_closes_friday() {
        let calendar = weekly();
        // 2024-01-07 is a Sunday; 22:00 UTC is the open.
        let sunday_2200 = 1_704_664_800;
        assert_eq!(calendar.expected_bars(sunday_2200, sunday_2200, 3_600), 1);
        // Monday 00:00 is open; both endpoints are inclusive.
        assert_eq!(
            calendar.expected_bars(sunday_2200, sunday_2200 + 2 * 3_600, 3_600),
            3
        );
        // Friday 21:00 is the exclusive close.
        let friday_2100 = sunday_2200 + 4 * 86_400 + 23 * 3_600;
        assert_eq!(calendar.expected_bars(friday_2100, friday_2100, 3_600), 0);
        // Saturday is closed.
        let saturday_1200 = sunday_2200 + 5 * 86_400 + 14 * 3_600;
        assert_eq!(
            calendar.expected_bars(saturday_1200, saturday_1200, 3_600),
            0
        );
    }

    #[test]
    fn a_holiday_closes_the_whole_day() {
        let holiday = 1_704_672_000 / 86_400; // 2024-01-08, a Monday
        let calendar = TradingCalendar::weekly(0, 22 * 60, 5, 21 * 60, vec![holiday])
            .expect("a valid calendar");
        let monday_0000 = holiday * 86_400;
        assert_eq!(calendar.expected_bars(monday_0000, monday_0000, 3_600), 0);
        let tuesday_0000 = (holiday + 1) * 86_400;
        assert_eq!(calendar.expected_bars(tuesday_0000, tuesday_0000, 3_600), 1);
    }

    #[test]
    fn a_bad_calendar_is_rejected() {
        assert_eq!(
            TradingCalendar::weekly(7, 0, 5, 0, Vec::new()),
            Err(CalendarError::Weekday(7))
        );
        assert_eq!(
            TradingCalendar::weekly(0, 1_440, 5, 0, Vec::new()),
            Err(CalendarError::Minute(1_440))
        );
        assert_eq!(
            TradingCalendar::weekly(0, 0, 0, 0, Vec::new()),
            Err(CalendarError::EmptyWindow)
        );
    }
}
