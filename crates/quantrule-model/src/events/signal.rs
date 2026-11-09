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
    /// Code 24: a compression breakout retested and held.
    BreakoutRetestHold = 24,
}

impl SignalSubtype {
    /// Every subtype, in contract code order.
    pub const ALL: [Self; 10] = [
        Self::ThresholdCrossing,
        Self::SeriesCrossover,
        Self::CompactStateChange,
        Self::HistogramSlopeFlip,
        Self::ZeroCross,
        Self::TrendContinuation,
        Self::CompressionBreakout,
        Self::CompressionRange,
        Self::CompressionRangeOnset,
        Self::BreakoutRetestHold,
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
            24 => Some(Self::BreakoutRetestHold),
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
    ema_periods: [usize; 3],
}

impl SignalConfig {
    /// Builds a configuration from a non-zero timeframe.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self {
            timeframe_secs,
            settings: SignalSettings::DEFAULT,
            ema_periods: [20, 50, 200],
        }
    }

    /// Builds a configuration with explicit signal settings.
    #[must_use]
    pub const fn with_settings(timeframe_secs: NonZeroU64, settings: SignalSettings) -> Self {
        Self {
            timeframe_secs,
            settings,
            ema_periods: [20, 50, 200],
        }
    }

    /// Uses the configured fast, middle, and slow EMA periods.
    #[must_use]
    pub const fn with_ema_periods(mut self, ema_periods: [usize; 3]) -> Self {
        self.ema_periods = ema_periods;
        self
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

    /// The configured fast, middle, and slow EMA periods.
    #[must_use]
    pub const fn ema_periods(self) -> [usize; 3] {
        self.ema_periods
    }
}

/// The Signal events detected over one readings series and its MACD series.
///
/// The events are stored in bar order. Inside one bar the fixed order is RSI
/// thresholds; EMA pairs; then the EMA-stack and RSI-zone state changes.
/// Distinct signal conditions can coincide on a bar; they are not duplicates
/// because each names a different defined relationship.
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
        let direction = if previous <= threshold && current > threshold {
            1
        } else if previous >= threshold && current < threshold {
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
        let Some(fast_role) = ema_role(config.ema_periods(), fast) else {
            continue;
        };
        let Some(slow_role) = ema_role(config.ema_periods(), slow) else {
            continue;
        };
        let (Some(previous_fast), Some(previous_slow), Some(current_fast), Some(current_slow)) = (
            ema_series(readings, fast_role).get(bar - 1),
            ema_series(readings, slow_role).get(bar - 1),
            ema_series(readings, fast_role).get(bar),
            ema_series(readings, slow_role).get(bar),
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
            ema_source(fast_role),
            Some(ema_reference(slow_role)),
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

    let Some(two_back_index) = bar.checked_sub(2) else {
        return;
    };
    let (Some(two_back), Some(previous), Some(current)) = (
        macd.histogram.get(two_back_index),
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

fn ema_role(periods: [usize; 3], period: usize) -> Option<usize> {
    periods.iter().position(|configured| *configured == period)
}

fn ema_series(readings: &Readings, role: usize) -> &crate::readings::ReadingSeries {
    match role {
        0 => &readings.ema20,
        1 => &readings.ema50,
        _ => &readings.ema200,
    }
}

fn ema_source(role: usize) -> u16 {
    match role {
        0 => EMA20_SOURCE_CODE,
        1 => EMA50_SOURCE_CODE,
        _ => EMA200_SOURCE_CODE,
    }
}

fn ema_reference(role: usize) -> u32 {
    match role {
        0 => EMA20_REFERENCE_CODE,
        1 => EMA50_REFERENCE_CODE,
        _ => EMA200_REFERENCE_CODE,
    }
}

#[cfg(test)]
mod tests;
