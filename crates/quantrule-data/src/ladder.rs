//! Streaming counts for a timeframe ladder.
//!
//! Inspection needs to know how many bars each timeframe holds without keeping
//! the series in memory. This module folds the same buckets [`crate::TimeframeSet`]
//! publishes, so a count and a published candle cannot disagree.

use crate::bucket::Bucket;
use crate::time::{TimestampError, is_supported_timestamp};
use crate::timeframe_set::{check_m1_alignment, series_position};
use crate::{CandleError, CandleValues, Timeframe, TimeframeError, WindowError};

/// Counts and coverage for one timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeframeCoverage {
    /// Which timeframe this covers.
    pub timeframe: Timeframe,
    /// Complete bars.
    pub bars: u64,
    /// Trailing buckets that were missing at least one M1 candle.
    pub partial_buckets: u64,
    /// First complete bar, Unix seconds UTC.
    pub first_bar: Option<i64>,
    /// Last complete bar, Unix seconds UTC.
    pub last_bar: Option<i64>,
}

impl TimeframeCoverage {
    fn new(timeframe: Timeframe) -> Self {
        Self {
            timeframe,
            bars: 0,
            partial_buckets: 0,
            first_bar: None,
            last_bar: None,
        }
    }
}

/// A fold over M1 candles that counts a ladder without retaining it.
#[derive(Debug, Clone, PartialEq)]
pub struct LadderSummary {
    candles: u64,
    first_candle: Option<i64>,
    last_candle: Option<i64>,
    series: [TimeframeCoverage; 5],
    buckets: [Option<Bucket>; 4],
    finished: bool,
}

impl LadderSummary {
    /// Creates an empty summary.
    #[must_use]
    pub fn new() -> Self {
        Self {
            candles: 0,
            first_candle: None,
            last_candle: None,
            series: Timeframe::ALL.map(TimeframeCoverage::new),
            buckets: [None; 4],
            finished: false,
        }
    }

    /// Accepts one M1 candle.
    ///
    /// # Errors
    ///
    /// Returns [`TimeframeError::Window`] for an unsupported, duplicate, or
    /// out-of-order timestamp, [`TimeframeError::MisalignedM1Timestamp`] when
    /// the row is not on a whole-minute boundary, and
    /// [`TimeframeError::VolumeOverflow`] when a bucket sum is no longer finite.
    pub fn push(&mut self, values: CandleValues) -> Result<(), TimeframeError> {
        if self.finished {
            return Err(TimeframeError::Finished);
        }
        if !is_supported_timestamp(values.timestamp) {
            return Err(TimeframeError::Window(WindowError::Candle(
                CandleError::Timestamp(TimestampError::UnsupportedYear),
            )));
        }
        check_m1_alignment(values.timestamp)?;
        if let Some(previous) = self.last_candle {
            if values.timestamp == previous {
                return Err(WindowError::DuplicateTimestamp {
                    timestamp: previous,
                }
                .into());
            }
            if values.timestamp < previous {
                return Err(WindowError::OutOfOrderTimestamp {
                    previous,
                    current: values.timestamp,
                }
                .into());
            }
        }

        self.candles += 1;
        self.first_candle.get_or_insert(values.timestamp);
        self.last_candle = Some(values.timestamp);
        let m1 = &mut self.series[series_position(Timeframe::M1)];
        m1.bars += 1;
        m1.first_bar.get_or_insert(values.timestamp);
        m1.last_bar = Some(values.timestamp);

        for (index, timeframe) in Timeframe::DERIVED.iter().enumerate() {
            let bucket_start = timeframe.bucket_start(values.timestamp);
            let transition =
                self.buckets[index].is_some_and(|bucket| bucket.start() != bucket_start);
            if transition && let Some(bucket) = self.buckets[index].take() {
                self.record(*timeframe, bucket);
            }
            match &mut self.buckets[index] {
                Some(bucket) => bucket.push(values).map_err(|error| match error {
                    crate::bucket::BucketError::VolumeOverflow { bucket_start } => {
                        TimeframeError::VolumeOverflow { bucket_start }
                    }
                })?,
                None => self.buckets[index] = Some(Bucket::new(*timeframe, values)),
            }
        }
        Ok(())
    }

    /// Finalizes the trailing buckets. Calling it again is a no-op.
    pub fn finish(&mut self) {
        if self.finished {
            return;
        }
        for (index, timeframe) in Timeframe::DERIVED.iter().enumerate() {
            if let Some(bucket) = self.buckets[index].take() {
                self.record(*timeframe, bucket);
            }
        }
        self.finished = true;
    }

    /// Whether [`LadderSummary::finish`] has been called.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    /// Number of M1 candles accepted.
    #[must_use]
    pub const fn candles(&self) -> u64 {
        self.candles
    }

    /// First M1 candle, Unix seconds UTC.
    #[must_use]
    pub const fn first_candle(&self) -> Option<i64> {
        self.first_candle
    }

    /// Last M1 candle, Unix seconds UTC.
    #[must_use]
    pub const fn last_candle(&self) -> Option<i64> {
        self.last_candle
    }

    /// Counts and coverage for one timeframe.
    #[must_use]
    pub fn coverage(&self, timeframe: Timeframe) -> TimeframeCoverage {
        self.series[series_position(timeframe)]
    }

    /// Counts and coverage for every timeframe in ascending order.
    pub fn coverages(&self) -> impl Iterator<Item = TimeframeCoverage> + '_ {
        self.series.iter().copied()
    }

    fn record(&mut self, timeframe: Timeframe, bucket: Bucket) {
        let coverage = &mut self.series[series_position(timeframe)];
        if bucket.is_complete() {
            coverage.bars += 1;
            coverage.first_bar.get_or_insert(bucket.start());
            coverage.last_bar = Some(bucket.start());
        } else {
            coverage.partial_buckets += 1;
        }
    }
}

impl Default for LadderSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: i64 = 1_735_689_600;

    fn values(offset: i64) -> CandleValues {
        CandleValues {
            timestamp: BASE + offset * 60,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: Some(10.0),
        }
    }

    fn push_minutes(summary: &mut LadderSummary, minutes: i64) {
        for offset in 0..minutes {
            summary.push(values(offset)).expect("candle");
        }
    }

    #[test]
    fn a_five_minute_series_has_one_m5_bar() {
        let mut summary = LadderSummary::new();
        push_minutes(&mut summary, 5);
        summary.finish();

        assert_eq!(summary.candles(), 5);
        assert_eq!(summary.coverage(Timeframe::M1).bars, 5);
        assert_eq!(summary.coverage(Timeframe::M5).bars, 1);
        assert_eq!(summary.coverage(Timeframe::M15).bars, 0);
        assert_eq!(summary.coverage(Timeframe::H1).bars, 0);
        assert_eq!(summary.coverage(Timeframe::H4).bars, 0);
    }

    #[test]
    fn an_incomplete_trailing_bucket_is_partial() {
        let mut summary = LadderSummary::new();
        push_minutes(&mut summary, 2);
        summary.finish();

        let m5 = summary.coverage(Timeframe::M5);
        assert_eq!(m5.bars, 0);
        assert_eq!(m5.partial_buckets, 1);
        assert_eq!(m5.first_bar, None);
    }

    #[test]
    fn a_complete_bucket_records_its_first_and_last_bar() {
        let mut summary = LadderSummary::new();
        push_minutes(&mut summary, 10);
        summary.finish();

        let m5 = summary.coverage(Timeframe::M5);
        assert_eq!(m5.bars, 2);
        assert_eq!(m5.first_bar, Some(BASE));
        assert_eq!(m5.last_bar, Some(BASE + 5 * 60));
    }

    #[test]
    fn duplicate_and_out_of_order_rows_are_refused() {
        let mut summary = LadderSummary::new();
        summary.push(values(0)).expect("first");
        assert_eq!(
            summary.push(values(0)),
            Err(WindowError::DuplicateTimestamp { timestamp: BASE }.into())
        );
        summary.push(values(1)).expect("second");
        assert_eq!(
            summary.push(values(0)),
            Err(WindowError::OutOfOrderTimestamp {
                previous: BASE + 60,
                current: BASE
            }
            .into())
        );
    }

    #[test]
    fn a_misaligned_row_is_refused() {
        let mut summary = LadderSummary::new();
        let error = summary
            .push(CandleValues {
                timestamp: BASE + 30,
                ..values(0)
            })
            .expect_err("misaligned");
        assert_eq!(
            error,
            TimeframeError::MisalignedM1Timestamp {
                timestamp: BASE + 30
            }
        );
    }

    #[test]
    fn the_summary_matches_the_published_set() {
        let mut summary = LadderSummary::new();
        let mut set = crate::TimeframeSet::with_limit(100);
        for offset in 0..12 {
            summary.push(values(offset)).expect("summary");
            set.push_values(values(offset)).expect("set");
        }
        summary.finish();
        set.finish().expect("finish");

        for timeframe in Timeframe::ALL {
            let coverage = summary.coverage(timeframe);
            assert_eq!(
                coverage.bars,
                set.series(timeframe).len() as u64,
                "{} bars",
                timeframe.code()
            );
            assert_eq!(
                coverage.partial_buckets,
                set.series(timeframe).partial_buckets().len() as u64,
                "{} partials",
                timeframe.code()
            );
        }
    }

    #[test]
    fn the_same_rows_give_the_same_summary() {
        let mut first = LadderSummary::new();
        let mut second = LadderSummary::new();
        push_minutes(&mut first, 12);
        push_minutes(&mut second, 12);
        first.finish();
        second.finish();
        assert_eq!(first, second);
    }

    #[test]
    fn volume_overflow_is_reported() {
        let mut summary = LadderSummary::new();
        summary
            .push(CandleValues {
                volume: Some(1e308),
                ..values(0)
            })
            .expect("first");
        let error = summary
            .push(CandleValues {
                volume: Some(1e308),
                ..values(1)
            })
            .expect_err("overflow");
        assert_eq!(error, TimeframeError::VolumeOverflow { bucket_start: BASE });
    }
}
