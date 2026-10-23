//! The bounded candle window.
//!
//! The window is the data engine's memory of the recent series. It accepts raw
//! source candles one at a time, validates them, assigns their stable series
//! index, and retains at most [`DEFAULT_WINDOW`] candles. It performs no I/O
//! and reads no clock.

use std::collections::VecDeque;
use std::fmt;

use crate::candle::{Candle, CandleError, RawCandle};

/// The maximum number of candles retained by a [`CandleWindow`].
pub const DEFAULT_WINDOW: usize = 1_000;

/// A candle could not be accepted into the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    /// The candle itself was invalid.
    Candle(CandleError),
    /// The timestamp equals the last accepted timestamp.
    DuplicateTimestamp {
        /// The duplicated timestamp as UTC Unix seconds.
        timestamp: i64,
    },
    /// The timestamp is earlier than the last accepted timestamp.
    OutOfOrderTimestamp {
        /// The last accepted timestamp as UTC Unix seconds.
        previous: i64,
        /// The rejected timestamp as UTC Unix seconds.
        current: i64,
    },
}

impl From<CandleError> for WindowError {
    fn from(source: CandleError) -> Self {
        Self::Candle(source)
    }
}

impl fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Candle(source) => write!(formatter, "invalid candle: {source}"),
            Self::DuplicateTimestamp { timestamp } => {
                write!(formatter, "duplicate candle timestamp at {timestamp}")
            }
            Self::OutOfOrderTimestamp { previous, current } => {
                write!(
                    formatter,
                    "candle timestamp {current} is earlier than {previous}"
                )
            }
        }
    }
}

impl std::error::Error for WindowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Candle(source) => Some(source),
            Self::DuplicateTimestamp { .. } | Self::OutOfOrderTimestamp { .. } => None,
        }
    }
}

/// A trailing window of at most [`DEFAULT_WINDOW`] accepted candles.
///
/// The window assigns `series_index` on acceptance. The index is stable even
/// after the candle is evicted, and `get` uses the position in the retained
/// window rather than the stable index.
#[derive(Debug, Clone, PartialEq)]
pub struct CandleWindow {
    candles: VecDeque<Candle>,
    next_series_index: u64,
    last_timestamp: Option<i64>,
}

impl CandleWindow {
    /// Creates an empty window.
    #[must_use]
    pub fn new() -> Self {
        Self {
            candles: VecDeque::with_capacity(DEFAULT_WINDOW),
            next_series_index: 0,
            last_timestamp: None,
        }
    }

    /// Parses and accepts one source candle.
    ///
    /// On success, returns the stable series index assigned to the candle. A
    /// rejected candle does not consume an index and does not change the
    /// window.
    ///
    /// # Errors
    ///
    /// Returns a [`WindowError`] when the candle is invalid, when its timestamp
    /// equals the last accepted timestamp, or when its timestamp is earlier
    /// than the last accepted timestamp.
    pub fn push(&mut self, raw: RawCandle<'_>) -> Result<u64, WindowError> {
        let candle = Candle::parse(self.next_series_index, raw)?;

        if let Some(previous) = self.last_timestamp {
            if candle.timestamp() == previous {
                return Err(WindowError::DuplicateTimestamp {
                    timestamp: previous,
                });
            }
            if candle.timestamp() < previous {
                return Err(WindowError::OutOfOrderTimestamp {
                    previous,
                    current: candle.timestamp(),
                });
            }
        }

        if self.candles.len() == DEFAULT_WINDOW {
            self.candles.pop_front();
        }

        let series_index = candle.series_index();
        self.last_timestamp = Some(candle.timestamp());
        self.candles.push_back(candle);
        self.next_series_index += 1;

        Ok(series_index)
    }

    /// The number of retained candles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.candles.len()
    }

    /// Whether the window holds no candles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    /// The oldest retained candle.
    #[must_use]
    pub fn first(&self) -> Option<&Candle> {
        self.candles.front()
    }

    /// The newest retained candle.
    #[must_use]
    pub fn last(&self) -> Option<&Candle> {
        self.candles.back()
    }

    /// The candle at a retained position, where 0 is the oldest.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Candle> {
        self.candles.get(index)
    }

    /// Iterates over the retained candles from oldest to newest.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Candle> + ExactSizeIterator {
        self.candles.iter()
    }
}

impl Default for CandleWindow {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{CandleWindow, DEFAULT_WINDOW, WindowError};
    use crate::candle::{CandleError, RawCandle};
    use crate::time::format_utc_timestamp;

    const BASE: i64 = 1_735_689_600;

    fn source_timestamp(seconds: i64) -> String {
        let rfc3339 = format_utc_timestamp(seconds);
        format!("{} {}+00:00", &rfc3339[..10], &rfc3339[11..19])
    }

    fn raw_candle<'a>(date: &'a str) -> RawCandle<'a> {
        RawCandle {
            date,
            open: "1.0",
            high: "2.0",
            low: "0.5",
            close: "1.5",
            volume: Some("10"),
        }
    }

    fn push_at(window: &mut CandleWindow, seconds: i64) -> Result<u64, WindowError> {
        let date = source_timestamp(seconds);
        window.push(raw_candle(&date))
    }

    fn build_window(count: u64) -> CandleWindow {
        let mut window = CandleWindow::new();
        for offset in 0..count {
            let seconds = BASE + i64::try_from(offset).expect("test count fits in i64") * 60;
            push_at(&mut window, seconds).expect("fixture candles are valid");
        }
        window
    }

    #[test]
    fn new_window_is_empty() {
        let window = CandleWindow::new();

        assert!(window.is_empty());
        assert_eq!(window.len(), 0);
        assert!(window.first().is_none());
        assert!(window.last().is_none());
        assert!(window.get(0).is_none());
        assert_eq!(window.iter().count(), 0);
        assert_eq!(window, CandleWindow::default());
    }

    #[test]
    fn accepts_a_single_candle_and_exposes_every_field() {
        let mut window = CandleWindow::new();
        let date = source_timestamp(BASE);
        let raw = RawCandle {
            date: &date,
            open: "1.25",
            high: "2.5",
            low: "1.0",
            close: "2.0",
            volume: None,
        };

        let series_index = window.push(raw).expect("the candle is valid");
        let candle = window.get(0).expect("the candle is retained");

        assert_eq!(series_index, 0);
        assert_eq!(window.len(), 1);
        assert_eq!(candle.series_index(), 0);
        assert_eq!(candle.timestamp(), BASE);
        assert_eq!(candle.open(), 1.25);
        assert_eq!(candle.high(), 2.5);
        assert_eq!(candle.low(), 1.0);
        assert_eq!(candle.close(), 2.0);
        assert_eq!(candle.volume(), None);
        assert_eq!(window.first(), Some(candle));
        assert_eq!(window.last(), Some(candle));

        let date = source_timestamp(BASE + 60);
        let raw = RawCandle {
            volume: Some("0"),
            ..raw_candle(&date)
        };
        window.push(raw).expect("zero volume is valid");

        assert_eq!(
            window.last().expect("the candle is retained").volume(),
            Some(0.0)
        );
    }

    #[test]
    fn retains_999_candles() {
        let window = build_window(999);

        assert_eq!(window.len(), 999);
        assert_eq!(window.first().expect("not empty").series_index(), 0);
        assert_eq!(window.last().expect("not empty").series_index(), 998);
    }

    #[test]
    fn retains_1000_candles() {
        let window = build_window(1_000);

        assert_eq!(window.len(), 1_000);
        assert_eq!(window.first().expect("not empty").series_index(), 0);
        assert_eq!(window.last().expect("not empty").series_index(), 999);
    }

    #[test]
    fn evicts_the_oldest_candle_on_1001() {
        let window = build_window(1_001);

        assert_eq!(window.len(), DEFAULT_WINDOW);
        assert_eq!(window.first().expect("not empty").series_index(), 1);
        assert_eq!(window.last().expect("not empty").series_index(), 1_000);
    }

    #[test]
    fn retains_the_last_1000_of_2500_candles() {
        let window = build_window(2_500);

        assert_eq!(window.len(), DEFAULT_WINDOW);
        assert_eq!(window.first().expect("not empty").series_index(), 1_500);
        assert_eq!(window.last().expect("not empty").series_index(), 2_499);
    }

    #[test]
    fn never_exceeds_the_bound() {
        let mut window = CandleWindow::new();

        for offset in 0..2_500_u64 {
            let seconds = BASE + i64::try_from(offset).expect("test count fits in i64") * 60;
            push_at(&mut window, seconds).expect("fixture candles are valid");
            assert!(window.len() <= DEFAULT_WINDOW);
        }

        assert_eq!(window.len(), DEFAULT_WINDOW);
    }

    #[test]
    fn series_index_survives_eviction() {
        let mut window = build_window(1_000);
        assert_eq!(window.last().expect("not empty").series_index(), 999);

        let series_index = push_at(&mut window, BASE + 1_000 * 60).expect("the candle is valid");

        assert_eq!(series_index, 1_000);
        assert_eq!(window.first().expect("not empty").series_index(), 1);
        assert_eq!(window.last().expect("not empty").series_index(), 1_000);
    }

    #[test]
    fn get_uses_the_retained_position() {
        let window = build_window(1_001);

        assert_eq!(window.get(0).expect("retained").series_index(), 1);
        assert_eq!(window.get(999).expect("retained").series_index(), 1_000);
        assert!(window.get(1_000).is_none());
    }

    #[test]
    fn iterates_from_oldest_to_newest() {
        let window = build_window(3);
        let indexes: Vec<u64> = window.iter().map(|candle| candle.series_index()).collect();

        assert_eq!(indexes, vec![0, 1, 2]);
    }

    #[test]
    fn rejects_a_duplicate_timestamp_without_consuming_an_index() {
        let mut window = CandleWindow::new();
        push_at(&mut window, BASE).expect("the first candle is valid");

        let result = push_at(&mut window, BASE);

        assert_eq!(
            result,
            Err(WindowError::DuplicateTimestamp { timestamp: BASE })
        );
        assert_eq!(window.len(), 1);
        assert_eq!(
            push_at(&mut window, BASE + 60).expect("the next candle is valid"),
            1
        );
        assert_eq!(window.len(), 2);
    }

    #[test]
    fn rejects_an_out_of_order_timestamp_without_consuming_an_index() {
        let mut window = CandleWindow::new();
        push_at(&mut window, BASE + 60).expect("the first candle is valid");

        let result = push_at(&mut window, BASE);

        assert_eq!(
            result,
            Err(WindowError::OutOfOrderTimestamp {
                previous: BASE + 60,
                current: BASE,
            })
        );
        assert_eq!(window.len(), 1);
        assert_eq!(
            push_at(&mut window, BASE + 120).expect("the next candle is valid"),
            1
        );
        assert_eq!(window.len(), 2);
    }

    #[test]
    fn rejects_an_invalid_candle_without_consuming_an_index() {
        let mut window = CandleWindow::new();
        let date = source_timestamp(BASE);
        let raw = RawCandle {
            high: "0.5",
            low: "1.0",
            ..raw_candle(&date)
        };

        let result = window.push(raw);

        assert_eq!(result, Err(WindowError::Candle(CandleError::HighBelowLow)));
        assert!(window.is_empty());
        assert_eq!(
            push_at(&mut window, BASE).expect("the next candle is valid"),
            0
        );
    }

    #[test]
    fn rejects_an_out_of_order_candle_after_the_previous_candle_was_evicted() {
        let mut window = build_window(1_000);
        let previous = BASE + 999 * 60;
        push_at(&mut window, BASE + 1_000 * 60).expect("the candle is valid");

        let result = push_at(&mut window, previous);

        assert_eq!(
            result,
            Err(WindowError::OutOfOrderTimestamp {
                previous: BASE + 1_000 * 60,
                current: previous,
            })
        );
        assert_eq!(window.len(), DEFAULT_WINDOW);
    }

    #[test]
    fn appending_later_candles_does_not_change_retained_values() {
        let mut window = build_window(1_000);
        let middle = *window.get(500).expect("retained");
        let last = *window.last().expect("not empty");

        push_at(&mut window, BASE + 1_000 * 60).expect("the candle is valid");

        assert_eq!(*window.get(499).expect("retained"), middle);
        assert_eq!(*window.get(998).expect("retained"), last);
        assert_eq!(window.len(), DEFAULT_WINDOW);
    }

    #[test]
    fn repeated_runs_are_deterministic() {
        assert_eq!(build_window(2_500), build_window(2_500));
    }
}
