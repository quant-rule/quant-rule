//! Extended events for the first indicator batch: VWAP and volume profile.
//!
//! The detector reads only closed candles and the published readings. It emits
//! Level, Divergence, and Signal records with the contract's numeric codes. The
//! module is pure: no I/O, clock, randomness, or statistics.

use std::collections::BTreeSet;
use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::events::{EventRecord, EventType};
use crate::readings::{ExtendedReadings, PivotKind, PivotScale, Readings};

/// Source code for the rolling VWAP.
pub const VWAP_SOURCE_CODE: u16 = 19;
/// Source code for the rolling volume profile.
pub const PROFILE_SOURCE_CODE: u16 = 20;
/// Source code for RVOL, used by the profile breakout confirmation.
pub const RVOL_SOURCE_CODE: u16 = 6;

/// The configuration of the extended event run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtendedEventConfig {
    timeframe_secs: NonZeroU64,
    reclaim_bars: usize,
    level_atr: f64,
}

impl ExtendedEventConfig {
    /// Builds a configuration from a non-zero timeframe.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self {
            timeframe_secs,
            reclaim_bars: 6,
            level_atr: 0.2,
        }
    }

    /// Builds a configuration from a timeframe in seconds.
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

/// The extended events detected over one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtendedEventSeries {
    events: Vec<EventRecord>,
}

impl ExtendedEventSeries {
    /// Computes every extended event over the candles and readings.
    #[must_use]
    pub fn compute(
        candles: &[Candle],
        readings: &Readings,
        extended: &ExtendedReadings,
        config: &ExtendedEventConfig,
    ) -> Self {
        let mut events = Vec::new();
        for bar in 0..candles.len() {
            let candle = candles[bar];
            let atr = readings.atr14.get(bar).filter(|atr| *atr > 0.0);
            vwap_events(candles, extended, config, bar, candle, atr, &mut events);
            profile_events(candles, readings, extended, config, bar, &mut events);
        }
        divergence_events(readings, extended, config, &mut events);
        events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));
        // The occurrence key is one event per timeframe, type, subtype, source,
        // bar, direction, and reference. A reclaim and a rejection can describe
        // the same bar; the first (reclaim) wins, so no occurrence is counted
        // twice.
        let mut seen = BTreeSet::new();
        events.retain(|event| {
            let threshold =
                if event.event_type == EventType::Signal && event.event_subtype_code == 1 {
                    event.reference_value.map(f64::to_bits)
                } else {
                    None
                };
            seen.insert((
                event.timeframe_secs,
                event.event_type.code(),
                event.event_subtype_code,
                event.source_code,
                event.occurred_bar_index,
                event.reference_code,
                threshold,
            ))
        });
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

    /// Every detected event.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }
}

// A private constructor that mirrors the EventRecord fields one-for-one.
// Grouping them into synthetic structs would obscure the mapping, so the
// argument count is intentional.
#[allow(clippy::too_many_arguments)]
fn push(
    events: &mut Vec<EventRecord>,
    event_type: EventType,
    subtype: u8,
    source: u16,
    timeframe: u64,
    bar: usize,
    direction: i8,
    primary: Option<f64>,
    reference: Option<f64>,
) {
    events.push(EventRecord {
        event_type,
        event_subtype_code: subtype,
        source_code: source,
        timeframe_secs: timeframe,
        occurred_bar_index: bar,
        available_bar_index: bar,
        direction_code: direction,
        reference_code: None,
        primary_value: primary,
        reference_value: reference,
    });
}

fn vwap_events(
    candles: &[Candle],
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    bar: usize,
    candle: Candle,
    atr: Option<f64>,
    events: &mut Vec<EventRecord>,
) {
    let Some(vwap) = extended.vwap.get(bar) else {
        return;
    };
    let Some(atr) = atr else {
        return;
    };
    let close = candle.close();
    let distance = (close - vwap).abs();
    let timeframe = config.timeframe_secs();
    if distance <= config.level_atr * atr {
        push(
            events,
            EventType::Level,
            1,
            VWAP_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(distance),
            Some(vwap),
        );
    }
    if bar > 0
        && let (Some(previous_vwap), Some(previous_close)) = (
            extended.vwap.get(bar - 1),
            candles.get(bar - 1).map(Candle::close),
        )
    {
        let crossed = (previous_close - previous_vwap) * (close - vwap) <= 0.0
            && (previous_close - previous_vwap) != (close - vwap);
        if crossed {
            push(
                events,
                EventType::Level,
                2,
                VWAP_SOURCE_CODE,
                timeframe,
                bar,
                if close > vwap { 1 } else { -1 },
                Some(distance),
                Some(vwap),
            );
        }
        let side = (close - vwap).signum();
        let previous_side = (previous_close - previous_vwap).signum();
        if side != 0.0
            && side == previous_side
            && distance >= config.level_atr * atr
            && (previous_close - previous_vwap).abs() >= config.level_atr * atr
        {
            push(
                events,
                EventType::Level,
                3,
                VWAP_SOURCE_CODE,
                timeframe,
                bar,
                if side > 0.0 { 1 } else { -1 },
                Some(distance),
                Some(vwap),
            );
        }
    }

    // Reclaim: a cross after at least `reclaim_bars` bars on the other side.
    if bar >= config.reclaim_bars {
        let mut below = true;
        for (index, prior) in candles
            .iter()
            .enumerate()
            .take(bar)
            .skip(bar - config.reclaim_bars)
        {
            let Some(prior_vwap) = extended.vwap.get(index) else {
                below = false;
                break;
            };
            if prior.close() >= prior_vwap {
                below = false;
                break;
            }
        }
        if below
            && let Some(prior_vwap) = extended.vwap.get(bar - 1)
            && candles[bar - 1].close() <= prior_vwap
            && close > vwap
        {
            push(
                events,
                EventType::Signal,
                14,
                VWAP_SOURCE_CODE,
                timeframe,
                bar,
                1,
                Some(distance),
                Some(vwap),
            );
        }
    }

    // Rejection: a wick through VWAP that closes back on the original side.
    if candle.high() >= vwap && close < vwap && close < candle.open() {
        push(
            events,
            EventType::Signal,
            14,
            VWAP_SOURCE_CODE,
            timeframe,
            bar,
            -1,
            Some(distance),
            Some(vwap),
        );
    } else if candle.low() <= vwap && close > vwap && close > candle.open() {
        push(
            events,
            EventType::Signal,
            14,
            VWAP_SOURCE_CODE,
            timeframe,
            bar,
            1,
            Some(distance),
            Some(vwap),
        );
    }

    // Slope change.
    if bar > 0
        && let (Some(slope), Some(previous)) = (
            extended.vwap_slope.get(bar),
            extended.vwap_slope.get(bar - 1),
        )
        && slope.signum() != previous.signum()
    {
        push(
            events,
            EventType::Signal,
            14,
            VWAP_SOURCE_CODE,
            timeframe,
            bar,
            if slope > 0.0 { 1 } else { -1 },
            Some(slope),
            Some(0.0),
        );
    }
}

fn profile_events(
    candles: &[Candle],
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let Some(poc) = extended.poc.get(bar) else {
        return;
    };
    let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) else {
        return;
    };
    let candle = candles[bar];
    let close = candle.close();
    let timeframe = config.timeframe_secs();
    if let Some(vah) = extended.vah.get(bar)
        && (candle.high() - vah).abs() <= config.level_atr * atr
    {
        push(
            events,
            EventType::Level,
            1,
            PROFILE_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(candle.high()),
            Some(vah),
        );
    }
    if let Some(val) = extended.val.get(bar)
        && (candle.low() - val).abs() <= config.level_atr * atr
    {
        push(
            events,
            EventType::Level,
            1,
            PROFILE_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(candle.low()),
            Some(val),
        );
    }
    if (close - poc).abs() <= config.level_atr * atr {
        push(
            events,
            EventType::Level,
            1,
            PROFILE_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(close),
            Some(poc),
        );
    }
    if let (Some(vah), Some(val)) = (extended.vah.get(bar), extended.val.get(bar)) {
        if bar > 0
            && let (Some(previous_vah), Some(previous_val), Some(previous_close)) = (
                extended.vah.get(bar - 1),
                extended.val.get(bar - 1),
                candles.get(bar - 1).map(Candle::close),
            )
        {
            if close > vah && previous_close <= previous_vah {
                push(
                    events,
                    EventType::Level,
                    3,
                    PROFILE_SOURCE_CODE,
                    timeframe,
                    bar,
                    1,
                    Some(close),
                    Some(vah),
                );
            } else if close < val && previous_close >= previous_val {
                push(
                    events,
                    EventType::Level,
                    3,
                    PROFILE_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(close),
                    Some(val),
                );
            }
            if close <= vah && previous_close > previous_vah {
                push(
                    events,
                    EventType::Level,
                    2,
                    PROFILE_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(close),
                    Some(vah),
                );
            } else if close >= val && previous_close < previous_val {
                push(
                    events,
                    EventType::Level,
                    2,
                    PROFILE_SOURCE_CODE,
                    timeframe,
                    bar,
                    1,
                    Some(close),
                    Some(val),
                );
            }
        }
        let rvol = readings.rvol20.get(bar).unwrap_or(0.0);
        if (close > vah || close < val) && rvol >= 1.5 {
            push(
                events,
                EventType::Signal,
                13,
                PROFILE_SOURCE_CODE,
                timeframe,
                bar,
                if close > vah { 1 } else { -1 },
                Some(rvol),
                Some(if close > vah { vah } else { val }),
            );
        }
    }
    if bar > 0 {
        if let (Some(previous_poc), Some(previous_close)) = (
            extended.poc.get(bar - 1),
            candles.get(bar - 1).map(Candle::close),
        ) {
            let crossed = (previous_close - previous_poc) * (close - poc) <= 0.0
                && (previous_close - previous_poc) != (close - poc);
            if crossed {
                push(
                    events,
                    EventType::Signal,
                    13,
                    PROFILE_SOURCE_CODE,
                    timeframe,
                    bar,
                    if close > poc { 1 } else { -1 },
                    Some(close),
                    Some(poc),
                );
            }
        }
        if let (Some(skew), Some(previous)) = (
            extended.profile_skew.get(bar),
            extended.profile_skew.get(bar - 1),
        ) && skew.signum() != previous.signum()
        {
            push(
                events,
                EventType::Signal,
                13,
                PROFILE_SOURCE_CODE,
                timeframe,
                bar,
                if skew > 0.0 { 1 } else { -1 },
                Some(skew),
                Some(0.0),
            );
        }
        if let (Some(concentration), Some(previous)) = (
            extended.profile_concentration.get(bar),
            extended.profile_concentration.get(bar - 1),
        ) && (concentration - 0.5).signum() != (previous - 0.5).signum()
        {
            push(
                events,
                EventType::Signal,
                13,
                PROFILE_SOURCE_CODE,
                timeframe,
                bar,
                if concentration > 0.5 { 1 } else { -1 },
                Some(concentration),
                Some(0.5),
            );
        }
    }
}

fn divergence_events(
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    events: &mut Vec<EventRecord>,
) {
    let mut previous_high: Option<(f64, f64, usize, usize)> = None;
    let mut previous_low: Option<(f64, f64, usize, usize)> = None;
    for pivot in readings.pivots.all_confirmed(PivotScale::Major) {
        let value = match pivot.kind {
            PivotKind::High => extended.vwap_distance_atr.get(pivot.occurred_at),
            PivotKind::Low => extended.vwap_distance_atr.get(pivot.occurred_at),
        };
        let Some(value) = value else { continue };
        let state = (pivot.price, value, pivot.occurred_at, pivot.available_at);
        let previous = match pivot.kind {
            PivotKind::High => previous_high.replace(state),
            PivotKind::Low => previous_low.replace(state),
        };
        let Some(previous) = previous else { continue };
        let (subtype, direction) = match pivot.kind {
            PivotKind::Low if pivot.price < previous.0 && value > previous.1 => (1, 1),
            PivotKind::Low if pivot.price > previous.0 && value < previous.1 => (3, 1),
            PivotKind::High if pivot.price > previous.0 && value < previous.1 => (2, -1),
            PivotKind::High if pivot.price < previous.0 && value > previous.1 => (4, -1),
            _ => continue,
        };
        events.push(EventRecord {
            event_type: EventType::Divergence,
            event_subtype_code: subtype,
            source_code: VWAP_SOURCE_CODE,
            timeframe_secs: config.timeframe_secs(),
            occurred_bar_index: state.2,
            available_bar_index: state.3,
            direction_code: direction,
            reference_code: None,
            primary_value: Some(state.0),
            reference_value: Some(value),
        });
    }

    let mut previous_high: Option<(f64, f64, usize, usize)> = None;
    let mut previous_low: Option<(f64, f64, usize, usize)> = None;
    for pivot in readings.pivots.all_confirmed(PivotScale::Major) {
        let Some(value) = extended.poc.get(pivot.occurred_at) else {
            continue;
        };
        let state = (pivot.price, value, pivot.occurred_at, pivot.available_at);
        let previous = match pivot.kind {
            PivotKind::High => previous_high.replace(state),
            PivotKind::Low => previous_low.replace(state),
        };
        let Some(previous) = previous else { continue };
        let (subtype, direction) = match pivot.kind {
            PivotKind::Low if pivot.price < previous.0 && value > previous.1 => (1, 1),
            PivotKind::High if pivot.price > previous.0 && value < previous.1 => (2, -1),
            _ => continue,
        };
        events.push(EventRecord {
            event_type: EventType::Divergence,
            event_subtype_code: subtype,
            source_code: PROFILE_SOURCE_CODE,
            timeframe_secs: config.timeframe_secs(),
            occurred_bar_index: state.2,
            available_bar_index: state.3,
            direction_code: direction,
            reference_code: None,
            primary_value: Some(state.0),
            reference_value: Some(value),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{ExtendedEventConfig, ExtendedEventSeries};
    use crate::readings::{ExtendedConfig, ExtendedReadings, Readings, ReadingsConfig};
    use quantrule_data::Candle;

    fn candle(index: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle::new(index, index as i64, open, high, low, close, Some(volume))
            .expect("valid candle")
    }

    fn compute(candles: &[Candle]) -> ExtendedEventSeries {
        let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
        let extended = ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT);
        let config = ExtendedEventConfig::from_secs(3600).expect("timeframe");
        ExtendedEventSeries::compute(candles, &readings, &extended, &config)
    }

    #[test]
    fn vwap_cross_is_a_level_event() {
        let mut candles = Vec::new();
        for index in 0..30 {
            let price = if index < 25 { 10.0 } else { 12.0 };
            candles.push(candle(index, price, price, price, price, 1.0));
        }
        let events = compute(&candles);
        assert!(events.events().iter().any(|event| {
            event.source_code == 19 && event.event_type == crate::events::EventType::Level
        }));
    }

    #[test]
    fn no_events_on_an_empty_or_short_series() {
        assert!(compute(&[]).is_empty());
        let short = [candle(0, 10.0, 10.0, 10.0, 10.0, 1.0)];
        assert!(compute(&short).is_empty());
    }

    #[test]
    fn appending_future_bars_does_not_change_past_events() {
        let candles: Vec<Candle> = (0..40)
            .map(|index| {
                let base = 10.0 + (index as f64 * 0.2).sin();
                candle(
                    index,
                    base,
                    base + 0.5,
                    base - 0.5,
                    base + 0.1,
                    1.0 + index as f64,
                )
            })
            .collect();
        let full = compute(&candles);
        let prefix = compute(&candles[..25]);
        let full_past: Vec<_> = full
            .events()
            .iter()
            .filter(|event| event.available_bar_index < 25)
            .copied()
            .collect();
        let prefix_events: Vec<_> = prefix.events().to_vec();
        assert_eq!(full_past, prefix_events);
    }

    #[test]
    fn repeated_computation_is_identical() {
        let candles: Vec<Candle> = (0..40)
            .map(|index| {
                let base = 10.0 + (index as f64 * 0.2).sin();
                candle(
                    index,
                    base,
                    base + 0.5,
                    base - 0.5,
                    base + 0.1,
                    1.0 + index as f64,
                )
            })
            .collect();
        assert_eq!(compute(&candles), compute(&candles));
    }
}
