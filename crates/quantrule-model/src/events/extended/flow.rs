//! Volume-flow event detection.

use super::shared::{cross_down_series, cross_up_series, mfi_zone};
use super::*;

pub(super) fn flow_events(
    candles: &[Candle],
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let timeframe = config.timeframe_secs();
    let close = candles[bar].close();
    let direction = (close - candles[bar].open()).signum() as i8;
    if cross_up_series(&extended.obv, &extended.obv_ma, bar) {
        push(
            events,
            codes::SIGNAL_OBV_CROSS,
            EventType::Signal,
            15,
            OBV_SOURCE_CODE,
            timeframe,
            bar,
            1,
            extended.obv.get(bar),
            extended.obv_ma.get(bar),
        );
    } else if cross_down_series(&extended.obv, &extended.obv_ma, bar) {
        push(
            events,
            codes::SIGNAL_OBV_CROSS,
            EventType::Signal,
            15,
            OBV_SOURCE_CODE,
            timeframe,
            bar,
            -1,
            extended.obv.get(bar),
            extended.obv_ma.get(bar),
        );
    }
    if bar > 0
        && let (Some(slope), Some(previous)) =
            (extended.obv_slope.get(bar), extended.obv_slope.get(bar - 1))
        && slope.signum() != previous.signum()
    {
        push(
            events,
            codes::SIGNAL_OBV_SLOPE,
            EventType::Signal,
            15,
            OBV_SOURCE_CODE,
            timeframe,
            bar,
            if slope > 0.0 { 1 } else { -1 },
            Some(slope),
            Some(0.0),
        );
    }
    if cross_up_value(&extended.cmf, bar, 0.0) {
        push(
            events,
            codes::SIGNAL_CMF_CROSS,
            EventType::Signal,
            15,
            CMF_SOURCE_CODE,
            timeframe,
            bar,
            direction,
            extended.cmf.get(bar),
            Some(0.0),
        );
    } else if cross_down_value(&extended.cmf, bar, 0.0) {
        push(
            events,
            codes::SIGNAL_CMF_CROSS,
            EventType::Signal,
            15,
            CMF_SOURCE_CODE,
            timeframe,
            bar,
            -direction,
            extended.cmf.get(bar),
            Some(0.0),
        );
    }
    if cross_up_value(&extended.cmf, bar, 0.2) {
        push(
            events,
            codes::SIGNAL_CMF_THRESHOLD,
            EventType::Signal,
            15,
            CMF_SOURCE_CODE,
            timeframe,
            bar,
            1,
            extended.cmf.get(bar),
            Some(0.2),
        );
    } else if cross_down_value(&extended.cmf, bar, -0.2) {
        push(
            events,
            codes::SIGNAL_CMF_THRESHOLD,
            EventType::Signal,
            15,
            CMF_SOURCE_CODE,
            timeframe,
            bar,
            -1,
            extended.cmf.get(bar),
            Some(-0.2),
        );
    }
    for level in [20.0, 50.0, 80.0] {
        if cross_up_value(&extended.mfi, bar, level) {
            push(
                events,
                codes::SIGNAL_MFI_THRESHOLD,
                EventType::Signal,
                15,
                MFI_SOURCE_CODE,
                timeframe,
                bar,
                1,
                extended.mfi.get(bar),
                Some(level),
            );
        } else if cross_down_value(&extended.mfi, bar, level) {
            push(
                events,
                codes::SIGNAL_MFI_THRESHOLD,
                EventType::Signal,
                15,
                MFI_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                extended.mfi.get(bar),
                Some(level),
            );
        }
    }
    if bar > 0
        && let (Some(current), Some(previous)) = (extended.mfi.get(bar), extended.mfi.get(bar - 1))
    {
        let current_zone = mfi_zone(current);
        let previous_zone = mfi_zone(previous);
        if current_zone != previous_zone {
            push(
                events,
                codes::SIGNAL_MFI_ZONE,
                EventType::Signal,
                15,
                MFI_SOURCE_CODE,
                timeframe,
                bar,
                current_zone,
                Some(current),
                Some(previous),
            );
        }
    }
    if cross_up_series(&extended.ad_line, &extended.ad_ma, bar) {
        push(
            events,
            codes::SIGNAL_AD_CROSS,
            EventType::Signal,
            15,
            AD_SOURCE_CODE,
            timeframe,
            bar,
            1,
            extended.ad_line.get(bar),
            extended.ad_ma.get(bar),
        );
    } else if cross_down_series(&extended.ad_line, &extended.ad_ma, bar) {
        push(
            events,
            codes::SIGNAL_AD_CROSS,
            EventType::Signal,
            15,
            AD_SOURCE_CODE,
            timeframe,
            bar,
            -1,
            extended.ad_line.get(bar),
            extended.ad_ma.get(bar),
        );
    }
}

pub(super) fn flow_divergences(
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    events: &mut Vec<EventRecord>,
) {
    for (source, series, hidden) in [
        (OBV_SOURCE_CODE, &extended.obv, true),
        (CMF_SOURCE_CODE, &extended.cmf, true),
        (MFI_SOURCE_CODE, &extended.mfi, true),
        (AD_SOURCE_CODE, &extended.ad_line, false),
    ] {
        let mut previous_high: Option<(f64, f64, usize, usize)> = None;
        let mut previous_low: Option<(f64, f64, usize, usize)> = None;
        for pivot in readings.pivots.all_confirmed(PivotScale::Major) {
            let Some(value) = series.get(pivot.occurred_at) else {
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
                PivotKind::Low if hidden && pivot.price > previous.0 && value < previous.1 => {
                    (3, 1)
                }
                PivotKind::High if hidden && pivot.price < previous.0 && value > previous.1 => {
                    (4, -1)
                }
                _ => continue,
            };
            let base = match source {
                OBV_SOURCE_CODE => codes::DIVERGENCE_OBV_REGULAR_BULLISH,
                CMF_SOURCE_CODE => codes::DIVERGENCE_CMF_REGULAR_BULLISH,
                MFI_SOURCE_CODE => codes::DIVERGENCE_MFI_REGULAR_BULLISH,
                _ => codes::DIVERGENCE_AD_REGULAR_BULLISH,
            };
            let event_code = base + u16::from(subtype - 1);
            events.push(EventRecord {
                event_code,
                event_type: EventType::Divergence,
                event_subtype_code: subtype,
                source_code: source,
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
}
