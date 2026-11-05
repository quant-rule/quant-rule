//! Signal events.
//!
//! A Signal event records a discrete change in a reading: an RSI 14 threshold
//! crossing, a configured EMA/EMA crossover, a MACD state change, or a
//! compression/trend condition. A price/EMA crossing is a Level event and is
//! never emitted here.
//!
//! The detector reads the readings and the MACD series. It never changes a
//! reading; a signal is evidence, not a vote. The module is pure: it performs no
//! I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use crate::events::{EventRecord, EventType};
use crate::input::SignalSettings;
use crate::readings::{MacdSeries, Readings};

/// The contract's source code for RSI 14.
pub const RSI_SOURCE_CODE: u16 = 4;
/// The contract's source code for the MACD line.
pub const MACD_LINE_SOURCE_CODE: u16 = 13;
/// The contract's source code for the MACD signal.
pub const MACD_SIGNAL_SOURCE_CODE: u16 = 14;
/// The contract's source code for the MACD histogram.
pub const MACD_HISTOGRAM_SOURCE_CODE: u16 = 15;
/// The contract's source code for EMA 20.
pub const EMA20_SOURCE_CODE: u16 = 1;
/// The contract's source code for EMA 50.
pub const EMA50_SOURCE_CODE: u16 = 2;
/// The contract's source code for EMA 200.
pub const EMA200_SOURCE_CODE: u16 = 3;

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
    /// Code 4: the MACD histogram's slope changed sign.
    HistogramSlopeFlip = 4,
    /// Code 5: a series crossed zero.
    ZeroCross = 5,
    /// Code 6: a registered trend-continuation conjunction fired.
    TrendContinuation = 6,
    /// Code 7: a compression range broke with participation.
    CompressionBreakout = 7,
    /// Code 8: a coiling window completed and published its range.
    CompressionRange = 8,
    /// Code 9: the first coiling window after a non-coiling bar.
    CompressionRangeOnset = 9,
}

impl SignalSubtype {
    /// Every subtype, in contract code order.
    pub const ALL: [Self; 9] = [
        Self::ThresholdCrossing,
        Self::SeriesCrossover,
        Self::CompactStateChange,
        Self::HistogramSlopeFlip,
        Self::ZeroCross,
        Self::TrendContinuation,
        Self::CompressionBreakout,
        Self::CompressionRange,
        Self::CompressionRangeOnset,
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
            4 => Some(Self::HistogramSlopeFlip),
            5 => Some(Self::ZeroCross),
            6 => Some(Self::TrendContinuation),
            7 => Some(Self::CompressionBreakout),
            8 => Some(Self::CompressionRange),
            9 => Some(Self::CompressionRangeOnset),
            _ => None,
        }
    }
}

/// The configuration of a Signal-event run.
///
/// The timeframe is required because every event record carries its source
/// timeframe. It must be a positive number of seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalConfig {
    timeframe_secs: NonZeroU64,
    settings: SignalSettings,
}

impl SignalConfig {
    /// Builds a configuration from a non-zero timeframe.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self {
            timeframe_secs,
            settings: SignalSettings::DEFAULT,
        }
    }

    /// Builds a configuration with explicit signal settings.
    #[must_use]
    pub const fn with_settings(timeframe_secs: NonZeroU64, settings: SignalSettings) -> Self {
        Self {
            timeframe_secs,
            settings,
        }
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

    /// The signal settings.
    #[must_use]
    pub const fn settings(self) -> SignalSettings {
        self.settings
    }
}

/// The Signal events detected over one readings series and its MACD series.
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
/// use quantrule_model::events::signal::{SignalConfig, SignalEventSeries};
/// use quantrule_model::readings::{MacdSeries, Readings};
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let candles = [candle];
/// let readings = Readings::compute(&candles);
/// let macd = MacdSeries::compute(&candles, 12, 26, 9);
/// let config = SignalConfig::new(NonZeroU64::new(300).expect("a non-zero timeframe"));
/// let events = SignalEventSeries::compute(&readings, &macd, &config);
///
/// assert!(events.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SignalEventSeries {
    events: Vec<EventRecord>,
}

impl SignalEventSeries {
    /// Computes every Signal event over the readings and MACD series.
    ///
    /// The input is not modified. The series covers the shorter of the input
    /// lengths; the model's own series always have equal length.
    #[must_use]
    pub fn compute(readings: &Readings, macd: &MacdSeries, config: &SignalConfig) -> Self {
        let len = readings.len().min(macd.len());
        let mut events = Vec::new();

        for bar in 1..len {
            threshold_events(readings, bar, config, &mut events);
            ema_events(readings, bar, config, &mut events);
            ema_stack_change(readings, bar, config, &mut events);
            rsi_zone_change(readings, bar, config, &mut events);
            macd_events(macd, bar, config, &mut events);
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

    for threshold in config.settings().rsi_levels {
        let direction = if previous < threshold && current > threshold {
            1
        } else if previous > threshold && current < threshold {
            -1
        } else {
            continue;
        };
        events.push(signal_event(
            crate::events::codes::SIGNAL_RSI_THRESHOLD,
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
    for (fast, slow) in config.settings().ema_pairs {
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
            crate::events::codes::SIGNAL_EMA_PAIR_CROSS,
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

fn macd_events(
    macd: &MacdSeries,
    bar: usize,
    config: &SignalConfig,
    events: &mut Vec<EventRecord>,
) {
    let (Some(previous_line), Some(current_line)) = (macd.line.get(bar - 1), macd.line.get(bar))
    else {
        return;
    };
    let (Some(previous_signal), Some(current_signal)) =
        (macd.signal.get(bar - 1), macd.signal.get(bar))
    else {
        return;
    };
    let cross = if previous_line <= previous_signal && current_line > current_signal {
        1
    } else if previous_line >= previous_signal && current_line < current_signal {
        -1
    } else {
        0
    };
    if cross != 0 && config.settings().macd_line_signal_cross {
        events.push(signal_event(
            crate::events::codes::SIGNAL_MACD_LINE_SIGNAL_CROSS,
            SignalSubtype::SeriesCrossover,
            MACD_LINE_SOURCE_CODE,
            Some(u32::from(MACD_SIGNAL_SOURCE_CODE)),
            Some(current_line),
            Some(current_signal),
            cross,
            bar,
            config,
        ));
    }
    let zero_cross = if previous_line <= 0.0 && current_line > 0.0 {
        1
    } else if previous_line >= 0.0 && current_line < 0.0 {
        -1
    } else {
        0
    };
    if zero_cross != 0 && config.settings().macd_zero_cross {
        events.push(signal_event(
            crate::events::codes::SIGNAL_MACD_ZERO_CROSS,
            SignalSubtype::ZeroCross,
            MACD_LINE_SOURCE_CODE,
            None,
            Some(current_line),
            Some(0.0),
            zero_cross,
            bar,
            config,
        ));
    }

    let (Some(two_back), Some(previous), Some(current)) = (
        macd.histogram.get(bar - 2),
        macd.histogram.get(bar - 1),
        macd.histogram.get(bar),
    ) else {
        return;
    };
    let previous_slope = previous - two_back;
    let current_slope = current - previous;
    if previous_slope == 0.0 || current_slope == 0.0 {
        return;
    }
    if previous_slope.signum() != current_slope.signum()
        && config.settings().macd_histogram_slope_flip
    {
        events.push(signal_event(
            crate::events::codes::SIGNAL_MACD_HISTOGRAM_SLOPE_FLIP,
            SignalSubtype::HistogramSlopeFlip,
            MACD_HISTOGRAM_SOURCE_CODE,
            None,
            Some(current_slope),
            Some(previous_slope),
            if current_slope > 0.0 { 1 } else { -1 },
            bar,
            config,
        ));
    }
}

fn ema_stack_state(readings: &Readings, bar: usize) -> Option<i8> {
    let fast = readings.ema20.get(bar)?;
    let mid = readings.ema50.get(bar)?;
    let slow = readings.ema200.get(bar)?;
    Some(if fast > mid && mid > slow {
        1
    } else if fast < mid && mid < slow {
        -1
    } else {
        0
    })
}

fn ema_stack_change(
    readings: &Readings,
    bar: usize,
    config: &SignalConfig,
    events: &mut Vec<EventRecord>,
) {
    let (Some(previous), Some(current)) = (
        ema_stack_state(readings, bar - 1),
        ema_stack_state(readings, bar),
    ) else {
        return;
    };
    if previous == current {
        return;
    }
    events.push(signal_event(
        crate::events::codes::SIGNAL_EMA_STACK_CHANGE,
        SignalSubtype::CompactStateChange,
        EMA20_SOURCE_CODE,
        None,
        Some(f64::from(current)),
        Some(f64::from(previous)),
        current,
        bar,
        config,
    ));
}

fn rsi_zone(value: f64) -> i8 {
    if value < 30.0 {
        -1
    } else if value > 70.0 {
        1
    } else {
        0
    }
}

fn rsi_zone_change(
    readings: &Readings,
    bar: usize,
    config: &SignalConfig,
    events: &mut Vec<EventRecord>,
) {
    let (Some(previous), Some(current)) = (readings.rsi14.get(bar - 1), readings.rsi14.get(bar))
    else {
        return;
    };
    let previous = rsi_zone(previous);
    let current = rsi_zone(current);
    if previous == current {
        return;
    }
    events.push(signal_event(
        crate::events::codes::SIGNAL_RSI_ZONE_CHANGE,
        SignalSubtype::CompactStateChange,
        RSI_SOURCE_CODE,
        None,
        Some(f64::from(current)),
        Some(f64::from(previous)),
        current,
        bar,
        config,
    ));
}

#[allow(clippy::too_many_arguments)]
fn signal_event(
    event_code: u16,
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
        event_code,
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
        EMA20_SOURCE_CODE, RSI_SOURCE_CODE, SignalConfig, SignalEventSeries, SignalSubtype,
    };
    use crate::input::SignalSettings;
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

    fn signals(_candles: &[Candle], readings: &Readings) -> SignalEventSeries {
        SignalEventSeries::compute(
            readings,
            &crate::readings::MacdSeries::empty(readings.len()),
            &config(),
        )
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
    fn macd_cross_zero_cross_and_histogram_flip_are_distinct() {
        use crate::readings::{MacdSeries, ReadingSeries};

        let candles = candles(&[100.0; 5]);
        let readings = readings_with(
            &candles,
            flat(5, 100.0),
            flat(5, 100.0),
            flat(5, 100.0),
            flat(5, 50.0),
            flat(5, 10.0),
        );
        let macd = MacdSeries {
            line: ReadingSeries::new(vec![None, Some(-0.2), Some(-0.1), Some(0.1), Some(0.15)]),
            signal: ReadingSeries::new(vec![None, Some(-0.1), Some(-0.1), Some(0.0), Some(0.1)]),
            histogram: ReadingSeries::new(vec![None, Some(-0.1), Some(0.0), Some(0.1), Some(0.05)]),
        };
        let series = SignalEventSeries::compute(&readings, &macd, &config());

        assert!(
            series.events().iter().any(|event| {
                event.event_subtype_code == SignalSubtype::SeriesCrossover.code()
                    && event.source_code == super::MACD_LINE_SOURCE_CODE
            }),
            "the line/signal cross must be emitted"
        );
        let zero: Vec<_> = series
            .events()
            .iter()
            .filter(|event| event.event_subtype_code == SignalSubtype::ZeroCross.code())
            .collect();
        assert_eq!(zero.len(), 1);
        let flips: Vec<_> = series
            .events()
            .iter()
            .filter(|event| event.event_subtype_code == SignalSubtype::HistogramSlopeFlip.code())
            .collect();
        assert_eq!(flips.len(), 1);
        assert_eq!(flips[0].direction_code, -1);
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
    fn custom_rsi_levels_replace_the_default_thresholds() {
        let candles = candles(&[100.0, 100.0]);
        let readings = readings_with(
            &candles,
            none(2),
            none(2),
            none(2),
            vec![Some(39.0), Some(41.0)],
            none(2),
        );
        let settings = SignalSettings {
            rsi_levels: [40.0, 50.0, 60.0],
            ..SignalSettings::DEFAULT
        };
        let config = SignalConfig::with_settings(
            NonZeroU64::new(300).expect("a non-zero timeframe"),
            settings,
        );
        let series = SignalEventSeries::compute(
            &readings,
            &crate::readings::MacdSeries::empty(readings.len()),
            &config,
        );
        let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].reference_value, Some(40.0));
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
    fn the_detector_does_not_change_the_readings() {
        let candles = candles(&[100.0, 101.0, 102.0]);
        let readings = readings_with(
            &candles,
            vec![Some(99.0), Some(101.0), Some(102.0)],
            flat(3, 100.0),
            flat(3, 90.0),
            vec![Some(29.0), Some(31.0), Some(51.0)],
            flat(3, 10.0),
        );
        let before = readings.clone();

        let events = SignalEventSeries::compute(
            &readings,
            &crate::readings::MacdSeries::empty(readings.len()),
            &config(),
        );
        assert!(!events.is_empty(), "the fixture must produce an event");
        assert_eq!(readings, before);
    }

    #[test]
    fn the_signal_codes_are_the_contract_codes() {
        assert_eq!(SignalSubtype::ThresholdCrossing.code(), 1);
        assert_eq!(SignalSubtype::SeriesCrossover.code(), 2);
        assert_eq!(SignalSubtype::CompactStateChange.code(), 3);
        assert_eq!(SignalSubtype::TrendContinuation.code(), 6);
        assert_eq!(super::RSI_THRESHOLDS, [30.0, 50.0, 70.0]);
        assert_eq!(super::EMA_PAIRS.len(), 3);
    }

    #[test]
    fn ema_stack_and_rsi_zone_changes_are_published() {
        let candles = candles(&[100.0; 3]);
        let readings = readings_with(
            &candles,
            vec![Some(100.0), Some(100.0), Some(101.0)],
            vec![Some(100.0), Some(100.0), Some(100.0)],
            vec![Some(100.0), Some(100.0), Some(99.0)],
            vec![Some(50.0), Some(50.0), Some(75.0)],
            flat(3, 10.0),
        );
        let series = signals(&candles, &readings);
        assert!(series.events().iter().any(|event| {
            event.event_code == crate::events::codes::SIGNAL_EMA_STACK_CHANGE
                && event.occurred_bar_index == 2
        }));
        assert!(series.events().iter().any(|event| {
            event.event_code == crate::events::codes::SIGNAL_RSI_ZONE_CHANGE
                && event.occurred_bar_index == 2
        }));
    }
}
