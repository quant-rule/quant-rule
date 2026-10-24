//! Level events.
//!
//! A Level event records price interacting with one of seven bounded references
//! per timeframe: EMA 20, EMA 50, EMA 200, and the latest confirmed swing high
//! and swing low at the minor and major scales. The detector evaluates break,
//! cross, and test/retest in that precedence, and emits at most one event per
//! reference per bar.
//!
//! ATR 14 is required at the evaluation bar. The break and test predicates use
//! it, so without it the subtype precedence cannot be resolved; the bar produces
//! no Level event. A missing ATR is not replaced by zero, and an ATR of exactly
//! zero remains a measured value.
//!
//! The module is pure: it reads only the candles, readings, and configuration it
//! is given. It performs no I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::axes::structure::registry::{ConfirmedLevel, ConfirmedLevelRegistry};
use crate::events::{EventRecord, EventType};
use crate::readings::{PivotKind, PivotScale, Readings};

/// The fraction of ATR 14 that scales the Level-event proximity zone.
pub const PROXIMITY_ATR_FRACTION: f64 = 0.2;

/// The contract's source code for EMA 20.
pub const EMA20_SOURCE_CODE: u16 = 1;
/// The contract's source code for EMA 50.
pub const EMA50_SOURCE_CODE: u16 = 2;
/// The contract's source code for EMA 200.
pub const EMA200_SOURCE_CODE: u16 = 3;
/// The contract's source code for confirmed minor pivots.
pub const MINOR_PIVOT_SOURCE_CODE: u16 = 7;
/// The contract's source code for confirmed major pivots.
pub const MAJOR_PIVOT_SOURCE_CODE: u16 = 8;

/// The contract's reference code for EMA 20.
pub const EMA20_REFERENCE_CODE: u32 = 1;
/// The contract's reference code for EMA 50.
pub const EMA50_REFERENCE_CODE: u32 = 2;
/// The contract's reference code for EMA 200.
pub const EMA200_REFERENCE_CODE: u32 = 3;

/// The Level event's compact subtype codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSubtype {
    /// Code 1: the candle range intersected the level's proximity zone.
    Test = 1,
    /// Code 2: consecutive closes crossed the level.
    Cross = 2,
    /// Code 3: the close crossed the level and finished at least 0.2 ATR beyond it.
    Break = 3,
}

impl LevelSubtype {
    /// Every subtype, in contract code order.
    pub const ALL: [Self; 3] = [Self::Test, Self::Cross, Self::Break];

    /// The contract's numeric code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The subtype for a contract code, or `None` when the code is unknown.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Test),
            2 => Some(Self::Cross),
            3 => Some(Self::Break),
            _ => None,
        }
    }
}

/// The configuration of a Level-event run.
///
/// The timeframe is required because every event record carries its source
/// timeframe. It must be a positive number of seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelEventConfig {
    timeframe_secs: NonZeroU64,
}

impl LevelEventConfig {
    /// Builds a configuration from a non-zero timeframe.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self { timeframe_secs }
    }

    /// Builds a configuration from a timeframe in seconds.
    ///
    /// Returns `None` when the timeframe is zero, because the contract requires
    /// a positive integer in every event record.
    #[must_use]
    pub fn from_secs(timeframe_secs: u64) -> Option<Self> {
        NonZeroU64::new(timeframe_secs).map(Self::new)
    }

    /// The configured timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(self) -> u64 {
        self.timeframe_secs.get()
    }
}

/// The Level events detected over one candle series.
///
/// The events are stored in bar order. Inside one bar they follow the reference
/// set's fixed order: EMA 20, EMA 50, EMA 200, minor high, minor low, major high,
/// major low. An event is appended once and is never upgraded in place.
///
/// The series holds every event computed over the input; it is a batch result,
/// not a live snapshot. A live consumer keeps the contract's bounded history of
/// the most recent records.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU64;
///
/// use quantrule_data::Candle;
/// use quantrule_model::events::level::{LevelEventConfig, LevelEventSeries};
/// use quantrule_model::readings::Readings;
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let readings = Readings::compute(&[candle]);
/// let config = LevelEventConfig::new(NonZeroU64::new(300).expect("a non-zero timeframe"));
/// let events = LevelEventSeries::compute(&[candle], &readings, &config);
///
/// assert!(events.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LevelEventSeries {
    events: Vec<EventRecord>,
}

impl LevelEventSeries {
    /// Computes every Level event over the closed candles.
    ///
    /// The input is not modified. The series covers the shorter of the candle
    /// and readings slices; the model's own readings always have equal length.
    #[must_use]
    pub fn compute(candles: &[Candle], readings: &Readings, config: &LevelEventConfig) -> Self {
        let len = candles.len().min(readings.len());
        let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
        let mut latest = LatestLevels::default();
        let mut next_level = 0;
        let mut events = Vec::new();

        for bar in 0..len {
            // Advance the confirmed-level pointer even when ATR is unavailable:
            // a level that becomes knowable now must be available on later bars.
            while let Some(level) = registry.get(next_level) {
                if level.available_at > bar {
                    break;
                }
                latest.apply(level);
                next_level += 1;
            }

            let Some(atr) = readings.atr14.get(bar) else {
                continue;
            };

            let mut references: [Option<Reference>; 7] = [None; 7];
            references[0] = ema_reference(
                readings.ema20.get(bar),
                EMA20_SOURCE_CODE,
                EMA20_REFERENCE_CODE,
            );
            references[1] = ema_reference(
                readings.ema50.get(bar),
                EMA50_SOURCE_CODE,
                EMA50_REFERENCE_CODE,
            );
            references[2] = ema_reference(
                readings.ema200.get(bar),
                EMA200_SOURCE_CODE,
                EMA200_REFERENCE_CODE,
            );
            references[3..7].copy_from_slice(&latest.references());

            let previous_close = if bar == 0 {
                None
            } else {
                Some(candles[bar - 1].close())
            };
            let candle = candles[bar];

            for reference in references.into_iter().flatten() {
                if let Some(event) = evaluate(reference, candle, previous_close, atr, config, bar) {
                    events.push(event);
                }
            }
        }

        Self { events }
    }

    /// The number of detected events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether no event was detected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Every detected event, in bar and reference order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    /// The latest detected event, or `None` when there is none.
    #[must_use]
    pub fn latest(&self) -> Option<&EventRecord> {
        self.events.last()
    }

    /// The events that became knowable on `bar`.
    pub fn events_at(&self, bar: usize) -> impl Iterator<Item = &EventRecord> + '_ {
        self.events
            .iter()
            .filter(move |event| event.available_bar_index == bar)
    }
}

/// One eligible price reference for the current bar.
#[derive(Debug, Clone, Copy)]
struct Reference {
    source_code: u16,
    reference_code: u32,
    price: f64,
}

/// The latest confirmed swing high and low at each pivot scale.
#[derive(Debug, Default, Clone, Copy)]
struct LatestLevels {
    minor_high: Option<ConfirmedLevel>,
    minor_low: Option<ConfirmedLevel>,
    major_high: Option<ConfirmedLevel>,
    major_low: Option<ConfirmedLevel>,
}

impl LatestLevels {
    /// Replaces the latest level of the confirmation's scale and kind.
    fn apply(&mut self, level: ConfirmedLevel) {
        match (level.scale, level.kind) {
            (PivotScale::Minor, PivotKind::High) => self.minor_high = Some(level),
            (PivotScale::Minor, PivotKind::Low) => self.minor_low = Some(level),
            (PivotScale::Major, PivotKind::High) => self.major_high = Some(level),
            (PivotScale::Major, PivotKind::Low) => self.major_low = Some(level),
        }
    }

    /// The four swing references in the contract's fixed order.
    fn references(&self) -> [Option<Reference>; 4] {
        [
            self.minor_high
                .map(|level| swing_reference(level, MINOR_PIVOT_SOURCE_CODE)),
            self.minor_low
                .map(|level| swing_reference(level, MINOR_PIVOT_SOURCE_CODE)),
            self.major_high
                .map(|level| swing_reference(level, MAJOR_PIVOT_SOURCE_CODE)),
            self.major_low
                .map(|level| swing_reference(level, MAJOR_PIVOT_SOURCE_CODE)),
        ]
    }
}

fn ema_reference(price: Option<f64>, source_code: u16, reference_code: u32) -> Option<Reference> {
    price.map(|price| Reference {
        source_code,
        reference_code,
        price,
    })
}

fn swing_reference(level: ConfirmedLevel, source_code: u16) -> Reference {
    Reference {
        source_code,
        reference_code: level.reference_code,
        price: level.price,
    }
}

/// Evaluates the three Level subtypes for one reference and one bar.
///
/// Precedence is break, then cross, then test/retest. The function returns at
/// most one event.
fn evaluate(
    reference: Reference,
    candle: Candle,
    previous_close: Option<f64>,
    atr: f64,
    config: &LevelEventConfig,
    bar: usize,
) -> Option<EventRecord> {
    let level = reference.price;
    let band = PROXIMITY_ATR_FRACTION * atr;
    let close = candle.close();

    let (crossed_up, crossed_down) = previous_close.map_or((false, false), |previous| {
        (
            previous < level && close > level,
            previous > level && close < level,
        )
    });
    let broke_up = crossed_up && close >= level + band;
    let broke_down = crossed_down && close <= level - band;
    let tested = candle.low() <= level + band && candle.high() >= level - band;

    let subtype = if broke_up || broke_down {
        LevelSubtype::Break
    } else if crossed_up || crossed_down {
        LevelSubtype::Cross
    } else if tested {
        LevelSubtype::Test
    } else {
        return None;
    };

    let direction_code = if close > level {
        1
    } else if close < level {
        -1
    } else {
        0
    };

    Some(EventRecord {
        event_type: EventType::Level,
        event_subtype_code: subtype.code(),
        source_code: reference.source_code,
        timeframe_secs: config.timeframe_secs(),
        occurred_bar_index: bar,
        available_bar_index: bar,
        direction_code,
        reference_code: Some(reference.reference_code),
        primary_value: Some(close),
        reference_value: Some(level),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        EMA20_REFERENCE_CODE, EMA20_SOURCE_CODE, EMA50_REFERENCE_CODE, LevelEventConfig,
        LevelEventSeries, LevelSubtype, MAJOR_PIVOT_SOURCE_CODE, MINOR_PIVOT_SOURCE_CODE,
        PROXIMITY_ATR_FRACTION,
    };
    use crate::readings::{PivotSeries, ReadingSeries, Readings};
    use quantrule_data::Candle;
    use std::num::NonZeroU64;

    fn candle(series_index: u64, timestamp: i64, high: f64, low: f64, close: f64) -> Candle {
        Candle::new(series_index, timestamp, close, high, low, close, Some(10.0))
            .expect("the test candle is valid")
    }

    fn candles_from(rows: &[(f64, f64, f64)]) -> Vec<Candle> {
        rows.iter()
            .enumerate()
            .map(|(index, (high, low, close))| {
                candle(index as u64, index as i64, *high, *low, *close)
            })
            .collect()
    }

    fn monotonic_candles() -> Vec<Candle> {
        candles_from(&[
            (100.5, 99.5, 100.0),
            (101.5, 100.5, 101.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
        ])
    }

    fn series_from(len: usize, value: impl Fn(usize) -> Option<f64>) -> ReadingSeries {
        ReadingSeries::new((0..len).map(value).collect())
    }

    fn readings_with(
        candles: &[Candle],
        ema20: impl Fn(usize) -> Option<f64>,
        ema50: impl Fn(usize) -> Option<f64>,
        ema200: impl Fn(usize) -> Option<f64>,
        atr14: impl Fn(usize) -> Option<f64>,
    ) -> Readings {
        let len = candles.len();
        Readings {
            ema20: series_from(len, ema20),
            ema50: series_from(len, ema50),
            ema200: series_from(len, ema200),
            rsi14: series_from(len, |_| None),
            atr14: series_from(len, atr14),
            rvol20: series_from(len, |_| None),
            pivots: PivotSeries::compute(candles),
        }
    }

    fn flat_readings(candles: &[Candle], ema20: Option<f64>, atr: Option<f64>) -> Readings {
        readings_with(candles, |_| ema20, |_| None, |_| None, |_| atr)
    }

    fn config() -> LevelEventConfig {
        LevelEventConfig::new(NonZeroU64::new(300).expect("a non-zero timeframe"))
    }

    fn event_for(
        series: &LevelEventSeries,
        bar: usize,
        reference_code: u32,
    ) -> Option<&crate::events::EventRecord> {
        series.events().iter().find(|event| {
            event.available_bar_index == bar && event.reference_code == Some(reference_code)
        })
    }

    #[test]
    fn a_test_emits_when_the_range_touches_the_zone_without_a_cross() {
        let candles = monotonic_candles();
        let mut readings = flat_readings(&candles, Some(100.0), Some(10.0));
        readings.ema20 = series_from(candles.len(), |bar| (bar >= 1).then_some(100.0));
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        let event = event_for(&events, 1, EMA20_REFERENCE_CODE).expect("a test event");
        assert_eq!(event.event_type.code(), 1);
        assert_eq!(event.event_subtype_code, LevelSubtype::Test.code());
        assert_eq!(event.source_code, EMA20_SOURCE_CODE);
        assert_eq!(event.direction_code, 1);
        assert_eq!(event.timeframe_secs, 300);
        assert_eq!(event.occurred_bar_index, 1);
        assert_eq!(event.available_bar_index, 1);
        assert_eq!(event.reference_value, Some(100.0));
        assert_eq!(event.primary_value, Some(101.0));
    }

    #[test]
    fn a_cross_emits_when_the_close_finishes_inside_the_proximity_band() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (99.5, 98.5, 99.0),
            (101.5, 100.5, 101.0),
        ]);
        let readings = readings_with(
            &candles,
            |bar| (bar == 2).then_some(100.0),
            |_| None,
            |_| None,
            |_| Some(10.0),
        );
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        let event = event_for(&events, 2, EMA20_REFERENCE_CODE).expect("a cross event");
        assert_eq!(event.event_subtype_code, LevelSubtype::Cross.code());
        assert_eq!(event.direction_code, 1);
    }

    #[test]
    fn a_break_takes_precedence_over_cross_and_test() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (99.5, 98.5, 99.0),
            (103.5, 101.5, 103.0),
        ]);
        let readings = readings_with(
            &candles,
            |bar| (bar == 2).then_some(100.0),
            |_| None,
            |_| None,
            |_| Some(10.0),
        );
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        let event = event_for(&events, 2, EMA20_REFERENCE_CODE).expect("a break event");
        assert_eq!(event.event_subtype_code, LevelSubtype::Break.code());
        assert_eq!(event.direction_code, 1);
    }

    #[test]
    fn a_cross_takes_precedence_over_a_test() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (99.5, 98.5, 99.0),
            (101.5, 100.5, 101.0),
        ]);
        let readings = readings_with(
            &candles,
            |bar| (bar == 2).then_some(100.0),
            |_| None,
            |_| None,
            |_| Some(10.0),
        );
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        let event = event_for(&events, 2, EMA20_REFERENCE_CODE).expect("a cross event");
        assert_eq!(event.event_subtype_code, LevelSubtype::Cross.code());
    }

    #[test]
    fn direction_records_the_side_of_the_close() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (100.5, 99.5, 100.0),
            (100.5, 99.5, 100.0),
            (100.5, 99.5, 100.0),
        ]);
        let mut readings = flat_readings(&candles, Some(100.0), Some(10.0));
        readings.ema20 = series_from(candles.len(), |bar| (bar >= 1).then_some(100.0));
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        assert_eq!(
            event_for(&events, 1, EMA20_REFERENCE_CODE).map(|event| event.direction_code),
            Some(0)
        );
    }

    #[test]
    fn a_missing_atr_produces_no_level_event() {
        let candles = monotonic_candles();
        let mut readings = flat_readings(&candles, Some(100.0), None);
        readings.ema20 = series_from(candles.len(), |bar| (bar >= 1).then_some(100.0));
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        assert!(events.is_empty());
    }

    #[test]
    fn a_zero_atr_is_a_measured_value_and_a_test_at_the_level_is_emitted() {
        let candles = candles_from(&[(101.0, 100.0, 101.0), (100.0, 100.0, 100.0)]);
        let mut readings = flat_readings(&candles, Some(100.0), Some(0.0));
        readings.ema20 = series_from(candles.len(), |bar| (bar >= 1).then_some(100.0));
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        let event = event_for(&events, 1, EMA20_REFERENCE_CODE).expect("a test event");
        assert_eq!(event.event_subtype_code, LevelSubtype::Test.code());
        assert_eq!(event.direction_code, 0);
        assert_eq!(event.reference_value, Some(100.0));
    }

    #[test]
    fn at_most_one_event_per_reference_per_bar() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (101.5, 100.5, 101.0),
            (101.5, 100.5, 101.0),
        ]);
        let mut readings = flat_readings(&candles, Some(100.0), Some(10.0));
        readings.ema20 = series_from(candles.len(), |bar| (bar >= 1).then_some(100.0));
        readings.ema50 = series_from(candles.len(), |bar| (bar >= 1).then_some(100.0));
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        let mut keys: Vec<(usize, u32)> = events
            .events()
            .iter()
            .map(|event| {
                (
                    event.available_bar_index,
                    event.reference_code.expect("a reference"),
                )
            })
            .collect();
        keys.sort_unstable();
        for pair in keys.windows(2) {
            assert_ne!(pair[0], pair[1], "a reference emitted twice on one bar");
        }
        assert_eq!(events.events_at(1).count(), 2);
        assert_eq!(events.events_at(2).count(), 2);
    }

    #[test]
    fn an_empty_series_has_no_events() {
        let candles = Vec::new();
        let readings = readings_with(&candles, |_| None, |_| None, |_| None, |_| None);
        let events = LevelEventSeries::compute(&candles, &readings, &config());
        assert!(events.is_empty());
        assert_eq!(events.len(), 0);
        assert_eq!(events.latest(), None);
    }

    #[test]
    fn a_single_candle_has_no_events() {
        let candles = candles_from(&[(100.5, 99.5, 100.0)]);
        let readings = Readings::compute(&candles);
        let events = LevelEventSeries::compute(&candles, &readings, &config());
        assert!(events.is_empty());
    }

    #[test]
    fn a_swing_level_is_not_used_before_it_becomes_available() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (101.5, 100.5, 101.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
            (105.5, 104.5, 105.0),
            (104.5, 103.5, 104.0),
            (103.5, 102.5, 103.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
        ]);
        let readings = readings_with(&candles, |_| None, |_| None, |_| None, |_| Some(20.0));
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        let swing_events: Vec<_> = events
            .events()
            .iter()
            .filter(|event| event.reference_code.is_some_and(|code| code >= 1001))
            .collect();
        assert!(
            !swing_events.is_empty(),
            "the fixture should produce a swing event"
        );
        for event in &swing_events {
            assert!(
                event.available_bar_index >= 7,
                "a swing level was used before its availability bar"
            );
        }
        let first = swing_events
            .iter()
            .find(|event| event.available_bar_index == 7)
            .expect("the minor high becomes available at bar 7");
        assert_eq!(first.source_code, MINOR_PIVOT_SOURCE_CODE);
        assert_eq!(first.reference_code, Some(1001));
    }

    #[test]
    fn a_major_swing_level_uses_the_major_source_code() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (101.5, 100.5, 101.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
            (105.5, 104.5, 105.0),
            (104.5, 103.5, 104.0),
            (103.5, 102.5, 103.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
        ]);
        let readings = readings_with(&candles, |_| None, |_| None, |_| None, |_| Some(20.0));
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        assert!(events.events().iter().any(|event| {
            event.available_bar_index == 10
                && event.source_code == MAJOR_PIVOT_SOURCE_CODE
                && event.reference_code == Some(1003)
        }));
    }

    #[test]
    fn the_reference_set_is_bounded_to_seven_per_bar() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (101.5, 100.5, 101.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
            (105.5, 104.5, 105.0),
            (104.5, 103.5, 104.0),
            (103.5, 102.5, 103.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
        ]);
        let readings = readings_with(
            &candles,
            |_| Some(100.0),
            |_| Some(100.0),
            |_| Some(100.0),
            |_| Some(20.0),
        );
        let events = LevelEventSeries::compute(&candles, &readings, &config());

        for bar in 0..candles.len() {
            assert!(events.events_at(bar).count() <= 7);
        }
    }

    #[test]
    fn later_candles_do_not_change_earlier_events() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (101.5, 100.5, 101.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
            (105.5, 104.5, 105.0),
            (104.5, 103.5, 104.0),
            (103.5, 102.5, 103.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
            (104.5, 103.5, 104.0),
        ]);
        let readings = readings_with(
            &candles,
            |_| Some(100.0),
            |_| Some(100.0),
            |_| Some(100.0),
            |_| Some(20.0),
        );
        let full = LevelEventSeries::compute(&candles, &readings, &config());

        for length in 0..candles.len() {
            let prefix = &candles[..length];
            let prefix_readings = readings_with(
                prefix,
                |_| Some(100.0),
                |_| Some(100.0),
                |_| Some(100.0),
                |_| Some(20.0),
            );
            let partial = LevelEventSeries::compute(prefix, &prefix_readings, &config());
            let expected: Vec<_> = full
                .events()
                .iter()
                .filter(|event| event.available_bar_index < length)
                .copied()
                .collect();
            assert_eq!(partial.events(), expected.as_slice());
        }
    }

    #[test]
    fn the_detector_is_deterministic() {
        let candles = candles_from(&[
            (100.5, 99.5, 100.0),
            (101.5, 100.5, 101.0),
            (102.5, 101.5, 102.0),
            (103.5, 102.5, 103.0),
        ]);
        let readings = readings_with(
            &candles,
            |_| Some(100.0),
            |_| Some(100.0),
            |_| Some(100.0),
            |_| Some(20.0),
        );
        let first = LevelEventSeries::compute(&candles, &readings, &config());
        let second = LevelEventSeries::compute(&candles, &readings, &config());
        assert_eq!(first, second);
    }

    #[test]
    fn the_proximity_fraction_is_the_contract_value() {
        assert_eq!(PROXIMITY_ATR_FRACTION, 0.2);
        assert_eq!(EMA50_REFERENCE_CODE, 2);
    }
}
