//! Divergence event detection.

use super::{
    ATR_SOURCE_CODE, EventRecord, EventType, ExtendedEventConfig, ExtendedReadings,
    PROFILE_SOURCE_CODE, PivotKind, PivotScale, Readings, VWAP_SOURCE_CODE, codes,
};
pub(super) fn divergence_events(
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
        let event_code = match subtype {
            1 => codes::DIVERGENCE_VWAP_REGULAR_BULLISH,
            2 => codes::DIVERGENCE_VWAP_REGULAR_BEARISH,
            3 => codes::DIVERGENCE_VWAP_HIDDEN_BULLISH,
            _ => codes::DIVERGENCE_VWAP_HIDDEN_BEARISH,
        };
        events.push(EventRecord {
            event_code,
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
        let event_code = if subtype == 1 {
            codes::DIVERGENCE_PROFILE_POC_BULLISH
        } else {
            codes::DIVERGENCE_PROFILE_POC_BEARISH
        };
        events.push(EventRecord {
            event_code,
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

pub(crate) fn divergence_subtype(
    kind: PivotKind,
    price_delta: f64,
    value_delta: f64,
) -> Option<(u8, i8)> {
    match kind {
        PivotKind::Low if price_delta < 0.0 && value_delta > 0.0 => Some((1, 1)),
        PivotKind::High if price_delta > 0.0 && value_delta < 0.0 => Some((2, -1)),
        _ => None,
    }
}

pub(super) fn atr_divergences(
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    events: &mut Vec<EventRecord>,
) {
    let mut previous_high: Option<(f64, f64, usize, usize)> = None;
    let mut previous_low: Option<(f64, f64, usize, usize)> = None;
    for pivot in readings.pivots.all_confirmed(PivotScale::Major) {
        let Some(value) = extended.atr_ratio.get(pivot.occurred_at) else {
            continue;
        };
        let state = (pivot.price, value, pivot.occurred_at, pivot.available_at);
        let previous = match pivot.kind {
            PivotKind::High => previous_high.replace(state),
            PivotKind::Low => previous_low.replace(state),
        };
        let Some(previous) = previous else { continue };
        let Some((subtype, direction)) =
            divergence_subtype(pivot.kind, pivot.price - previous.0, value - previous.1)
        else {
            continue;
        };
        let event_code = if subtype == 1 {
            codes::DIVERGENCE_ATR_BULLISH
        } else {
            codes::DIVERGENCE_ATR_BEARISH
        };
        events.push(EventRecord {
            event_code,
            event_type: EventType::Divergence,
            event_subtype_code: subtype,
            source_code: ATR_SOURCE_CODE,
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
