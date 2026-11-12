//! Compression-breakout events.
//!
//! A compression breakout is a signal event: a coiling volatility and
//! participation regime is followed by a close beyond the compression range
//! with rising participation. The event publishes the breakout direction, but
//! its claim is forward range expansion, not direction.
//!
//! The module is pure: it reads candles, readings, and the published context.
//! It performs no I/O, reads no clock, and uses no randomness.

pub use crate::events::source_codes::ATR_SOURCE_CODE;
use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::context::{COILING_CODE, ContextSeries};
use crate::events::signal::SignalSubtype;
use crate::events::{EventRecord, EventType};
use crate::readings::Readings;

/// The configuration of a compression-breakout run.
///
/// The defaults are the pre-registered protocol values: five coiling bars, a
/// 0.2 ATR break distance, and a 1.2 RVOL confirmation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionBreakoutConfig {
    timeframe_secs: NonZeroU64,
    coiling_bars: usize,
    break_atr: f64,
    rvol_min: f64,
}

impl CompressionBreakoutConfig {
    /// The contract defaults.
    pub const DEFAULT_COILING_BARS: usize = 5;
    /// The contract default break distance in ATR units.
    pub const DEFAULT_BREAK_ATR: f64 = 0.2;
    /// The contract default RVOL confirmation.
    pub const DEFAULT_RVOL_MIN: f64 = 1.2;

    /// Builds a configuration with the default thresholds.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self {
            timeframe_secs,
            coiling_bars: Self::DEFAULT_COILING_BARS,
            break_atr: Self::DEFAULT_BREAK_ATR,
            rvol_min: Self::DEFAULT_RVOL_MIN,
        }
    }

    /// Builds a configuration with explicit thresholds.
    #[must_use]
    pub const fn with_settings(
        timeframe_secs: NonZeroU64,
        coiling_bars: usize,
        break_atr: f64,
        rvol_min: f64,
    ) -> Self {
        Self {
            timeframe_secs,
            coiling_bars,
            break_atr,
            rvol_min,
        }
    }

    /// The configured timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(self) -> u64 {
        self.timeframe_secs.get()
    }

    /// The number of coiling bars before the trigger.
    #[must_use]
    pub const fn coiling_bars(self) -> usize {
        self.coiling_bars
    }

    /// The break distance in ATR units.
    #[must_use]
    pub const fn break_atr(self) -> f64 {
        self.break_atr
    }

    /// The minimum RVOL ratio on the trigger bar.
    #[must_use]
    pub const fn rvol_min(self) -> f64 {
        self.rvol_min
    }

    /// Whether the thresholds are usable by the detector.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.coiling_bars > 0
            && self.break_atr.is_finite()
            && self.break_atr > 0.0
            && self.rvol_min.is_finite()
            && self.rvol_min > 0.0
    }
}

/// The compression-breakout events over one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionBreakoutSeries {
    events: Vec<EventRecord>,
}

impl CompressionBreakoutSeries {
    /// An empty event series.
    #[must_use]
    pub const fn empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Computes every compression-breakout event.
    ///
    /// The detector requires the previous `coiling_bars` bars to carry the
    /// published coiling context code, then a close beyond that window's high
    /// or low by `break_atr` ATR with RVOL at least `rvol_min`. The trigger bar's
    /// own context code is not required: the RVOL confirmation is the
    /// participation test, and requiring the context code to remain coiling
    /// would make the event impossible once participation rises.
    #[must_use]
    pub fn compute(
        candles: &[Candle],
        readings: &Readings,
        context: &ContextSeries,
        config: &CompressionBreakoutConfig,
    ) -> Self {
        if !config.is_valid() {
            return Self::empty();
        }
        let len = candles.len().min(readings.len()).min(context.len());
        let mut events = Vec::new();
        for bar in config.coiling_bars..len {
            let Some(atr) = readings.atr14.get(bar) else {
                continue;
            };
            if !atr.is_finite() || atr <= 0.0 {
                continue;
            }
            let Some(rvol) = readings.rvol20.get(bar) else {
                continue;
            };
            if !rvol.is_finite() || rvol < config.rvol_min {
                continue;
            }
            let window_start = bar - config.coiling_bars;
            if !(window_start..bar).all(|index| {
                context.get(index).and_then(|value| value.context_code) == Some(COILING_CODE)
            }) {
                continue;
            }
            let mut range_high = f64::NEG_INFINITY;
            let mut range_low = f64::INFINITY;
            for candle in &candles[window_start..bar] {
                range_high = range_high.max(candle.high());
                range_low = range_low.min(candle.low());
            }
            if !range_high.is_finite() || !range_low.is_finite() {
                continue;
            }
            let close = candles[bar].close();
            let distance = config.break_atr * atr;
            let long = close >= range_high + distance;
            let short = close <= range_low - distance;
            if long == short {
                continue;
            }
            let (direction, boundary) = if long {
                (1, range_high)
            } else {
                (-1, range_low)
            };
            events.push(EventRecord {
                event_code: crate::events::codes::SIGNAL_COMPRESSION_BREAKOUT,
                event_type: EventType::Signal,
                event_subtype_code: SignalSubtype::CompressionBreakout.code(),
                source_code: ATR_SOURCE_CODE,
                timeframe_secs: config.timeframe_secs(),
                occurred_bar_index: bar,
                available_bar_index: bar,
                direction_code: direction,
                reference_code: None,
                primary_value: Some(close),
                reference_value: Some(boundary),
            });
        }
        Self { events }
    }

    /// The events in bar order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    /// Whether no event was detected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// The configuration of a compression-range run.
///
/// The range event publishes the high and low of a completed coiling window. It
/// has no breakout distance and no participation threshold: those belong to the
/// breakout event, which is a separate claim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionRangeConfig {
    timeframe_secs: NonZeroU64,
    coiling_bars: usize,
}

impl CompressionRangeConfig {
    /// The contract default coiling window.
    pub const DEFAULT_COILING_BARS: usize = 5;

    /// Builds a range configuration with the default window.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self {
            timeframe_secs,
            coiling_bars: Self::DEFAULT_COILING_BARS,
        }
    }

    /// Builds a range configuration with an explicit window.
    #[must_use]
    pub const fn with_coiling_bars(timeframe_secs: NonZeroU64, coiling_bars: usize) -> Self {
        Self {
            timeframe_secs,
            coiling_bars,
        }
    }

    /// The configured timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(self) -> u64 {
        self.timeframe_secs.get()
    }

    /// The number of coiling bars in the window.
    #[must_use]
    pub const fn coiling_bars(self) -> usize {
        self.coiling_bars
    }

    /// Whether the window is usable by the detector.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.coiling_bars > 0
    }
}

/// The compression-range events over one timeframe.
///
/// The event is neutral: it publishes where the range is, not which side will
/// break. It becomes knowable at the last coiling bar, before any later bar is
/// read.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionRangeSeries {
    events: Vec<EventRecord>,
}

impl CompressionRangeSeries {
    /// An empty event series.
    #[must_use]
    pub const fn empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Computes every completed coiling-window event.
    ///
    /// A window ending at bar `b` requires the previous `coiling_bars` bars,
    /// including `b`, to carry the published coiling context code. Overlapping
    /// windows each produce their own event; the detector does not suppress a
    /// window because a neighbouring one also completed.
    #[must_use]
    pub fn compute(
        candles: &[Candle],
        context: &ContextSeries,
        config: &CompressionRangeConfig,
    ) -> Self {
        if !config.is_valid() {
            return Self::empty();
        }
        let bars = config.coiling_bars;
        let len = candles.len().min(context.len());
        let mut events = Vec::new();
        for bar in bars.saturating_sub(1)..len {
            let Some((range_high, range_low)) = coiling_range(candles, context, bar, bars) else {
                continue;
            };
            events.push(EventRecord {
                event_code: crate::events::codes::SIGNAL_COMPRESSION_RANGE,
                event_type: EventType::Signal,
                event_subtype_code: SignalSubtype::CompressionRange.code(),
                source_code: ATR_SOURCE_CODE,
                timeframe_secs: config.timeframe_secs(),
                occurred_bar_index: bar,
                available_bar_index: bar,
                direction_code: 0,
                reference_code: None,
                primary_value: Some(range_high),
                reference_value: Some(range_low),
            });
        }
        Self { events }
    }

    /// The events in bar order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    /// Whether no event was detected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// The first compression-range event of a coiling run.
///
/// The onset event fires only when the completed window is all-coiling and the
/// bar immediately before it carries a known non-coiling context code. The
/// unknown-predecessor case is skipped: an onset cannot be claimed when the
/// prior regime is not published.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionRangeOnsetSeries {
    events: Vec<EventRecord>,
}

impl CompressionRangeOnsetSeries {
    /// An empty event series.
    #[must_use]
    pub const fn empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Computes the first completed coiling window after a non-coiling bar.
    #[must_use]
    pub fn compute(
        candles: &[Candle],
        context: &ContextSeries,
        config: &CompressionRangeConfig,
    ) -> Self {
        if !config.is_valid() {
            return Self::empty();
        }
        let bars = config.coiling_bars;
        let len = candles.len().min(context.len());
        let mut events = Vec::new();
        for bar in bars..len {
            let Some(previous) = context.get(bar - bars).and_then(|value| value.context_code)
            else {
                continue;
            };
            if previous == COILING_CODE {
                continue;
            }
            let Some((range_high, range_low)) = coiling_range(candles, context, bar, bars) else {
                continue;
            };
            events.push(EventRecord {
                event_code: crate::events::codes::SIGNAL_COMPRESSION_RANGE_ONSET,
                event_type: EventType::Signal,
                event_subtype_code: SignalSubtype::CompressionRangeOnset.code(),
                source_code: ATR_SOURCE_CODE,
                timeframe_secs: config.timeframe_secs(),
                occurred_bar_index: bar,
                available_bar_index: bar,
                direction_code: 0,
                reference_code: None,
                primary_value: Some(range_high),
                reference_value: Some(range_low),
            });
        }
        Self { events }
    }

    /// The events in bar order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    /// Whether no event was detected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// The high and low of a completed coiling window, or `None`.
fn coiling_range(
    candles: &[Candle],
    context: &ContextSeries,
    bar: usize,
    bars: usize,
) -> Option<(f64, f64)> {
    let window_start = bar.checked_add(1)?.checked_sub(bars)?;
    if !(window_start..=bar)
        .all(|index| context.get(index).and_then(|value| value.context_code) == Some(COILING_CODE))
    {
        return None;
    }
    let mut range_high = f64::NEG_INFINITY;
    let mut range_low = f64::INFINITY;
    for candle in &candles[window_start..=bar] {
        range_high = range_high.max(candle.high());
        range_low = range_low.min(candle.low());
    }
    (range_high.is_finite() && range_low.is_finite()).then_some((range_high, range_low))
}

#[cfg(test)]
mod tests {
    use super::{
        CompressionBreakoutConfig, CompressionBreakoutSeries, CompressionRangeConfig,
        CompressionRangeOnsetSeries, CompressionRangeSeries,
    };
    use crate::context::ContextSeries;
    use crate::events::EventType;
    use crate::readings::{PivotSeries, ReadingSeries, Readings};
    use quantrule_data::Candle;
    use std::num::NonZeroU64;

    fn candle(index: u64, high: f64, low: f64, close: f64) -> Candle {
        Candle::new(index, index as i64, close, high, low, close, Some(10.0))
            .expect("the test candle is valid")
    }

    fn series(len: usize, values: impl Fn(usize) -> Option<f64>) -> ReadingSeries {
        ReadingSeries::new((0..len).map(values).collect())
    }

    /// 30 bars: 20 baseline ATR bars, five coiling bars, and five trigger bars.
    fn fixture() -> (Vec<Candle>, Readings, ContextSeries) {
        let mut candles = Vec::new();
        for index in 0..30 {
            let close = if index == 25 { 101.0 } else { 100.0 };
            let high = if index == 25 { 101.2 } else { 100.5 };
            let low = if index == 25 { 100.8 } else { 99.5 };
            candles.push(candle(index as u64, high, low, close));
        }
        let readings = Readings {
            ema20: series(30, |_| Some(100.0)),
            ema50: series(30, |_| Some(100.0)),
            ema200: series(30, |_| Some(100.0)),
            rsi14: series(30, |_| Some(50.0)),
            atr14: series(30, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
            rvol20: series(30, |index| Some(if index < 25 { 0.5 } else { 1.5 })),
            pivots: PivotSeries::compute(&[]),
        };
        let context = ContextSeries::compute(&readings);
        (candles, readings, context)
    }

    fn config() -> CompressionBreakoutConfig {
        CompressionBreakoutConfig::with_settings(
            NonZeroU64::new(300).expect("a non-zero timeframe"),
            5,
            0.2,
            1.2,
        )
    }

    #[test]
    fn a_coiling_window_followed_by_a_breakout_emits_one_event() {
        let (candles, readings, context) = fixture();
        let series = CompressionBreakoutSeries::compute(&candles, &readings, &context, &config());
        assert_eq!(series.events().len(), 1);
        let event = series.events()[0];
        assert_eq!(event.event_type, EventType::Signal);
        assert_eq!(event.event_subtype_code, 7);
        assert_eq!(event.source_code, 5);
        assert_eq!(event.direction_code, 1);
        assert_eq!(event.occurred_bar_index, 25);
        assert_eq!(event.available_bar_index, 25);
        assert_eq!(event.primary_value, Some(101.0));
        assert_eq!(event.reference_value, Some(100.5));
    }

    #[test]
    fn no_event_without_the_coiling_window() {
        let (candles, readings, _) = fixture();
        let context = ContextSeries::compute(&Readings {
            atr14: series(30, |_| Some(2.0)),
            ..readings.clone()
        });
        let series = CompressionBreakoutSeries::compute(&candles, &readings, &context, &config());
        assert!(series.is_empty());
    }

    #[test]
    fn no_event_without_the_rvol_confirmation() {
        let (candles, readings, context) = fixture();
        let readings = Readings {
            rvol20: series(30, |_| Some(0.5)),
            ..readings
        };
        let series = CompressionBreakoutSeries::compute(&candles, &readings, &context, &config());
        assert!(series.is_empty());
    }

    #[test]
    fn no_event_when_the_break_distance_is_not_reached() {
        let (candles, readings, context) = fixture();
        let series = CompressionBreakoutSeries::compute(
            &candles,
            &readings,
            &context,
            &CompressionBreakoutConfig::with_settings(
                NonZeroU64::new(300).expect("a non-zero timeframe"),
                5,
                2.0,
                1.2,
            ),
        );
        assert!(series.is_empty());
    }

    #[test]
    fn a_missing_atr_produces_no_event() {
        let (candles, readings, context) = fixture();
        let readings = Readings {
            atr14: series(30, |index| {
                (index != 25).then_some(if index < 20 { 2.0 } else { 1.0 })
            }),
            ..readings
        };
        let series = CompressionBreakoutSeries::compute(&candles, &readings, &context, &config());
        assert!(series.is_empty());
    }

    #[test]
    fn the_prefix_events_are_unchanged_when_later_bars_are_appended() {
        let (candles, readings, context) = fixture();
        let full = CompressionBreakoutSeries::compute(&candles, &readings, &context, &config());
        let prefix =
            CompressionBreakoutSeries::compute(&candles[..26], &readings, &context, &config());
        assert_eq!(prefix.events(), full.events());
    }

    #[test]
    fn the_detector_is_deterministic() {
        let (candles, readings, context) = fixture();
        let first = CompressionBreakoutSeries::compute(&candles, &readings, &context, &config());
        let second = CompressionBreakoutSeries::compute(&candles, &readings, &context, &config());
        assert_eq!(first, second);
    }

    #[test]
    fn an_empty_series_is_empty() {
        let readings = Readings::compute(&[]);
        let context = ContextSeries::compute(&readings);
        let series = CompressionBreakoutSeries::compute(&[], &readings, &context, &config());
        assert!(series.is_empty());
    }

    fn range_config() -> CompressionRangeConfig {
        CompressionRangeConfig::with_coiling_bars(
            NonZeroU64::new(300).expect("a non-zero timeframe"),
            5,
        )
    }

    #[test]
    fn a_completed_coiling_window_emits_one_neutral_range_event() {
        let (candles, _, context) = fixture();
        let series = CompressionRangeSeries::compute(&candles, &context, &range_config());
        assert_eq!(series.events().len(), 1);
        let event = series.events()[0];
        assert_eq!(event.event_type, EventType::Signal);
        assert_eq!(event.event_subtype_code, 8);
        assert_eq!(event.source_code, 5);
        assert_eq!(event.direction_code, 0);
        assert_eq!(event.occurred_bar_index, 24);
        assert_eq!(event.available_bar_index, 24);
        assert_eq!(event.primary_value, Some(100.5));
        assert_eq!(event.reference_value, Some(99.5));
    }

    #[test]
    fn overlapping_coiling_windows_emit_one_event_each() {
        let (candles, readings, _) = fixture();
        let readings = Readings {
            atr14: series(30, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
            rvol20: series(30, |_| Some(0.5)),
            ..readings
        };
        let context = ContextSeries::compute(&readings);
        let series = CompressionRangeSeries::compute(&candles, &context, &range_config());
        assert_eq!(series.events().len(), 6);
        assert_eq!(
            series
                .events()
                .iter()
                .map(|event| event.available_bar_index)
                .collect::<Vec<_>>(),
            vec![24, 25, 26, 27, 28, 29]
        );
        assert!(
            series
                .events()
                .iter()
                .all(|event| event.direction_code == 0)
        );
    }

    #[test]
    fn a_non_coiling_window_emits_nothing() {
        let (candles, readings, _) = fixture();
        let readings = Readings {
            atr14: series(30, |_| Some(2.0)),
            rvol20: series(30, |_| Some(0.5)),
            ..readings
        };
        let context = ContextSeries::compute(&readings);
        let series = CompressionRangeSeries::compute(&candles, &context, &range_config());
        assert!(series.is_empty());
    }

    #[test]
    fn a_series_shorter_than_the_window_emits_nothing() {
        let readings = Readings::compute(&[]);
        let context = ContextSeries::compute(&readings);
        let series = CompressionRangeSeries::compute(&[], &context, &range_config());
        assert!(series.is_empty());

        let candles = vec![candle(0, 100.0, 99.0, 100.0); 4];
        let series = CompressionRangeSeries::compute(&candles, &context, &range_config());
        assert!(series.is_empty());
    }

    #[test]
    fn equal_highs_and_lows_publish_equal_boundaries() {
        let candles = vec![candle(0, 100.0, 100.0, 100.0); 25];
        let readings = Readings {
            ema20: series(25, |_| Some(100.0)),
            ema50: series(25, |_| Some(100.0)),
            ema200: series(25, |_| Some(100.0)),
            rsi14: series(25, |_| Some(50.0)),
            atr14: series(25, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
            rvol20: series(25, |_| Some(0.5)),
            pivots: PivotSeries::compute(&[]),
        };
        let context = ContextSeries::compute(&readings);
        let series = CompressionRangeSeries::compute(&candles, &context, &range_config());
        assert_eq!(series.events().len(), 1);
        let event = series.events()[0];
        assert_eq!(event.primary_value, Some(100.0));
        assert_eq!(event.reference_value, Some(100.0));
    }

    #[test]
    fn the_range_prefix_events_are_unchanged_when_later_bars_are_appended() {
        let (candles, readings, _) = fixture();
        let readings = Readings {
            atr14: series(30, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
            rvol20: series(30, |_| Some(0.5)),
            ..readings
        };
        let context = ContextSeries::compute(&readings);
        let full = CompressionRangeSeries::compute(&candles, &context, &range_config());
        for length in 0..=candles.len() {
            let prefix_context = ContextSeries::compute(&Readings {
                atr14: series(length, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
                rvol20: series(length, |_| Some(0.5)),
                ..Readings::compute(&candles[..length])
            });
            let prefix = CompressionRangeSeries::compute(
                &candles[..length],
                &prefix_context,
                &range_config(),
            );
            let expected: Vec<_> = full
                .events()
                .iter()
                .filter(|event| event.available_bar_index < length)
                .copied()
                .collect();
            assert_eq!(
                prefix.events(),
                expected.as_slice(),
                "the prefix of length {length} changed an earlier range event"
            );
        }
    }

    #[test]
    fn the_first_window_after_a_non_coiling_bar_is_an_onset() {
        let (candles, _, context) = fixture();
        let series = CompressionRangeOnsetSeries::compute(&candles, &context, &range_config());
        assert_eq!(series.events().len(), 1);
        let event = series.events()[0];
        assert_eq!(event.event_subtype_code, 9);
        assert_eq!(event.direction_code, 0);
        assert_eq!(event.available_bar_index, 24);
        assert_eq!(event.primary_value, Some(100.5));
        assert_eq!(event.reference_value, Some(99.5));
    }

    #[test]
    fn overlapping_windows_after_the_onset_are_suppressed() {
        let (candles, readings, _) = fixture();
        let readings = Readings {
            atr14: series(30, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
            rvol20: series(30, |_| Some(0.5)),
            ..readings
        };
        let context = ContextSeries::compute(&readings);
        let series = CompressionRangeOnsetSeries::compute(&candles, &context, &range_config());
        assert_eq!(series.events().len(), 1);
        assert_eq!(series.events()[0].available_bar_index, 24);
    }

    #[test]
    fn a_window_without_a_known_non_coiling_predecessor_is_not_an_onset() {
        let (candles, readings, _) = fixture();
        let readings = Readings {
            atr14: series(30, |_| Some(2.0)),
            rvol20: series(30, |_| Some(0.5)),
            ..readings
        };
        let context = ContextSeries::compute(&readings);
        let series = CompressionRangeOnsetSeries::compute(&candles, &context, &range_config());
        assert!(series.is_empty());
    }

    #[test]
    fn the_onset_prefix_events_are_unchanged_when_later_bars_are_appended() {
        let (candles, readings, _) = fixture();
        let readings = Readings {
            atr14: series(30, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
            rvol20: series(30, |_| Some(0.5)),
            ..readings
        };
        let context = ContextSeries::compute(&readings);
        let full = CompressionRangeOnsetSeries::compute(&candles, &context, &range_config());
        for length in 0..=candles.len() {
            let prefix_context = ContextSeries::compute(&Readings {
                atr14: series(length, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
                rvol20: series(length, |_| Some(0.5)),
                ..Readings::compute(&candles[..length])
            });
            let prefix = CompressionRangeOnsetSeries::compute(
                &candles[..length],
                &prefix_context,
                &range_config(),
            );
            let expected: Vec<_> = full
                .events()
                .iter()
                .filter(|event| event.available_bar_index < length)
                .copied()
                .collect();
            assert_eq!(
                prefix.events(),
                expected.as_slice(),
                "the prefix of length {length} changed an earlier onset event"
            );
        }
    }

    #[test]
    fn the_onset_detector_is_deterministic() {
        let (candles, _, context) = fixture();
        let first = CompressionRangeOnsetSeries::compute(&candles, &context, &range_config());
        let second = CompressionRangeOnsetSeries::compute(&candles, &context, &range_config());
        assert_eq!(first, second);
    }

    #[test]
    fn the_range_detector_is_deterministic() {
        let (candles, _, context) = fixture();
        let first = CompressionRangeSeries::compute(&candles, &context, &range_config());
        let second = CompressionRangeSeries::compute(&candles, &context, &range_config());
        assert_eq!(first, second);
    }
}
