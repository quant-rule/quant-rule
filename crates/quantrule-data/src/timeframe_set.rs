//! Multi-timeframe construction from a single M1 stream.
//!
//! The set owns one bounded series per supported timeframe. M1 candles enter
//! through [`TimeframeSet::push`] or [`TimeframeSet::push_values`]; the engine
//! builds M5, M15, and H1 from them. A derived bucket is published only after
//! every expected M1 candle is present, so a partial bucket can never become a
//! candle. Gaps are recorded between consecutive candles of each series.
//!
//! This module performs no I/O and reads no clock. The shell supplies the M1
//! candles and renders the resulting series.

use std::fmt;

use crate::bucket::Bucket;
use crate::{
    Candle, CandleValues, CandleWindow, RawCandle, Timeframe, WindowError, format_utc_timestamp,
    parse_utc_timestamp,
};

/// A missing run between two consecutive candles in one series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// Timestamp of the candle before the gap, as UTC Unix seconds.
    pub after: i64,
    /// Timestamp of the candle after the gap, as UTC Unix seconds.
    pub before: i64,
    /// Number of expected candles missing between the two timestamps.
    pub missing: u64,
}

/// A higher-timeframe bucket that was dropped because it was incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialBucket {
    /// The bucket's UTC start timestamp as Unix seconds.
    pub bucket_start: i64,
    /// The number of M1 candles actually present.
    pub present: u32,
    /// The number of M1 candles a complete bucket requires.
    pub expected: u32,
}

/// One first-class timeframe series.
///
/// The series owns a bounded [`CandleWindow`] and the gap and partial-bucket
/// records produced while it was built. Candles use the same [`Candle`] shape
/// as every other timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeSeries {
    timeframe: Timeframe,
    window: CandleWindow,
    gaps: Vec<Gap>,
    partial_buckets: Vec<PartialBucket>,
}

impl TimeframeSeries {
    fn new(timeframe: Timeframe) -> Self {
        Self {
            timeframe,
            window: CandleWindow::new(),
            gaps: Vec::new(),
            partial_buckets: Vec::new(),
        }
    }

    /// The timeframe this series represents.
    #[must_use]
    pub const fn timeframe(&self) -> Timeframe {
        self.timeframe
    }

    /// The number of retained candles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Whether the series holds no candles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// The oldest retained candle.
    #[must_use]
    pub fn first(&self) -> Option<&Candle> {
        self.window.first()
    }

    /// The newest retained candle.
    #[must_use]
    pub fn last(&self) -> Option<&Candle> {
        self.window.last()
    }

    /// The candle at a retained position, where 0 is the oldest.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Candle> {
        self.window.get(index)
    }

    /// Iterates over the retained candles from oldest to newest.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Candle> + ExactSizeIterator {
        self.window.iter()
    }

    /// The gaps recorded between consecutive candles, oldest first.
    #[must_use]
    pub fn gaps(&self) -> &[Gap] {
        &self.gaps
    }

    /// The incomplete buckets dropped from this series, oldest first.
    #[must_use]
    pub fn partial_buckets(&self) -> &[PartialBucket] {
        &self.partial_buckets
    }

    fn push_raw(&mut self, raw: RawCandle<'_>) -> Result<Candle, WindowError> {
        let previous = self.last().map(Candle::timestamp);
        let candle = self.window.push_and_return(raw)?;
        self.record_gap(previous, candle.timestamp());
        Ok(candle)
    }

    fn push_values(&mut self, values: CandleValues) -> Result<Candle, WindowError> {
        let previous = self.last().map(Candle::timestamp);
        let candle = self.window.push_values_and_return(values)?;
        self.record_gap(previous, candle.timestamp());
        Ok(candle)
    }

    fn push_aggregated(&mut self, values: CandleValues) -> Result<(), WindowError> {
        let previous = self.last().map(Candle::timestamp);
        let candle = self.window.push_values_and_return(values)?;
        self.record_gap(previous, candle.timestamp());
        Ok(())
    }

    fn record_partial(&mut self, bucket: &Bucket) {
        self.partial_buckets.push(PartialBucket {
            bucket_start: bucket.start(),
            present: bucket.present(),
            expected: bucket.expected(),
        });
    }

    fn record_gap(&mut self, previous: Option<i64>, current: i64) {
        let Some(previous) = previous else {
            return;
        };
        let step = i64::from(self.timeframe.seconds());
        let delta = current - previous;
        if delta > step {
            let missing = u64::try_from(delta / step - 1).unwrap_or(u64::MAX);
            self.gaps.push(Gap {
                after: previous,
                before: current,
                missing,
            });
        }
    }
}

/// A set of first-class M1, M5, M15, and H1 series.
///
/// Feed closed M1 candles in strictly increasing timestamp order. The set
/// validates and retains M1, derives M5/M15/H1, and bounds every series at
/// [`crate::DEFAULT_WINDOW`] candles. Call [`TimeframeSet::finish`] at the end
/// of the input so a complete trailing bucket is published and an incomplete
/// one is recorded as partial.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeSet {
    series: [TimeframeSeries; 4],
    buckets: [Option<Bucket>; 3],
    finished: bool,
}

impl TimeframeSet {
    /// Creates an empty set with one series per supported timeframe.
    #[must_use]
    pub fn new() -> Self {
        Self {
            series: Timeframe::ALL.map(TimeframeSeries::new),
            buckets: [None; 3],
            finished: false,
        }
    }

    /// Parses and accepts one M1 source candle.
    ///
    /// On success, returns the stable series index assigned in the M1 series.
    /// A rejected candle does not change any series or aggregation state.
    ///
    /// # Errors
    ///
    /// Returns [`TimeframeError::Window`] for an invalid, duplicate, or
    /// out-of-order candle; [`TimeframeError::MisalignedM1Timestamp`] when the
    /// timestamp is not on a whole-minute UTC boundary; or
    /// [`TimeframeError::Finished`] after [`TimeframeSet::finish`].
    pub fn push(&mut self, raw: RawCandle<'_>) -> Result<u64, TimeframeError> {
        self.ensure_active()?;
        if let Ok(timestamp) = parse_utc_timestamp(raw.date) {
            check_m1_alignment(timestamp)?;
        }
        let candle = self.series_mut(Timeframe::M1).push_raw(raw)?;
        self.aggregate(candle)?;
        Ok(candle.series_index())
    }

    /// Accepts one M1 candle whose values are already parsed.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`TimeframeSet::push`].
    pub fn push_values(&mut self, values: CandleValues) -> Result<u64, TimeframeError> {
        self.ensure_active()?;
        check_m1_alignment(values.timestamp)?;
        let candle = self.series_mut(Timeframe::M1).push_values(values)?;
        self.aggregate(candle)?;
        Ok(candle.series_index())
    }

    /// Finishes the input.
    ///
    /// A complete trailing bucket is published; an incomplete trailing bucket
    /// is recorded as partial and dropped. Calling `finish` again is a no-op.
    /// Further pushes are rejected with [`TimeframeError::Finished`].
    ///
    /// # Errors
    ///
    /// Returns [`TimeframeError::Window`] if a complete bucket cannot be
    /// accepted by its series window.
    pub fn finish(&mut self) -> Result<(), TimeframeError> {
        if self.finished {
            return Ok(());
        }

        for (index, timeframe) in Timeframe::DERIVED.iter().enumerate() {
            if let Some(bucket) = self.buckets[index].take() {
                self.finalize(*timeframe, bucket)?;
            }
        }
        self.finished = true;
        Ok(())
    }

    /// Whether [`TimeframeSet::finish`] has been called.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    /// The series for one timeframe.
    ///
    /// The returned series is always present because every [`Timeframe`] value
    /// has a corresponding series.
    #[must_use]
    pub fn series(&self, timeframe: Timeframe) -> &TimeframeSeries {
        &self.series[series_position(timeframe)]
    }

    /// Iterates over M1, M5, M15, and H1 in that fixed order.
    pub fn series_iter(&self) -> impl Iterator<Item = &TimeframeSeries> {
        self.series.iter()
    }

    fn series_mut(&mut self, timeframe: Timeframe) -> &mut TimeframeSeries {
        &mut self.series[series_position(timeframe)]
    }

    fn ensure_active(&self) -> Result<(), TimeframeError> {
        if self.finished {
            Err(TimeframeError::Finished)
        } else {
            Ok(())
        }
    }

    fn aggregate(&mut self, candle: Candle) -> Result<(), TimeframeError> {
        let values = CandleValues {
            timestamp: candle.timestamp(),
            open: candle.open(),
            high: candle.high(),
            low: candle.low(),
            close: candle.close(),
            volume: candle.volume(),
        };

        for (index, timeframe) in Timeframe::DERIVED.iter().enumerate() {
            let bucket_start = timeframe.bucket_start(values.timestamp);
            let transition =
                self.buckets[index].is_some_and(|bucket| bucket.start() != bucket_start);

            if transition && let Some(bucket) = self.buckets[index].take() {
                self.finalize(*timeframe, bucket)?;
            }

            match &mut self.buckets[index] {
                Some(bucket) => bucket.push(values),
                None => {
                    self.buckets[index] = Some(Bucket::new(*timeframe, values));
                }
            }
        }

        Ok(())
    }

    fn finalize(&mut self, timeframe: Timeframe, bucket: Bucket) -> Result<(), TimeframeError> {
        if bucket.is_complete() {
            self.series_mut(timeframe)
                .push_aggregated(bucket.into_values())?;
        } else {
            self.series_mut(timeframe).record_partial(&bucket);
        }
        Ok(())
    }
}

impl Default for TimeframeSet {
    fn default() -> Self {
        Self::new()
    }
}

/// A multi-timeframe operation could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeframeError {
    /// A candle was rejected by its series window.
    Window(WindowError),
    /// The M1 timestamp is not on a whole-minute UTC boundary.
    MisalignedM1Timestamp {
        /// The rejected timestamp as UTC Unix seconds.
        timestamp: i64,
    },
    /// The set was already finished.
    Finished,
}

impl From<WindowError> for TimeframeError {
    fn from(source: WindowError) -> Self {
        Self::Window(source)
    }
}

impl fmt::Display for TimeframeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Window(source) => write!(formatter, "{source}"),
            Self::MisalignedM1Timestamp { timestamp } => write!(
                formatter,
                "M1 timestamp {} is not aligned to a 60-second boundary",
                format_utc_timestamp(*timestamp)
            ),
            Self::Finished => formatter.write_str("the timeframe set is already finished"),
        }
    }
}

impl std::error::Error for TimeframeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Window(source) => Some(source),
            Self::MisalignedM1Timestamp { .. } | Self::Finished => None,
        }
    }
}

fn check_m1_alignment(timestamp: i64) -> Result<(), TimeframeError> {
    let step = i64::from(Timeframe::M1.seconds());
    if timestamp.rem_euclid(step) == 0 {
        Ok(())
    } else {
        Err(TimeframeError::MisalignedM1Timestamp { timestamp })
    }
}

fn series_position(timeframe: Timeframe) -> usize {
    Timeframe::ALL
        .iter()
        .position(|candidate| *candidate == timeframe)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{PartialBucket, TimeframeError, TimeframeSet};
    use crate::{CandleValues, DEFAULT_WINDOW, RawCandle, Timeframe};

    const BASE: i64 = 1_735_689_600;

    fn minute(offset: i64) -> i64 {
        BASE + offset * 60
    }

    fn values_at(seconds: i64) -> CandleValues {
        CandleValues {
            timestamp: seconds,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: Some(10.0),
        }
    }

    fn values_with(
        seconds: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<f64>,
    ) -> CandleValues {
        CandleValues {
            timestamp: seconds,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    fn push(set: &mut TimeframeSet, offset: i64) {
        set.push_values(values_at(minute(offset)))
            .expect("fixture value is valid");
    }

    fn push_range(set: &mut TimeframeSet, start: i64, end: i64) {
        for offset in start..end {
            push(set, offset);
        }
    }

    fn build_minutes(minutes: i64) -> TimeframeSet {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, minutes);
        set.finish().expect("fixture buckets are valid");
        set
    }

    #[test]
    fn a_new_set_has_four_empty_series_in_fixed_order() {
        let set = TimeframeSet::new();
        let series: Vec<Timeframe> = set.series_iter().map(|series| series.timeframe()).collect();

        assert_eq!(
            series,
            vec![Timeframe::M1, Timeframe::M5, Timeframe::M15, Timeframe::H1]
        );
        for timeframe in Timeframe::ALL {
            let series = set.series(timeframe);
            assert!(series.is_empty());
            assert_eq!(series.len(), 0);
            assert!(series.first().is_none());
            assert!(series.last().is_none());
            assert!(series.get(0).is_none());
            assert!(series.gaps().is_empty());
            assert!(series.partial_buckets().is_empty());
        }
        assert!(!set.is_finished());
    }

    #[test]
    fn empty_and_finished_sets_are_stable() {
        let mut set = TimeframeSet::new();

        set.finish().expect("an empty set finishes");
        set.finish().expect("finish is idempotent");

        let mut expected = TimeframeSet::new();
        expected.finish().expect("an empty set finishes");

        assert!(set.is_finished());
        assert_eq!(set, expected);
    }

    #[test]
    fn a_single_candle_stays_on_m1_and_is_partial_everywhere_else() {
        let mut set = TimeframeSet::new();
        push(&mut set, 0);
        set.finish().expect("the partial bucket is recordable");

        assert_eq!(set.series(Timeframe::M1).len(), 1);
        assert_eq!(
            set.series(Timeframe::M1)
                .last()
                .expect("retained")
                .timestamp(),
            BASE
        );
        assert!(set.series(Timeframe::M5).is_empty());
        assert!(set.series(Timeframe::M15).is_empty());
        assert!(set.series(Timeframe::H1).is_empty());

        assert_eq!(
            set.series(Timeframe::M5).partial_buckets(),
            &[PartialBucket {
                bucket_start: BASE,
                present: 1,
                expected: 5,
            }]
        );
        assert_eq!(
            set.series(Timeframe::M15).partial_buckets(),
            &[PartialBucket {
                bucket_start: BASE,
                present: 1,
                expected: 15,
            }]
        );
        assert_eq!(
            set.series(Timeframe::H1).partial_buckets(),
            &[PartialBucket {
                bucket_start: BASE,
                present: 1,
                expected: 60,
            }]
        );
    }

    #[test]
    fn a_complete_m5_bucket_is_aggregated_with_exact_ohlcv() {
        let mut set = TimeframeSet::new();
        set.push_values(values_with(minute(0), 1.0, 2.0, 0.5, 1.5, Some(1.0)))
            .expect("valid");
        set.push_values(values_with(minute(1), 1.5, 2.5, 0.75, 2.0, Some(2.0)))
            .expect("valid");
        set.push_values(values_with(minute(2), 2.0, 3.0, 1.0, 2.5, Some(3.0)))
            .expect("valid");
        set.push_values(values_with(minute(3), 2.5, 2.75, 0.25, 1.0, Some(4.0)))
            .expect("valid");
        set.push_values(values_with(minute(4), 1.0, 1.5, 0.5, 0.75, Some(5.0)))
            .expect("valid");
        set.finish().expect("the M5 bucket is complete");

        let series = set.series(Timeframe::M5);
        let candle = series.first().expect("the M5 bucket was published");
        assert_eq!(series.len(), 1);
        assert_eq!(candle.series_index(), 0);
        assert_eq!(candle.timestamp(), BASE);
        assert_eq!(candle.open(), 1.0);
        assert_eq!(candle.high(), 3.0);
        assert_eq!(candle.low(), 0.25);
        assert_eq!(candle.close(), 0.75);
        assert_eq!(candle.volume(), Some(15.0));
        assert!(series.partial_buckets().is_empty());
        assert!(set.series(Timeframe::M15).is_empty());
        assert!(set.series(Timeframe::H1).is_empty());
    }

    #[test]
    fn a_partial_bucket_is_never_published() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 4);
        push(&mut set, 5);
        set.finish().expect("partial buckets are recordable");

        let series = set.series(Timeframe::M5);
        assert!(series.is_empty());
        assert_eq!(
            series.partial_buckets(),
            &[
                PartialBucket {
                    bucket_start: BASE,
                    present: 4,
                    expected: 5,
                },
                PartialBucket {
                    bucket_start: BASE + 300,
                    present: 1,
                    expected: 5,
                }
            ]
        );
    }

    #[test]
    fn leading_and_trailing_partials_are_counted_but_complete_buckets_are_kept() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 2, 5);
        push_range(&mut set, 5, 10);
        set.finish().expect("the leading partial is recordable");

        let series = set.series(Timeframe::M5);
        assert_eq!(series.len(), 1);
        assert_eq!(series.first().expect("retained").timestamp(), BASE + 300);
        assert_eq!(
            series.partial_buckets(),
            &[PartialBucket {
                bucket_start: BASE,
                present: 3,
                expected: 5,
            }]
        );
    }

    #[test]
    fn a_missing_m1_run_is_a_gap_in_the_series_that_contains_it() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 5);
        push_range(&mut set, 10, 15);
        set.finish().expect("all buckets are complete");

        let m1 = set.series(Timeframe::M1);
        assert_eq!(m1.gaps().len(), 1);
        assert_eq!(m1.gaps()[0].after, minute(4));
        assert_eq!(m1.gaps()[0].before, minute(10));
        assert_eq!(m1.gaps()[0].missing, 5);

        let m5 = set.series(Timeframe::M5);
        assert_eq!(m5.len(), 2);
        assert_eq!(m5.gaps().len(), 1);
        assert_eq!(m5.gaps()[0].after, BASE);
        assert_eq!(m5.gaps()[0].before, BASE + 600);
        assert_eq!(m5.gaps()[0].missing, 1);
        assert!(m5.partial_buckets().is_empty());
    }

    #[test]
    fn a_dropped_partial_bucket_appears_as_a_gap_in_the_higher_series() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 5);
        push_range(&mut set, 5, 9);
        push_range(&mut set, 10, 15);
        set.finish().expect("all buckets are complete");

        let m5 = set.series(Timeframe::M5);
        assert_eq!(m5.len(), 2);
        assert_eq!(m5.first().expect("retained").timestamp(), BASE);
        assert_eq!(m5.last().expect("retained").timestamp(), BASE + 600);
        assert_eq!(
            m5.partial_buckets(),
            &[PartialBucket {
                bucket_start: BASE + 300,
                present: 4,
                expected: 5,
            }]
        );
        assert_eq!(m5.gaps().len(), 1);
        assert_eq!(m5.gaps()[0].after, BASE);
        assert_eq!(m5.gaps()[0].before, BASE + 600);
        assert_eq!(m5.gaps()[0].missing, 1);
    }

    #[test]
    fn complete_m15_and_h1_buckets_come_from_the_same_m1_stream() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 60);
        push(&mut set, 60);
        set.finish().expect("the complete hour is publishable");

        assert_eq!(set.series(Timeframe::M15).len(), 4);
        assert_eq!(set.series(Timeframe::H1).len(), 1);

        let h1 = set.series(Timeframe::H1).first().expect("published H1");
        assert_eq!(h1.timestamp(), BASE);
        assert_eq!(h1.open(), 1.0);
        assert_eq!(h1.high(), 2.0);
        assert_eq!(h1.low(), 0.5);
        assert_eq!(h1.close(), 1.5);
        assert_eq!(h1.volume(), Some(600.0));

        let m15 = set.series(Timeframe::M15);
        assert_eq!(m15.first().expect("retained").timestamp(), BASE);
        assert_eq!(m15.last().expect("retained").timestamp(), BASE + 2_700);
        assert_eq!(m15.gaps().len(), 0);
    }

    #[test]
    fn an_m15_gap_is_recorded_at_m15_resolution() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 15);
        push_range(&mut set, 30, 45);
        set.finish().expect("the M15 buckets are complete");

        let m15 = set.series(Timeframe::M15);
        assert_eq!(m15.len(), 2);
        assert_eq!(m15.gaps().len(), 1);
        assert_eq!(m15.gaps()[0].after, BASE);
        assert_eq!(m15.gaps()[0].before, BASE + 1_800);
        assert_eq!(m15.gaps()[0].missing, 1);
    }

    #[test]
    fn an_h1_gap_is_recorded_at_h1_resolution() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 60);
        push_range(&mut set, 120, 180);
        set.finish().expect("the H1 buckets are complete");

        let h1 = set.series(Timeframe::H1);
        assert_eq!(h1.len(), 2);
        assert_eq!(h1.gaps().len(), 1);
        assert_eq!(h1.gaps()[0].after, BASE);
        assert_eq!(h1.gaps()[0].before, BASE + 7_200);
        assert_eq!(h1.gaps()[0].missing, 1);
    }

    #[test]
    fn volume_is_summed_only_when_every_source_volume_is_known() {
        let mut set = TimeframeSet::new();
        for offset in 0..5 {
            set.push_values(values_with(minute(offset), 1.0, 2.0, 0.5, 1.5, Some(2.0)))
                .expect("valid");
        }
        set.finish().expect("complete");
        assert_eq!(
            set.series(Timeframe::M5)
                .first()
                .expect("published")
                .volume(),
            Some(10.0)
        );

        let mut set = TimeframeSet::new();
        for offset in 0..5 {
            let volume = if offset == 2 { None } else { Some(2.0) };
            set.push_values(values_with(minute(offset), 1.0, 2.0, 0.5, 1.5, volume))
                .expect("valid");
        }
        set.finish().expect("complete");
        assert_eq!(
            set.series(Timeframe::M5)
                .first()
                .expect("published")
                .volume(),
            None
        );
    }

    #[test]
    fn a_real_zero_volume_survives_aggregation() {
        let mut set = TimeframeSet::new();
        for offset in 0..5 {
            set.push_values(values_with(minute(offset), 1.0, 1.0, 1.0, 1.0, Some(0.0)))
                .expect("valid");
        }
        set.finish().expect("complete");

        let candle = set.series(Timeframe::M5).first().expect("published");
        assert_eq!(candle.high(), 1.0);
        assert_eq!(candle.low(), 1.0);
        assert_eq!(candle.volume(), Some(0.0));
    }

    #[test]
    fn misaligned_m1_timestamps_are_rejected_without_changing_the_set() {
        let mut set = TimeframeSet::new();
        let misaligned = minute(0) + 30;

        assert_eq!(
            set.push_values(values_at(misaligned)),
            Err(TimeframeError::MisalignedM1Timestamp {
                timestamp: misaligned
            })
        );
        assert!(set.series(Timeframe::M1).is_empty());
        assert_eq!(
            set.push_values(values_at(minute(0)))
                .expect("the aligned value is accepted"),
            0
        );
    }

    #[test]
    fn raw_misaligned_timestamps_are_rejected_before_acceptance() {
        let mut set = TimeframeSet::new();
        let result = set.push(RawCandle {
            date: "2025-01-01 00:00:30+00:00",
            open: "1",
            high: "2",
            low: "0.5",
            close: "1.5",
            volume: Some("10"),
        });

        assert_eq!(
            result,
            Err(TimeframeError::MisalignedM1Timestamp {
                timestamp: BASE + 30
            })
        );
        assert!(set.series(Timeframe::M1).is_empty());
    }

    #[test]
    fn duplicate_and_out_of_order_values_do_not_disturb_the_buckets() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 5);

        assert!(matches!(
            set.push_values(values_at(minute(0))),
            Err(TimeframeError::Window(
                crate::WindowError::OutOfOrderTimestamp { .. }
            ))
        ));
        assert!(matches!(
            set.push_values(values_at(minute(4))),
            Err(TimeframeError::Window(
                crate::WindowError::DuplicateTimestamp { .. }
            ))
        ));

        push(&mut set, 5);
        set.finish().expect("the first bucket is complete");
        assert_eq!(set.series(Timeframe::M5).len(), 1);
    }

    #[test]
    fn finish_is_idempotent_and_rejects_later_pushes() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 5);
        set.finish().expect("the bucket is complete");
        set.finish().expect("finish is idempotent");

        assert_eq!(set.series(Timeframe::M5).len(), 1);
        assert_eq!(
            set.push_values(values_at(minute(5))),
            Err(TimeframeError::Finished)
        );
        assert_eq!(set.series(Timeframe::M1).len(), 5);
        assert!(set.is_finished());
    }

    #[test]
    fn every_series_retains_at_most_one_thousand_candles() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 5_005);
        set.finish().expect("all buckets are valid");

        for timeframe in Timeframe::ALL {
            assert!(
                set.series(timeframe).len() <= DEFAULT_WINDOW,
                "{} exceeded the window",
                timeframe.code()
            );
        }
        assert_eq!(set.series(Timeframe::M5).len(), DEFAULT_WINDOW);
        assert_eq!(
            set.series(Timeframe::M5)
                .first()
                .expect("retained")
                .series_index(),
            1
        );
        assert_eq!(
            set.series(Timeframe::M5)
                .last()
                .expect("retained")
                .series_index(),
            1_000
        );
    }

    #[test]
    fn each_timeframe_owns_its_series_index() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 6);
        set.finish().expect("the first M5 bucket is complete");

        let m5 = set.series(Timeframe::M5);
        assert_eq!(m5.first().expect("retained").series_index(), 0);
        assert_eq!(
            set.series(Timeframe::M1)
                .first()
                .expect("retained")
                .series_index(),
            0
        );
    }

    #[test]
    fn appending_later_candles_does_not_change_earlier_output() {
        let mut set = TimeframeSet::new();
        push_range(&mut set, 0, 15);
        push(&mut set, 15);

        let earlier: Vec<_> = set.series(Timeframe::M5).iter().copied().collect();
        let earlier_gaps = set.series(Timeframe::M5).gaps().to_vec();
        let earlier_partials = set.series(Timeframe::M5).partial_buckets().to_vec();

        push_range(&mut set, 16, 25);
        set.finish().expect("all buckets are valid");

        for (index, candle) in earlier.iter().enumerate() {
            assert_eq!(
                set.series(Timeframe::M5).get(index),
                Some(candle),
                "candle {index} changed"
            );
        }
        assert_eq!(set.series(Timeframe::M5).gaps(), earlier_gaps);
        assert_eq!(
            set.series(Timeframe::M5).partial_buckets(),
            earlier_partials
        );
    }

    #[test]
    fn repeated_runs_are_identical() {
        let first = build_minutes(125);
        let second = build_minutes(125);

        assert_eq!(first, second);
        assert_eq!(
            first.series_iter().collect::<Vec<_>>(),
            second.series_iter().collect::<Vec<_>>()
        );
    }

    fn build_in_chunks(chunk_size: i64) -> TimeframeSet {
        let mut set = TimeframeSet::new();
        let mut offset = 0;
        while offset < 125 {
            let end = (offset + chunk_size).min(125);
            push_range(&mut set, offset, end);
            offset = end;
        }
        set.finish().expect("fixture buckets are valid");
        set
    }

    #[test]
    fn a_different_split_of_the_input_produces_the_same_set() {
        assert_eq!(build_in_chunks(1), build_in_chunks(7));
        assert_eq!(build_in_chunks(7), build_in_chunks(125));
    }
}
