//! Divergence events.
//!
//! A Divergence event records a disagreement between consecutive confirmed
//! price pivots of the same kind and the RSI 14 values around them. Around a
//! price-low pivot the detector takes the minimum RSI within two bars either
//! side; around a price-high pivot it takes the maximum. The comparison is made
//! only when the newer pivot is confirmed, and the event is appended once.
//!
//! The detector reports a structural relationship with RSI as its source. It
//! never changes an axis value and is not a second RSI vote. The module is pure:
//! it reads only the readings and the configuration it is given.

use std::num::NonZeroU64;

use crate::axes::structure::registry::{ConfirmedLevel, ConfirmedLevelRegistry};
use crate::events::{EventRecord, EventType};
use crate::readings::{PivotKind, PivotScale, ReadingSeries, Readings};

/// The contract's source code for RSI 14.
pub const RSI_SOURCE_CODE: u16 = 4;

/// The number of bars on either side of a pivot used for its RSI window.
pub const RSI_WINDOW_BARS: usize = 2;

/// The Divergence event's compact subtype codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceSubtype {
    /// Code 1: regular bullish — a lower low with a higher RSI low.
    RegularBullish = 1,
    /// Code 2: regular bearish — a higher high with a lower RSI high.
    RegularBearish = 2,
    /// Code 3: hidden bullish — a higher low with a lower RSI low.
    HiddenBullish = 3,
    /// Code 4: hidden bearish — a lower high with a higher RSI high.
    HiddenBearish = 4,
}

impl DivergenceSubtype {
    /// Every subtype, in contract code order.
    pub const ALL: [Self; 4] = [
        Self::RegularBullish,
        Self::RegularBearish,
        Self::HiddenBullish,
        Self::HiddenBearish,
    ];

    /// The contract's numeric code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The subtype for a contract code, or `None` when the code is unknown.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::RegularBullish),
            2 => Some(Self::RegularBearish),
            3 => Some(Self::HiddenBullish),
            4 => Some(Self::HiddenBearish),
            _ => None,
        }
    }
}

/// The configuration of a Divergence-event run.
///
/// The scale selects which confirmed pivot sequence is compared. The default
/// used by callers is the major scale, the structural sequence; a run may
/// request the minor scale instead. The timeframe is required because every
/// event record carries its source timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivergenceConfig {
    timeframe_secs: NonZeroU64,
    scale: PivotScale,
}

impl DivergenceConfig {
    /// Builds a configuration from a non-zero timeframe and a pivot scale.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64, scale: PivotScale) -> Self {
        Self {
            timeframe_secs,
            scale,
        }
    }

    /// Builds a configuration from a timeframe in seconds and a pivot scale.
    ///
    /// Returns `None` when the timeframe is zero.
    #[must_use]
    pub fn from_secs(timeframe_secs: u64, scale: PivotScale) -> Option<Self> {
        NonZeroU64::new(timeframe_secs).map(|timeframe_secs| Self::new(timeframe_secs, scale))
    }

    /// The configured timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(self) -> u64 {
        self.timeframe_secs.get()
    }

    /// The configured pivot scale.
    #[must_use]
    pub const fn scale(self) -> PivotScale {
        self.scale
    }
}

/// The Divergence events detected over one readings series.
///
/// The events are stored in availability-bar order. A divergence is emitted
/// only when the newer pivot is confirmed; `occurred_bar_index` is the newer
/// pivot's occurrence bar and `available_bar_index` is its confirmation bar.
/// The sample is a batch result over the input.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU64;
///
/// use quantrule_data::Candle;
/// use quantrule_model::events::divergence::{DivergenceConfig, DivergenceEventSeries};
/// use quantrule_model::readings::{PivotScale, Readings};
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let readings = Readings::compute(&[candle]);
/// let config = DivergenceConfig::new(
///     NonZeroU64::new(300).expect("a non-zero timeframe"),
///     PivotScale::Major,
/// );
/// let events = DivergenceEventSeries::compute(&readings, &config);
///
/// assert!(events.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DivergenceEventSeries {
    events: Vec<EventRecord>,
}

impl DivergenceEventSeries {
    /// Computes every Divergence event over the readings.
    ///
    /// The input is not modified. The scale comes from the configuration; the
    /// other scale is not compared, so a minor counter-move is never blended
    /// into the major sequence.
    #[must_use]
    pub fn compute(readings: &Readings, config: &DivergenceConfig) -> Self {
        let scale = config.scale();
        let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
        let levels: Vec<ConfirmedLevel> = registry
            .levels()
            .iter()
            .copied()
            .filter(|level| level.scale == scale)
            .collect();
        let pivots: Vec<_> = readings.pivots.all_confirmed(scale).copied().collect();
        let mut events = Vec::new();
        let mut previous_high: Option<PivotState> = None;
        let mut previous_low: Option<PivotState> = None;

        for (pivot, level) in pivots.iter().zip(levels.iter()) {
            let state = PivotState {
                price: pivot.price,
                occurred_at: pivot.occurred_at,
                available_at: pivot.available_at,
                reference_code: level.reference_code,
                rsi: rsi_window_value(&readings.rsi14, pivot.occurred_at, pivot.kind),
            };

            let previous = match pivot.kind {
                PivotKind::High => previous_high.replace(state),
                PivotKind::Low => previous_low.replace(state),
            };
            let Some(previous) = previous else {
                continue;
            };
            if let Some(event) = compare(previous, state, pivot.kind, readings, config) {
                events.push(event);
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

    /// Every detected event, in availability-bar order.
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

/// The state kept for the previous confirmed pivot of one kind.
#[derive(Debug, Clone, Copy)]
struct PivotState {
    price: f64,
    occurred_at: usize,
    available_at: usize,
    reference_code: u32,
    rsi: Option<f64>,
}

/// The minimum or maximum RSI within two bars either side of a pivot.
///
/// A price-low pivot uses the minimum and a price-high pivot uses the maximum.
/// If any bar in the window has no RSI value, the summary is `None`: the
/// detector never infers a divergence from an incomplete window.
fn rsi_window_value(series: &ReadingSeries, occurred_at: usize, kind: PivotKind) -> Option<f64> {
    let last = series.len().checked_sub(1)?;
    let start = occurred_at.saturating_sub(RSI_WINDOW_BARS);
    let end = occurred_at.saturating_add(RSI_WINDOW_BARS).min(last);
    let mut value: Option<f64> = None;

    for bar in start..=end {
        let reading = series.get(bar)?;
        value = Some(match value {
            None => reading,
            Some(current) => match kind {
                PivotKind::High => current.max(reading),
                PivotKind::Low => current.min(reading),
            },
        });
    }

    value
}

/// Applies the contract's four-subtype matrix to consecutive pivots.
fn compare(
    previous: PivotState,
    current: PivotState,
    kind: PivotKind,
    readings: &Readings,
    config: &DivergenceConfig,
) -> Option<EventRecord> {
    let previous_rsi = previous.rsi?;
    let current_rsi = current.rsi?;
    let price_delta = current.price - previous.price;
    let rsi_delta = current_rsi - previous_rsi;

    let (subtype, direction) = match kind {
        PivotKind::Low if price_delta < 0.0 && rsi_delta > 0.0 => {
            (DivergenceSubtype::RegularBullish, 1)
        }
        PivotKind::Low if price_delta > 0.0 && rsi_delta < 0.0 => {
            (DivergenceSubtype::HiddenBullish, 1)
        }
        PivotKind::High if price_delta > 0.0 && rsi_delta < 0.0 => {
            (DivergenceSubtype::RegularBearish, -1)
        }
        PivotKind::High if price_delta < 0.0 && rsi_delta > 0.0 => {
            (DivergenceSubtype::HiddenBearish, -1)
        }
        _ => return None,
    };

    let price_delta_atr = readings
        .atr14
        .get(current.occurred_at)
        .filter(|atr| *atr != 0.0 && atr.is_finite())
        .map(|atr| price_delta / atr);

    Some(EventRecord {
        event_type: EventType::Divergence,
        event_subtype_code: subtype.code(),
        source_code: RSI_SOURCE_CODE,
        timeframe_secs: config.timeframe_secs(),
        occurred_bar_index: current.occurred_at,
        available_bar_index: current.available_at,
        direction_code: direction,
        reference_code: Some(current.reference_code),
        primary_value: price_delta_atr,
        reference_value: Some(rsi_delta),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DivergenceConfig, DivergenceEventSeries, DivergenceSubtype, RSI_SOURCE_CODE,
        RSI_WINDOW_BARS,
    };
    use crate::axes::direction::DirectionSeries;
    use crate::axes::momentum::MomentumSeries;
    use crate::axes::structure::StructureSeries;
    use crate::axes::structure::registry::ConfirmedLevelRegistry;
    use crate::readings::{PivotKind, PivotScale, PivotSeries, ReadingSeries, Readings};
    use quantrule_data::Candle;
    use std::num::NonZeroU64;

    fn candle(series_index: u64, timestamp: i64, mid: f64) -> Candle {
        Candle::new(
            series_index,
            timestamp,
            mid,
            mid + 0.5,
            mid - 0.5,
            mid,
            Some(10.0),
        )
        .expect("the test candle is valid")
    }

    fn candles(mids: &[f64]) -> Vec<Candle> {
        mids.iter()
            .enumerate()
            .map(|(index, mid)| candle(index as u64, index as i64, *mid))
            .collect()
    }

    fn series(len: usize, value: Option<f64>) -> ReadingSeries {
        ReadingSeries::new(vec![value; len])
    }

    fn readings(candles: &[Candle], rsi: Vec<Option<f64>>, atr: Vec<Option<f64>>) -> Readings {
        let len = candles.len();
        Readings {
            ema20: series(len, Some(100.0)),
            ema50: series(len, Some(100.0)),
            ema200: series(len, Some(100.0)),
            rsi14: ReadingSeries::new(rsi),
            atr14: ReadingSeries::new(atr),
            rvol20: series(len, None),
            pivots: PivotSeries::compute(candles),
        }
    }

    fn atr_values(len: usize, bar: usize, value: f64) -> Vec<Option<f64>> {
        let mut values = vec![None; len];
        if bar < len {
            values[bar] = Some(value);
        }
        values
    }

    fn config() -> DivergenceConfig {
        DivergenceConfig::new(
            NonZeroU64::new(300).expect("a non-zero timeframe"),
            PivotScale::Minor,
        )
    }

    fn reference_code(
        readings: &Readings,
        scale: PivotScale,
        kind: PivotKind,
        occurred_at: usize,
    ) -> u32 {
        ConfirmedLevelRegistry::from_pivots(&readings.pivots)
            .levels()
            .iter()
            .find(|level| {
                level.scale == scale && level.kind == kind && level.occurred_at == occurred_at
            })
            .expect("the confirmed level exists")
            .reference_code
    }

    fn regular_bullish_series() -> Vec<f64> {
        vec![
            100.0, 99.0, 98.0, 99.0, 100.0, 101.0, 100.0, 99.0, 97.0, 98.0, 99.0, 100.0,
        ]
    }

    fn regular_bullish_rsi() -> Vec<Option<f64>> {
        vec![
            Some(40.0),
            Some(35.0),
            Some(20.0),
            Some(25.0),
            Some(30.0),
            Some(50.0),
            Some(45.0),
            Some(40.0),
            Some(30.0),
            Some(35.0),
            Some(40.0),
            Some(50.0),
        ]
    }

    #[test]
    fn a_lower_low_with_a_higher_rsi_low_is_regular_bullish() {
        let candles = candles(&regular_bullish_series());
        let readings = readings(
            &candles,
            regular_bullish_rsi(),
            atr_values(candles.len(), 8, 2.0),
        );
        let events = DivergenceEventSeries::compute(&readings, &config());

        assert_eq!(events.len(), 1);
        let event = events.latest().expect("one event");
        assert_eq!(event.event_type.code(), 2);
        assert_eq!(
            event.event_subtype_code,
            DivergenceSubtype::RegularBullish.code()
        );
        assert_eq!(event.source_code, RSI_SOURCE_CODE);
        assert_eq!(event.direction_code, 1);
        assert_eq!(event.timeframe_secs, 300);
        assert_eq!(event.occurred_bar_index, 8);
        assert_eq!(event.available_bar_index, 10);
        assert_eq!(
            event.reference_code,
            Some(reference_code(
                &readings,
                PivotScale::Minor,
                PivotKind::Low,
                8
            ))
        );
        assert!((event.primary_value.expect("price delta") - (-0.5)).abs() <= 1e-9);
        assert!((event.reference_value.expect("rsi delta") - 10.0).abs() <= 1e-9);
    }

    #[test]
    fn a_higher_low_with_a_lower_rsi_low_is_hidden_bullish() {
        let candles = candles(&[
            100.0, 99.0, 98.0, 99.0, 100.0, 101.0, 100.0, 99.0, 98.5, 99.0, 100.0, 101.0,
        ]);
        let rsi = vec![
            Some(50.0),
            Some(45.0),
            Some(30.0),
            Some(35.0),
            Some(40.0),
            Some(50.0),
            Some(45.0),
            Some(40.0),
            Some(20.0),
            Some(25.0),
            Some(30.0),
            Some(50.0),
        ];
        let readings = readings(&candles, rsi, atr_values(candles.len(), 8, 2.0));
        let events = DivergenceEventSeries::compute(&readings, &config());

        let event = events.latest().expect("one event");
        assert_eq!(
            event.event_subtype_code,
            DivergenceSubtype::HiddenBullish.code()
        );
        assert_eq!(event.direction_code, 1);
    }

    #[test]
    fn a_higher_high_with_a_lower_rsi_high_is_regular_bearish() {
        let candles = candles(&[
            100.0, 101.0, 102.0, 101.0, 100.0, 99.0, 100.0, 101.0, 103.0, 102.0, 101.0, 100.0,
        ]);
        let rsi = vec![
            Some(60.0),
            Some(65.0),
            Some(70.0),
            Some(65.0),
            Some(60.0),
            Some(50.0),
            Some(55.0),
            Some(60.0),
            Some(60.0),
            Some(55.0),
            Some(50.0),
            Some(40.0),
        ];
        let readings = readings(&candles, rsi, atr_values(candles.len(), 8, 2.0));
        let events = DivergenceEventSeries::compute(&readings, &config());

        let event = events.latest().expect("one event");
        assert_eq!(
            event.event_subtype_code,
            DivergenceSubtype::RegularBearish.code()
        );
        assert_eq!(event.direction_code, -1);
        assert_eq!(event.occurred_bar_index, 8);
        assert_eq!(event.available_bar_index, 10);
    }

    #[test]
    fn a_lower_high_with_a_higher_rsi_high_is_hidden_bearish() {
        let candles = candles(&[
            100.0, 101.0, 102.0, 101.0, 100.0, 99.0, 100.0, 101.0, 101.5, 101.0, 100.0, 99.0,
        ]);
        let rsi = vec![
            Some(50.0),
            Some(55.0),
            Some(60.0),
            Some(55.0),
            Some(50.0),
            Some(50.0),
            Some(55.0),
            Some(60.0),
            Some(80.0),
            Some(75.0),
            Some(70.0),
            Some(60.0),
        ];
        let readings = readings(&candles, rsi, atr_values(candles.len(), 8, 2.0));
        let events = DivergenceEventSeries::compute(&readings, &config());

        let event = events.latest().expect("one event");
        assert_eq!(
            event.event_subtype_code,
            DivergenceSubtype::HiddenBearish.code()
        );
        assert_eq!(event.direction_code, -1);
    }

    #[test]
    fn a_missing_rsi_value_in_a_window_produces_no_event() {
        let candles = candles(&regular_bullish_series());
        let mut rsi = vec![Some(30.0); candles.len()];
        rsi[2] = None;
        let readings = readings(&candles, rsi, atr_values(candles.len(), 8, 2.0));
        let events = DivergenceEventSeries::compute(&readings, &config());

        assert!(events.is_empty());
    }

    #[test]
    fn a_single_pivot_produces_no_event() {
        let candles = candles(&[100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0]);
        let rsi = vec![Some(50.0); candles.len()];
        let readings = readings(&candles, rsi, atr_values(candles.len(), 3, 2.0));
        let events = DivergenceEventSeries::compute(&readings, &config());

        assert!(events.is_empty());
    }

    #[test]
    fn a_higher_low_with_a_higher_rsi_low_is_not_a_divergence() {
        let candles = candles(&[
            100.0, 99.0, 98.0, 99.0, 100.0, 101.0, 100.0, 99.0, 98.5, 99.0, 100.0, 101.0,
        ]);
        let rsi = vec![
            Some(30.0),
            Some(35.0),
            Some(40.0),
            Some(45.0),
            Some(50.0),
            Some(50.0),
            Some(45.0),
            Some(40.0),
            Some(60.0),
            Some(65.0),
            Some(70.0),
            Some(50.0),
        ];
        let readings = readings(&candles, rsi, atr_values(candles.len(), 8, 2.0));
        let events = DivergenceEventSeries::compute(&readings, &config());

        assert!(events.is_empty());
    }

    #[test]
    fn equal_rsi_values_are_not_a_divergence() {
        let candles = candles(&regular_bullish_series());
        let rsi = vec![Some(30.0); candles.len()];
        let readings = readings(&candles, rsi, atr_values(candles.len(), 8, 2.0));
        let events = DivergenceEventSeries::compute(&readings, &config());

        assert!(events.is_empty());
    }

    #[test]
    fn an_unavailable_atr_leaves_the_price_delta_null_but_keeps_the_event() {
        let candles = candles(&regular_bullish_series());
        let readings = readings(&candles, regular_bullish_rsi(), vec![None; candles.len()]);
        let events = DivergenceEventSeries::compute(&readings, &config());

        let event = events.latest().expect("one event");
        assert_eq!(
            event.event_subtype_code,
            DivergenceSubtype::RegularBullish.code()
        );
        assert_eq!(event.primary_value, None);
        assert_eq!(event.reference_value, Some(10.0));
    }

    #[test]
    fn a_zero_atr_leaves_the_price_delta_null() {
        let candles = candles(&regular_bullish_series());
        let readings = readings(
            &candles,
            regular_bullish_rsi(),
            atr_values(candles.len(), 8, 0.0),
        );
        let events = DivergenceEventSeries::compute(&readings, &config());

        let event = events.latest().expect("one event");
        assert_eq!(event.primary_value, None);
    }

    #[test]
    fn an_event_is_not_emitted_before_the_newer_pivot_is_confirmed() {
        let candles = candles(&regular_bullish_series());
        let readings = readings(
            &candles,
            regular_bullish_rsi(),
            atr_values(candles.len(), 8, 2.0),
        );
        let events = DivergenceEventSeries::compute(&readings, &config());

        assert_eq!(events.events_at(9).count(), 0);
        assert_eq!(events.events_at(10).count(), 1);
        assert!(
            events
                .events()
                .iter()
                .all(|event| event.available_bar_index >= 10)
        );
    }

    #[test]
    fn later_candles_do_not_change_earlier_events() {
        let candles = candles(&regular_bullish_series());
        let rsi = vec![Some(30.0); candles.len()];
        let atr = atr_values(candles.len(), 8, 2.0);
        let full_readings = readings(&candles, rsi.clone(), atr.clone());
        let full = DivergenceEventSeries::compute(&full_readings, &config());

        for length in 0..=candles.len() {
            let prefix = &candles[..length];
            let prefix_readings = readings(prefix, rsi[..length].to_vec(), atr[..length].to_vec());
            let partial = DivergenceEventSeries::compute(&prefix_readings, &config());
            let expected: Vec<_> = full
                .events()
                .iter()
                .filter(|event| event.available_bar_index < length)
                .copied()
                .collect();
            assert_eq!(
                partial.events(),
                expected.as_slice(),
                "the prefix of length {length} changed an earlier event"
            );
        }
    }

    #[test]
    fn the_detector_is_deterministic() {
        let candles = candles(&regular_bullish_series());
        let rsi = vec![Some(30.0); candles.len()];
        let readings = readings(&candles, rsi, atr_values(candles.len(), 8, 2.0));
        let first = DivergenceEventSeries::compute(&readings, &config());
        let second = DivergenceEventSeries::compute(&readings, &config());
        assert_eq!(first, second);
    }

    #[test]
    fn the_detector_changes_no_axis_value() {
        let candles = candles(&regular_bullish_series());
        let readings = readings(
            &candles,
            regular_bullish_rsi(),
            atr_values(candles.len(), 8, 2.0),
        );
        let direction_before = DirectionSeries::compute(&readings);
        let momentum_before = MomentumSeries::compute(&readings);
        let structure_before = StructureSeries::compute(&candles, &readings);

        let events = DivergenceEventSeries::compute(&readings, &config());
        assert!(!events.is_empty(), "the fixture must produce an event");

        assert_eq!(direction_before, DirectionSeries::compute(&readings));
        assert_eq!(momentum_before, MomentumSeries::compute(&readings));
        assert_eq!(
            structure_before,
            StructureSeries::compute(&candles, &readings)
        );
    }

    #[test]
    fn the_rsi_window_is_two_bars_either_side() {
        assert_eq!(RSI_WINDOW_BARS, 2);
        assert_eq!(DivergenceSubtype::ALL.len(), 4);
    }
}
