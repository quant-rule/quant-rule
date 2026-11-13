//! Streaming a timeframe ladder.
//!
//! Inspection needs counts, and a run needs the bars themselves. Both fold the
//! same buckets [`crate::TimeframeSet`] publishes, so a count, an emitted candle,
//! and a published series cannot disagree.

use crate::bucket::Bucket;
use crate::time::{TimestampError, is_supported_timestamp};
use crate::timeframe_set::{check_m1_alignment, series_position};
use crate::{CandleError, CandleValues, Timeframe, TimeframeError, WindowError};

/// One event from a ladder fold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LadderEmission {
    /// A complete bar.
    Bar(Timeframe, CandleValues),
    /// A bucket that ended before every expected M1 candle arrived. The stamp
    /// is the bucket's UTC start.
    Partial(Timeframe, i64),
}

/// Streams a ladder without retaining it.
///
/// Feed closed M1 candles in strictly increasing order, then call
/// [`LadderWriter::finish`] so a complete trailing bucket is emitted.
#[derive(Debug, Clone, PartialEq)]
pub struct LadderWriter {
    buckets: [Option<Bucket>; 4],
    last_timestamp: Option<i64>,
    finished: bool,
}

impl LadderWriter {
    /// Creates an empty writer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buckets: [None; 4],
            last_timestamp: None,
            finished: false,
        }
    }

    /// Accepts one M1 candle and emits every bar or partial it closes.
    ///
    /// # Errors
    ///
    /// Returns [`TimeframeError::Window`] for an unsupported, duplicate, or
    /// out-of-order timestamp, [`TimeframeError::MisalignedM1Timestamp`] when
    /// the row is not on a whole-minute boundary, and
    /// [`TimeframeError::VolumeOverflow`] when a bucket sum is no longer finite.
    pub fn push<F>(&mut self, values: CandleValues, emit: &mut F) -> Result<(), TimeframeError>
    where
        F: FnMut(LadderEmission),
    {
        self.validate(values)?;
        self.last_timestamp = Some(values.timestamp);
        emit(LadderEmission::Bar(Timeframe::M1, values));

        for (index, timeframe) in Timeframe::DERIVED.iter().enumerate() {
            let bucket_start = timeframe.bucket_start(values.timestamp);
            let transition =
                self.buckets[index].is_some_and(|bucket| bucket.start() != bucket_start);
            if transition && let Some(bucket) = self.buckets[index].take() {
                Self::record(*timeframe, bucket, emit);
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
    pub fn finish<F>(&mut self, emit: &mut F)
    where
        F: FnMut(LadderEmission),
    {
        if self.finished {
            return;
        }
        for (index, timeframe) in Timeframe::DERIVED.iter().enumerate() {
            if let Some(bucket) = self.buckets[index].take() {
                Self::record(*timeframe, bucket, emit);
            }
        }
        self.finished = true;
    }

    /// Whether [`LadderWriter::finish`] has been called.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    fn validate(&self, values: CandleValues) -> Result<(), TimeframeError> {
        if self.finished {
            return Err(TimeframeError::Finished);
        }
        if !is_supported_timestamp(values.timestamp) {
            return Err(TimeframeError::Window(WindowError::Candle(
                CandleError::Timestamp(TimestampError::UnsupportedYear),
            )));
        }
        check_m1_alignment(values.timestamp)?;
        if let Some(previous) = self.last_timestamp {
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
        Ok(())
    }

    fn record<F>(timeframe: Timeframe, bucket: Bucket, emit: &mut F)
    where
        F: FnMut(LadderEmission),
    {
        if bucket.is_complete() {
            emit(LadderEmission::Bar(timeframe, bucket.into_values()));
        } else {
            emit(LadderEmission::Partial(timeframe, bucket.start()));
        }
    }
}

impl Default for LadderWriter {
    fn default() -> Self {
        Self::new()
    }
}

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
    writer: LadderWriter,
    series: [TimeframeCoverage; 5],
}

impl LadderSummary {
    /// Creates an empty summary.
    #[must_use]
    pub fn new() -> Self {
        Self {
            writer: LadderWriter::new(),
            series: Timeframe::ALL.map(TimeframeCoverage::new),
        }
    }

    /// Accepts one M1 candle.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`LadderWriter::push`].
    pub fn push(&mut self, values: CandleValues) -> Result<(), TimeframeError> {
        let writer = &mut self.writer;
        let series = &mut self.series;
        writer.push(values, &mut |emission| record(series, emission))
    }

    /// Finalizes the trailing buckets. Calling it again is a no-op.
    pub fn finish(&mut self) {
        let Self { writer, series } = self;
        writer.finish(&mut |emission| record(series, emission));
    }

    /// Whether [`LadderSummary::finish`] has been called.
    #[must_use]
    pub const fn is_finished(&self) -> bool {
        self.writer.is_finished()
    }

    /// Number of M1 candles accepted.
    #[must_use]
    pub fn candles(&self) -> u64 {
        self.series[series_position(Timeframe::M1)].bars
    }

    /// First M1 candle, Unix seconds UTC.
    #[must_use]
    pub fn first_candle(&self) -> Option<i64> {
        self.series[series_position(Timeframe::M1)].first_bar
    }

    /// Last M1 candle, Unix seconds UTC.
    #[must_use]
    pub fn last_candle(&self) -> Option<i64> {
        self.series[series_position(Timeframe::M1)].last_bar
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
}

impl Default for LadderSummary {
    fn default() -> Self {
        Self::new()
    }
}

fn record(series: &mut [TimeframeCoverage; 5], emission: LadderEmission) {
    match emission {
        LadderEmission::Bar(timeframe, candle) => {
            let coverage = &mut series[series_position(timeframe)];
            coverage.bars += 1;
            coverage.first_bar.get_or_insert(candle.timestamp);
            coverage.last_bar = Some(candle.timestamp);
        }
        LadderEmission::Partial(timeframe, _) => {
            series[series_position(timeframe)].partial_buckets += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: i64 = 1_735_689_600;

    fn values(offset: i64) -> CandleValues {
        CandleValues {
            timestamp: BASE + offset * 60,
            open: 100.0 + offset as f64,
            high: 101.0 + offset as f64,
            low: 99.0 + offset as f64,
            close: 100.5 + offset as f64,
            volume: Some(10.0),
        }
    }

    fn push_minutes(summary: &mut LadderSummary, minutes: i64) {
        for offset in 0..minutes {
            summary.push(values(offset)).expect("candle");
        }
    }

    fn writer_emissions(minutes: i64) -> Vec<LadderEmission> {
        let mut writer = LadderWriter::new();
        let mut emissions = Vec::new();
        for offset in 0..minutes {
            writer
                .push(values(offset), &mut |emission| emissions.push(emission))
                .expect("candle");
        }
        writer.finish(&mut |emission| emissions.push(emission));
        emissions
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

    #[test]
    fn the_writer_emits_m1_immediately_and_m5_at_finish() {
        let emissions = writer_emissions(5);
        let m1 = emissions
            .iter()
            .filter(|emission| matches!(emission, LadderEmission::Bar(Timeframe::M1, _)))
            .count();
        let m5 = emissions
            .iter()
            .filter(|emission| matches!(emission, LadderEmission::Bar(Timeframe::M5, _)))
            .count();
        assert_eq!(m1, 5);
        assert_eq!(m5, 1);
    }

    #[test]
    fn the_writer_emits_a_partial_for_an_incomplete_bucket() {
        let emissions = writer_emissions(2);
        assert!(emissions.iter().any(|emission| matches!(
            emission,
            LadderEmission::Partial(Timeframe::M5, start) if *start == BASE
        )));
    }

    #[test]
    fn the_writer_matches_the_published_set() {
        let mut set = crate::TimeframeSet::with_limit(100);
        for offset in 0..12 {
            set.push_values(values(offset)).expect("set");
        }
        set.finish().expect("finish");

        let emissions = writer_emissions(12);
        for timeframe in Timeframe::ALL {
            let emitted: Vec<CandleValues> = emissions
                .iter()
                .filter_map(|emission| match emission {
                    LadderEmission::Bar(emitted, candle) if emitted == &timeframe => Some(*candle),
                    _ => None,
                })
                .collect();
            let published: Vec<CandleValues> = set
                .series(timeframe)
                .iter()
                .map(|candle| CandleValues {
                    timestamp: candle.timestamp(),
                    open: candle.open(),
                    high: candle.high(),
                    low: candle.low(),
                    close: candle.close(),
                    volume: candle.volume(),
                })
                .collect();
            assert_eq!(emitted, published, "{}", timeframe.code());
        }
    }

    #[test]
    fn the_same_rows_give_the_same_emissions() {
        assert_eq!(writer_emissions(12), writer_emissions(12));
    }
}
