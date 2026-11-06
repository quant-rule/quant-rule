//! Multi-timeframe construction from a single M1 stream.
//!
//! The set owns one bounded series per supported timeframe. M1 candles enter
//! through [`TimeframeSet::push`] or [`TimeframeSet::push_values`]; the engine
//! builds M5, M15, H1, and H4 from them. A derived bucket is published only after
//! every expected M1 candle is present, so a partial bucket can never become a
//! candle. Gaps are recorded between consecutive candles of each series.
//!
//! The set accepts sparse whole-minute rows: a gap is data, not an error. The
//! shell's ladder loader additionally requires the source rows to be
//! M1-spaced before it publishes the set; see [`crate::SourceCadence`].
//!
//! This module performs no I/O and reads no clock. The shell supplies the M1
//! candles and renders the resulting series.

use std::fmt;

use crate::bucket::Bucket;
use crate::{
    Candle, CandleError, CandleValues, CandleWindow, RawCandle, Timeframe, TimestampError,
    WindowError, format_utc_timestamp, is_supported_timestamp, parse_utc_timestamp,
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
/// The series owns a [`CandleWindow`] and the gap and partial-bucket records
/// produced while it was built. Candles use the same [`Candle`] shape as every
/// other timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeSeries {
    timeframe: Timeframe,
    window: CandleWindow,
    gaps: Vec<Gap>,
    partial_buckets: Vec<PartialBucket>,
}

impl TimeframeSeries {
    fn new(timeframe: Timeframe) -> Self {
        Self::with_window(timeframe, CandleWindow::new())
    }

    fn with_window(timeframe: Timeframe, window: CandleWindow) -> Self {
        Self {
            timeframe,
            window,
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

    /// The series' candle limit.
    ///
    /// The limit is the refusal bound: a fixed-retention series rejects the
    /// candle over it instead of evicting the oldest one.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.window.capacity()
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
        let Some(delta) = current.checked_sub(previous) else {
            return;
        };
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

/// A set of first-class M1, M5, M15, H1, and H4 series.
///
/// Feed closed M1 candles in strictly increasing timestamp order. The set
/// validates and retains M1, derives M5/M15/H1, and bounds every series. Call
/// [`TimeframeSet::finish`] at the end of the input so a complete trailing
/// bucket is published and an incomplete one is recorded as partial.
///
/// [`TimeframeSet::new`] keeps the most recent [`crate::DEFAULT_WINDOW`]
/// candles per series and evicts the oldest; it is the inspection set the
/// shell's existing commands use. [`TimeframeSet::full`] keeps every candle up
/// to [`crate::MAX_SERIES`] and rejects the next one, so a long dataset is
/// never silently truncated.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeSet {
    series: [TimeframeSeries; 5],
    buckets: [Option<Bucket>; 4],
    finished: bool,
}

impl TimeframeSet {
    /// Creates an empty trailing set with one series per supported timeframe.
    #[must_use]
    pub fn new() -> Self {
        Self {
            series: Timeframe::ALL.map(TimeframeSeries::new),
            buckets: [None; 4],
            finished: false,
        }
    }

    /// Creates an empty full-series set with the [`crate::MAX_SERIES`] cap.
    ///
    /// The 100,001st M1 candle is rejected with
    /// [`TimeframeError::Window`] carrying [`WindowError::CapacityExceeded`],
    /// rather than evicting the first candle.
    #[must_use]
    pub fn full() -> Self {
        Self::with_limit(crate::MAX_SERIES)
    }

    /// Creates an empty fixed-retention set for an extended analysis range.
    ///
    /// The set keeps up to [`crate::EXTENDED_SERIES`] M1 candles and rejects
    /// the next one instead of evicting the first. It exists for a full-period
    /// analysis or backtest; the existing model commands, the store, and
    /// research keep [`crate::MAX_SERIES`].
    ///
    /// The extended limit is a refusal bound. A series that would exceed it
    /// fails with [`TimeframeError::Window`] rather than being truncated, so a
    /// partial year can never look complete.
    #[must_use]
    pub fn extended() -> Self {
        Self::with_limit(crate::EXTENDED_SERIES)
    }

    /// Creates an empty set with a fixed-retention window per series.
    ///
    /// This is the shared constructor behind [`TimeframeSet::full`],
    /// [`TimeframeSet::extended`], and the tests. Every series receives the
    /// same bound; the limit is a refusal bound rather than an up-front
    /// allocation.
    #[must_use]
    pub fn with_limit(limit: usize) -> Self {
        Self {
            series: Timeframe::ALL.map(|timeframe| {
                TimeframeSeries::with_window(timeframe, CandleWindow::with_limit(limit))
            }),
            buckets: [None; 4],
            finished: false,
        }
    }

    /// Parses and accepts one M1 source candle.
    ///
    /// On success, returns the stable series index assigned in the M1 series.
    /// A candle rejected by the M1 window does not change any series or
    /// aggregation state. A volume overflow is fatal: the M1 candle has already
    /// been accepted when the bucket reports it, so the set must be discarded.
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
        if !is_supported_timestamp(values.timestamp) {
            return Err(TimeframeError::Window(WindowError::Candle(
                CandleError::Timestamp(TimestampError::UnsupportedYear),
            )));
        }
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

    /// Iterates over M1, M5, M15, H1, and H4 in that fixed order.
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
                Some(bucket) => bucket.push(values).map_err(|error| match error {
                    crate::bucket::BucketError::VolumeOverflow { bucket_start } => {
                        TimeframeError::VolumeOverflow { bucket_start }
                    }
                })?,
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
    /// The source rows are not spaced as M1 candles.
    ///
    /// A ladder built from a native higher-timeframe file would label hourly
    /// rows `1m` and leave every derived series empty. The shell rejects the
    /// file instead of publishing that state.
    SourceCadence {
        /// The smallest spacing between consecutive source rows, in seconds.
        observed_seconds: i64,
    },
    /// The set was already finished.
    Finished,
    /// A higher-timeframe bucket's volume sum exceeded the finite range.
    VolumeOverflow {
        /// The UTC start timestamp of the bucket that overflowed.
        bucket_start: i64,
    },
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
            Self::SourceCadence { observed_seconds } => write!(
                formatter,
                "source cadence is {observed_seconds}s, not 60s; provide an M1 source file for the timeframe ladder"
            ),
            Self::Finished => formatter.write_str("the timeframe set is already finished"),
            Self::VolumeOverflow { bucket_start } => write!(
                formatter,
                "aggregated volume overflowed in the bucket starting {}",
                format_utc_timestamp(*bucket_start)
            ),
        }
    }
}

impl std::error::Error for TimeframeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Window(source) => Some(source),
            Self::MisalignedM1Timestamp { .. }
            | Self::SourceCadence { .. }
            | Self::Finished
            | Self::VolumeOverflow { .. } => None,
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
mod tests;
