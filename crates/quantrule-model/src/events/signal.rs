//! Signal events.
//!
//! A Signal event records a discrete change in a reading: an RSI 14 threshold
//! crossing, a configured EMA/EMA crossover, or a change in a published compact
//! Direction, Momentum, or Structure state code. A price/EMA crossing is a Level
//! event and is never emitted here.
//!
//! The detector reads the readings and the three axis series. It never changes
//! an axis value; a signal is evidence, not a vote. The module is pure: it
//! performs no I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use crate::axes::direction::DirectionSeries;
use crate::axes::momentum::MomentumSeries;
use crate::axes::structure::StructureSeries;
use crate::events::{EventRecord, EventType};
use crate::readings::Readings;

/// The contract's source code for RSI 14.
pub const RSI_SOURCE_CODE: u16 = 4;
/// The contract's source code for EMA 20.
pub const EMA20_SOURCE_CODE: u16 = 1;
/// The contract's source code for EMA 50.
pub const EMA50_SOURCE_CODE: u16 = 2;
/// The contract's source code for EMA 200.
pub const EMA200_SOURCE_CODE: u16 = 3;
/// The contract's source code for EMA spread alignment (Direction).
pub const DIRECTION_SOURCE_CODE: u16 = 9;
/// The contract's source code for swing relations (Structure).
pub const STRUCTURE_SOURCE_CODE: u16 = 10;
/// The contract's source code for RSI force (Momentum).
pub const MOMENTUM_SOURCE_CODE: u16 = 11;

/// The contract's reference code for EMA 20.
pub const EMA20_REFERENCE_CODE: u32 = 1;
/// The contract's reference code for EMA 50.
pub const EMA50_REFERENCE_CODE: u32 = 2;
/// The contract's reference code for EMA 200.
pub const EMA200_REFERENCE_CODE: u32 = 3;

/// The RSI thresholds whose crossings are Signal events.
pub const RSI_THRESHOLDS: [f64; 3] = [30.0, 50.0, 70.0];

/// The configured EMA pairs, fastest first.
pub const EMA_PAIRS: [(usize, usize); 3] = [(20, 50), (50, 200), (20, 200)];

/// The Signal event's compact subtype codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalSubtype {
    /// Code 1: a reading crossed a defined threshold.
    ThresholdCrossing = 1,
    /// Code 2: one configured series crossed another.
    SeriesCrossover = 2,
    /// Code 3: a published compact state code changed.
    CompactStateChange = 3,
}

impl SignalSubtype {
    /// Every subtype, in contract code order.
    pub const ALL: [Self; 3] = [
        Self::ThresholdCrossing,
        Self::SeriesCrossover,
        Self::CompactStateChange,
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
            1 => Some(Self::ThresholdCrossing),
            2 => Some(Self::SeriesCrossover),
            3 => Some(Self::CompactStateChange),
            _ => None,
        }
    }
}

/// The configuration of a Signal-event run.
///
/// The timeframe is required because every event record carries its source
/// timeframe. It must be a positive number of seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalConfig {
    timeframe_secs: NonZeroU64,
}

impl SignalConfig {
    /// Builds a configuration from a non-zero timeframe.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self { timeframe_secs }
    }

    /// Builds a configuration from a timeframe in seconds.
    ///
    /// Returns `None` when the timeframe is zero.
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

/// The Signal events detected over one readings and axis series.
///
/// The events are stored in bar order. Inside one bar the fixed order is RSI
/// thresholds 30, 50, 70; EMA pairs 20/50, 50/200, 20/200; then Direction,
/// Momentum, and Structure state changes. Distinct signal conditions can
/// coincide on a bar; they are not duplicates because each names a different
/// defined relationship.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU64;
///
/// use quantrule_data::Candle;
/// use quantrule_model::axes::direction::DirectionSeries;
/// use quantrule_model::axes::momentum::MomentumSeries;
/// use quantrule_model::axes::structure::StructureSeries;
/// use quantrule_model::events::signal::{SignalConfig, SignalEventSeries};
/// use quantrule_model::readings::Readings;
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let candles = [candle];
/// let readings = Readings::compute(&candles);
/// let direction = DirectionSeries::compute(&readings);
/// let momentum = MomentumSeries::compute(&readings);
/// let structure = StructureSeries::compute(&candles, &readings);
/// let config = SignalConfig::new(NonZeroU64::new(300).expect("a non-zero timeframe"));
/// let events = SignalEventSeries::compute(
///     &readings,
///     &direction,
///     &momentum,
///     &structure,
///     &config,
/// );
///
/// assert!(events.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SignalEventSeries {
    events: Vec<EventRecord>,
}

impl SignalEventSeries {
    /// Computes every Signal event over the readings and axis series.
    ///
    /// The input is not modified. The series covers the shorter of the input
    /// lengths; the model's own series always have equal length.
    #[must_use]
    pub fn compute(
        readings: &Readings,
        direction: &DirectionSeries,
        momentum: &MomentumSeries,
        structure: &StructureSeries,
        config: &SignalConfig,
    ) -> Self {
        let len = readings
            .len()
            .min(direction.len())
            .min(momentum.len())
            .min(structure.len());
        let mut events = Vec::new();

        for bar in 1..len {
            threshold_events(readings, bar, config, &mut events);
            ema_events(readings, bar, config, &mut events);
            state_events(direction, momentum, structure, bar, config, &mut events);
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

    /// Every detected event, in bar and condition order.
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

fn threshold_events(
    readings: &Readings,
    bar: usize,
    config: &SignalConfig,
    events: &mut Vec<EventRecord>,
) {
    let (Some(previous), Some(current)) = (readings.rsi14.get(bar - 1), readings.rsi14.get(bar))
    else {
        return;
    };

    for threshold in RSI_THRESHOLDS {
        let direction = if previous < threshold && current > threshold {
            1
        } else if previous > threshold && current < threshold {
            -1
        } else {
            continue;
        };
        events.push(signal_event(
            SignalSubtype::ThresholdCrossing,
            RSI_SOURCE_CODE,
            None,
            Some(current),
            Some(threshold),
            direction,
            bar,
            config,
        ));
    }
}

fn ema_events(
    readings: &Readings,
    bar: usize,
    config: &SignalConfig,
    events: &mut Vec<EventRecord>,
) {
    for (fast, slow) in EMA_PAIRS {
        let (Some(previous_fast), Some(previous_slow), Some(current_fast), Some(current_slow)) = (
            ema_value(readings, fast, bar - 1),
            ema_value(readings, slow, bar - 1),
            ema_value(readings, fast, bar),
            ema_value(readings, slow, bar),
        ) else {
            continue;
        };

        let direction = if previous_fast <= previous_slow && current_fast > current_slow {
            1
        } else if previous_fast >= previous_slow && current_fast < current_slow {
            -1
        } else {
            continue;
        };
        events.push(signal_event(
            SignalSubtype::SeriesCrossover,
            ema_source(fast),
            Some(ema_reference(slow)),
            Some(current_fast),
            Some(current_slow),
            direction,
            bar,
            config,
        ));
    }
}

fn state_events(
    direction: &DirectionSeries,
    momentum: &MomentumSeries,
    structure: &StructureSeries,
    bar: usize,
    config: &SignalConfig,
    events: &mut Vec<EventRecord>,
) {
    if let Some((previous, current, direction_code)) = state_change(
        |index| direction.get(index).and_then(|bar| bar.state_code),
        bar,
        false,
    ) {
        events.push(state_event(
            DIRECTION_SOURCE_CODE,
            previous,
            current,
            direction_code,
            bar,
            config,
        ));
    }

    if let Some((previous, current, direction_code)) = state_change(
        |index| momentum.get(index).and_then(|bar| bar.state_code),
        bar,
        false,
    ) {
        events.push(state_event(
            MOMENTUM_SOURCE_CODE,
            previous,
            current,
            direction_code,
            bar,
            config,
        ));
    }

    if let Some((previous, current, direction_code)) = state_change(
        |index| structure.get(index).and_then(|bar| bar.state_code),
        bar,
        true,
    ) {
        events.push(state_event(
            STRUCTURE_SOURCE_CODE,
            previous,
            current,
            direction_code,
            bar,
            config,
        ));
    }
}

/// Returns the previous and current codes when both are present and differ.
///
/// The first code-bearing bar is not a change, and a transition to or from a
/// null code is not a published code change.
fn state_change(
    code: impl Fn(usize) -> Option<i8>,
    bar: usize,
    sign_only: bool,
) -> Option<(f64, f64, i8)> {
    let previous = code(bar - 1)?;
    let current = code(bar)?;
    if previous == current {
        return None;
    }
    let direction = if sign_only { current.signum() } else { current };
    Some((f64::from(previous), f64::from(current), direction))
}

fn state_event(
    source_code: u16,
    previous: f64,
    current: f64,
    direction: i8,
    bar: usize,
    config: &SignalConfig,
) -> EventRecord {
    signal_event(
        SignalSubtype::CompactStateChange,
        source_code,
        None,
        Some(current),
        Some(previous),
        direction,
        bar,
        config,
    )
}

#[allow(clippy::too_many_arguments)]
fn signal_event(
    subtype: SignalSubtype,
    source_code: u16,
    reference_code: Option<u32>,
    primary_value: Option<f64>,
    reference_value: Option<f64>,
    direction: i8,
    bar: usize,
    config: &SignalConfig,
) -> EventRecord {
    EventRecord {
        event_type: EventType::Signal,
        event_subtype_code: subtype.code(),
        source_code,
        timeframe_secs: config.timeframe_secs(),
        occurred_bar_index: bar,
        available_bar_index: bar,
        direction_code: direction,
        reference_code,
        primary_value,
        reference_value,
    }
}

fn ema_value(readings: &Readings, period: usize, bar: usize) -> Option<f64> {
    match period {
        20 => readings.ema20.get(bar),
        50 => readings.ema50.get(bar),
        200 => readings.ema200.get(bar),
        _ => None,
    }
}

fn ema_source(period: usize) -> u16 {
    match period {
        20 => EMA20_SOURCE_CODE,
        50 => EMA50_SOURCE_CODE,
        200 => EMA200_SOURCE_CODE,
        _ => 0,
    }
}

fn ema_reference(period: usize) -> u32 {
    match period {
        20 => EMA20_REFERENCE_CODE,
        50 => EMA50_REFERENCE_CODE,
        200 => EMA200_REFERENCE_CODE,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DIRECTION_SOURCE_CODE, DirectionSeries, EMA20_SOURCE_CODE, MOMENTUM_SOURCE_CODE,
        MomentumSeries, RSI_SOURCE_CODE, SignalConfig, SignalEventSeries, SignalSubtype,
        StructureSeries,
    };
    use crate::readings::{PivotSeries, ReadingSeries, Readings};
    use quantrule_data::Candle;
    use std::num::NonZeroU64;

    fn candle(index: u64, mid: f64) -> Candle {
        Candle::new(
            index,
            index as i64,
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
            .map(|(index, mid)| candle(index as u64, *mid))
            .collect()
    }

    fn series(values: Vec<Option<f64>>) -> ReadingSeries {
        ReadingSeries::new(values)
    }

    fn flat(len: usize, value: f64) -> Vec<Option<f64>> {
        vec![Some(value); len]
    }

    fn none(len: usize) -> Vec<Option<f64>> {
        vec![None; len]
    }

    #[allow(clippy::too_many_arguments)]
    fn readings_with(
        candles: &[Candle],
        ema20: Vec<Option<f64>>,
        ema50: Vec<Option<f64>>,
        ema200: Vec<Option<f64>>,
        rsi: Vec<Option<f64>>,
        atr: Vec<Option<f64>>,
    ) -> Readings {
        let len = candles.len();
        assert_eq!(ema20.len(), len);
        assert_eq!(ema50.len(), len);
        assert_eq!(ema200.len(), len);
        assert_eq!(rsi.len(), len);
        assert_eq!(atr.len(), len);
        Readings {
            ema20: series(ema20),
            ema50: series(ema50),
            ema200: series(ema200),
            rsi14: series(rsi),
            atr14: series(atr),
            rvol20: series(none(len)),
            pivots: PivotSeries::compute(candles),
        }
    }

    fn config() -> SignalConfig {
        SignalConfig::new(NonZeroU64::new(300).expect("a non-zero timeframe"))
    }

    fn signals(candles: &[Candle], readings: &Readings) -> SignalEventSeries {
        let direction = DirectionSeries::compute(readings);
        let momentum = MomentumSeries::compute(readings);
        let structure = StructureSeries::compute(candles, readings);
        SignalEventSeries::compute(readings, &direction, &momentum, &structure, &config())
    }

    fn events_of(
        series: &SignalEventSeries,
        subtype: SignalSubtype,
        source_code: u16,
    ) -> Vec<&crate::events::EventRecord> {
        series
            .events()
            .iter()
            .filter(|event| {
                event.event_subtype_code == subtype.code() && event.source_code == source_code
            })
            .collect()
    }

    #[test]
    fn an_rsi_upward_crossing_emits_a_threshold_event() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            none(2),
            none(2),
            none(2),
            vec![Some(29.0), Some(31.0)],
            none(2),
        );
        let series = signals(&candles, &readings);

        let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
        assert_eq!(events.len(), 1);
        let event = events[0];
        assert_eq!(event.source_code, RSI_SOURCE_CODE);
        assert_eq!(event.direction_code, 1);
        assert_eq!(event.reference_value, Some(30.0));
        assert_eq!(event.primary_value, Some(31.0));
        assert_eq!(event.reference_code, None);
        assert_eq!(event.occurred_bar_index, 1);
        assert_eq!(event.available_bar_index, 1);
    }

    #[test]
    fn an_rsi_downward_crossing_emits_a_threshold_event() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            none(2),
            none(2),
            none(2),
            vec![Some(31.0), Some(29.0)],
            none(2),
        );
        let series = signals(&candles, &readings);

        let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].direction_code, -1);
        assert_eq!(events[0].reference_value, Some(30.0));
    }

    #[test]
    fn a_close_exactly_on_a_threshold_is_not_a_crossing() {
        for values in [vec![Some(30.0), Some(31.0)], vec![Some(29.0), Some(30.0)]] {
            let candles = candles(&[100.0, 100.0]);
            let readings = readings_with(&candles, none(2), none(2), none(2), values, none(2));
            let series = signals(&candles, &readings);
            assert!(
                events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE).is_empty()
            );
        }
    }

    #[test]
    fn two_thresholds_can_cross_on_one_bar() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            none(2),
            none(2),
            none(2),
            vec![Some(29.0), Some(51.0)],
            none(2),
        );
        let series = signals(&candles, &readings);

        let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].reference_value, Some(30.0));
        assert_eq!(events[1].reference_value, Some(50.0));
    }

    #[test]
    fn an_ema_crossing_emits_a_series_event() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            vec![Some(99.0), Some(101.0)],
            flat(2, 100.0),
            flat(2, 90.0),
            none(2),
            none(2),
        );
        let series = signals(&candles, &readings);

        let events = events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE);
        assert_eq!(events.len(), 1);
        let event = events[0];
        assert_eq!(event.reference_code, Some(super::EMA50_REFERENCE_CODE));
        assert_eq!(event.primary_value, Some(101.0));
        assert_eq!(event.reference_value, Some(100.0));
        assert_eq!(event.direction_code, 1);
    }

    #[test]
    fn an_ema_downward_crossing_emits_a_series_event() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            vec![Some(101.0), Some(99.0)],
            flat(2, 100.0),
            flat(2, 90.0),
            none(2),
            none(2),
        );
        let series = signals(&candles, &readings);

        let events = events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].direction_code, -1);
    }

    #[test]
    fn a_cross_from_equality_is_emitted() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            vec![Some(100.0), Some(101.0)],
            flat(2, 100.0),
            flat(2, 90.0),
            none(2),
            none(2),
        );
        let series = signals(&candles, &readings);

        let events = events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].direction_code, 1);
    }

    #[test]
    fn current_equality_is_not_a_crossing() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            vec![Some(99.0), Some(100.0)],
            flat(2, 100.0),
            flat(2, 90.0),
            none(2),
            none(2),
        );
        let series = signals(&candles, &readings);

        assert!(events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE).is_empty());
    }

    #[test]
    fn all_three_ema_pairs_can_cross_on_one_bar() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            vec![Some(99.0), Some(102.0)],
            vec![Some(100.0), Some(101.0)],
            vec![Some(101.0), Some(100.0)],
            none(2),
            none(2),
        );
        let series = signals(&candles, &readings);

        let events: Vec<_> = series
            .events()
            .iter()
            .filter(|event| event.event_subtype_code == SignalSubtype::SeriesCrossover.code())
            .collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].source_code, EMA20_SOURCE_CODE);
        assert_eq!(events[1].source_code, super::EMA50_SOURCE_CODE);
        assert_eq!(events[2].source_code, EMA20_SOURCE_CODE);
    }

    #[test]
    fn a_price_ema_crossing_is_not_a_signal() {
        let candles = candles(&[99.0, 101.0]);
        let readings = readings_with(
            &candles,
            flat(2, 100.0),
            flat(2, 100.0),
            flat(2, 100.0),
            none(2),
            none(2),
        );
        let series = signals(&candles, &readings);

        assert!(events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE).is_empty());
        assert!(
            events_of(
                &series,
                SignalSubtype::SeriesCrossover,
                super::EMA50_SOURCE_CODE
            )
            .is_empty()
        );
        assert!(
            events_of(
                &series,
                SignalSubtype::SeriesCrossover,
                super::EMA200_SOURCE_CODE
            )
            .is_empty()
        );
    }

    #[test]
    fn a_direction_state_change_emits_on_the_change_bar() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            vec![Some(101.0), Some(110.0)],
            flat(2, 100.0),
            flat(2, 99.0),
            flat(2, 50.0),
            flat(2, 10.0),
        );
        let series = signals(&candles, &readings);

        let events = events_of(
            &series,
            SignalSubtype::CompactStateChange,
            DIRECTION_SOURCE_CODE,
        );
        assert_eq!(events.len(), 1);
        let event = events[0];
        assert_eq!(event.direction_code, 1);
        assert_eq!(event.primary_value, Some(1.0));
        assert_eq!(event.reference_value, Some(0.0));
        assert_eq!(event.available_bar_index, 1);
    }

    #[test]
    fn a_momentum_state_change_and_its_threshold_both_fire() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            none(2),
            none(2),
            none(2),
            vec![Some(44.0), Some(56.0)],
            none(2),
        );
        let series = signals(&candles, &readings);

        let threshold = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
        let state = events_of(
            &series,
            SignalSubtype::CompactStateChange,
            MOMENTUM_SOURCE_CODE,
        );
        assert_eq!(threshold.len(), 1);
        assert_eq!(state.len(), 1);
        assert_eq!(threshold[0].available_bar_index, 1);
        assert_eq!(state[0].available_bar_index, 1);
        assert_eq!(state[0].direction_code, 1);
    }

    #[test]
    fn the_first_code_is_not_a_state_change() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            none(2),
            none(2),
            none(2),
            vec![None, Some(60.0)],
            none(2),
        );
        let series = signals(&candles, &readings);

        assert!(
            events_of(
                &series,
                SignalSubtype::CompactStateChange,
                MOMENTUM_SOURCE_CODE
            )
            .is_empty()
        );
    }

    #[test]
    fn a_transition_to_null_is_not_a_state_change() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            none(2),
            none(2),
            none(2),
            vec![Some(60.0), None],
            none(2),
        );
        let series = signals(&candles, &readings);

        assert!(
            events_of(
                &series,
                SignalSubtype::CompactStateChange,
                MOMENTUM_SOURCE_CODE
            )
            .is_empty()
        );
    }

    #[test]
    fn an_empty_series_has_no_events() {
        let candles = Vec::new();
        let readings = readings_with(&candles, vec![], vec![], vec![], vec![], vec![]);
        let series = signals(&candles, &readings);
        assert!(series.is_empty());
    }

    #[test]
    fn a_single_candle_has_no_events() {
        let candles = candles(&[100.0]);
        let readings = Readings::compute(&candles);
        let series = signals(&candles, &readings);
        assert!(series.is_empty());
    }

    #[test]
    fn later_candles_do_not_change_earlier_signals() {
        let candles = candles(&[100.0, 101.0, 102.0, 103.0, 104.0]);
        let ema20 = vec![
            Some(99.0),
            Some(101.0),
            Some(102.0),
            Some(103.0),
            Some(104.0),
        ];
        let ema50 = flat(5, 100.0);
        let ema200 = flat(5, 90.0);
        let rsi = vec![Some(29.0), Some(31.0), Some(51.0), Some(49.0), Some(71.0)];
        let atr = flat(5, 10.0);
        let full_readings = readings_with(
            &candles,
            ema20.clone(),
            ema50.clone(),
            ema200.clone(),
            rsi.clone(),
            atr.clone(),
        );
        let full = signals(&candles, &full_readings);

        for length in 0..=candles.len() {
            let prefix_candles = &candles[..length];
            let prefix_readings = readings_with(
                prefix_candles,
                ema20[..length].to_vec(),
                ema50[..length].to_vec(),
                ema200[..length].to_vec(),
                rsi[..length].to_vec(),
                atr[..length].to_vec(),
            );
            let partial = signals(prefix_candles, &prefix_readings);
            let expected: Vec<_> = full
                .events()
                .iter()
                .filter(|event| event.available_bar_index < length)
                .copied()
                .collect();
            assert_eq!(
                partial.events(),
                expected.as_slice(),
                "the prefix of length {length} changed an earlier signal"
            );
        }
    }

    #[test]
    fn the_detector_is_deterministic() {
        let candles = candles(&[100.0, 101.0, 102.0]);
        let readings = readings_with(
            &candles,
            vec![Some(99.0), Some(101.0), Some(102.0)],
            flat(3, 100.0),
            flat(3, 90.0),
            vec![Some(29.0), Some(31.0), Some(51.0)],
            flat(3, 10.0),
        );
        let first = signals(&candles, &readings);
        let second = signals(&candles, &readings);
        assert_eq!(first, second);
    }

    #[test]
    fn the_detector_changes_no_axis_value() {
        let candles = candles(&[100.0, 101.0, 102.0]);
        let readings = readings_with(
            &candles,
            vec![Some(99.0), Some(101.0), Some(102.0)],
            flat(3, 100.0),
            flat(3, 90.0),
            vec![Some(29.0), Some(31.0), Some(51.0)],
            flat(3, 10.0),
        );
        let direction_before = DirectionSeries::compute(&readings);
        let momentum_before = MomentumSeries::compute(&readings);
        let structure_before = StructureSeries::compute(&candles, &readings);

        let _ = SignalEventSeries::compute(
            &readings,
            &direction_before,
            &momentum_before,
            &structure_before,
            &config(),
        );

        assert_eq!(direction_before, DirectionSeries::compute(&readings));
        assert_eq!(momentum_before, MomentumSeries::compute(&readings));
        assert_eq!(
            structure_before,
            StructureSeries::compute(&candles, &readings)
        );
    }

    #[test]
    fn the_signal_codes_are_the_contract_codes() {
        assert_eq!(SignalSubtype::ThresholdCrossing.code(), 1);
        assert_eq!(SignalSubtype::SeriesCrossover.code(), 2);
        assert_eq!(SignalSubtype::CompactStateChange.code(), 3);
        assert_eq!(super::RSI_THRESHOLDS, [30.0, 50.0, 70.0]);
        assert_eq!(super::EMA_PAIRS.len(), 3);
    }
}
