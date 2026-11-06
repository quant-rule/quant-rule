//! Participation event detection.

use super::*;

pub(super) fn participation_events(
    candles: &[Candle],
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let candle = candles[bar];
    let close = candle.close();
    let direction = (close - candle.open()).signum() as i8;
    let timeframe = config.timeframe_secs();
    if cross_up_value(&extended.rvol_fast, bar, 1.5) {
        push(
            events,
            codes::SIGNAL_RVOL_EXPANSION,
            EventType::Signal,
            11,
            RVOL_SOURCE_CODE,
            timeframe,
            bar,
            direction,
            extended.rvol_fast.get(bar),
            Some(1.5),
        );
    }
    if cross_down_value(&extended.rvol_fast, bar, 0.7) {
        push(
            events,
            codes::SIGNAL_RVOL_CONTRACTION,
            EventType::Signal,
            11,
            RVOL_SOURCE_CODE,
            timeframe,
            bar,
            0,
            extended.rvol_fast.get(bar),
            Some(0.7),
        );
    }
    if cross_up_value(&extended.rvol_ratio, bar, 1.5) {
        push(
            events,
            codes::SIGNAL_RVOL_RATIO,
            EventType::Signal,
            11,
            RVOL_SOURCE_CODE,
            timeframe,
            bar,
            direction,
            extended.rvol_ratio.get(bar),
            Some(1.5),
        );
    }
    if cross_up_value(&extended.volume_oscillator, bar, 0.0) {
        push(
            events,
            codes::SIGNAL_VOLUME_OSCILLATOR_CROSS,
            EventType::Signal,
            11,
            VOLUME_OSCILLATOR_SOURCE_CODE,
            timeframe,
            bar,
            direction,
            extended.volume_oscillator.get(bar),
            Some(0.0),
        );
    } else if cross_down_value(&extended.volume_oscillator, bar, 0.0) {
        push(
            events,
            codes::SIGNAL_VOLUME_OSCILLATOR_CROSS,
            EventType::Signal,
            11,
            VOLUME_OSCILLATOR_SOURCE_CODE,
            timeframe,
            bar,
            -direction,
            extended.volume_oscillator.get(bar),
            Some(0.0),
        );
    }

    if extended.climax_flag.get(bar) == Some(1.0) {
        let climax_direction = extended.climax_direction.get(bar).unwrap_or(0.0) as i8;
        if bar > 0 {
            let previous = candles[bar - 1];
            if close >= previous.low() && close <= previous.high() {
                push(
                    events,
                    codes::SIGNAL_CLIMAX_REVERSAL,
                    EventType::Signal,
                    11,
                    CLIMAX_SOURCE_CODE,
                    timeframe,
                    bar,
                    -climax_direction,
                    extended.climax_strength.get(bar),
                    Some(previous.high()),
                );
            }
        }
        let span = candle.high() - candle.low();
        let close_position = if span > 0.0 {
            (close - candle.low()) / span
        } else {
            0.0
        };
        if close_position >= 0.8 {
            push(
                events,
                codes::SIGNAL_CLIMAX_CONTINUATION,
                EventType::Signal,
                11,
                CLIMAX_SOURCE_CODE,
                timeframe,
                bar,
                1,
                extended.climax_strength.get(bar),
                Some(close_position),
            );
        } else if close_position <= 0.2 {
            push(
                events,
                codes::SIGNAL_CLIMAX_CONTINUATION,
                EventType::Signal,
                11,
                CLIMAX_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                extended.climax_strength.get(bar),
                Some(close_position),
            );
        }
    }

    if extended.absorption_flag.get(bar) == Some(1.0) {
        let level = extended.vwap.get(bar).or_else(|| extended.poc.get(bar));
        push(
            events,
            codes::LEVEL_ABSORPTION_TEST,
            EventType::Level,
            1,
            ABSORPTION_SOURCE_CODE,
            timeframe,
            bar,
            0,
            extended.absorption_strength.get(bar),
            level,
        );
    }
    if bar > 0 && extended.absorption_flag.get(bar - 1) == Some(1.0) {
        let previous = candles[bar - 1];
        let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) else {
            return;
        };
        if close > previous.high() + config.level_atr * atr {
            push(
                events,
                codes::SIGNAL_ABSORPTION_BREAK,
                EventType::Signal,
                12,
                ABSORPTION_SOURCE_CODE,
                timeframe,
                bar,
                1,
                Some(close),
                Some(previous.high()),
            );
        } else if close < previous.low() - config.level_atr * atr {
            push(
                events,
                codes::SIGNAL_ABSORPTION_BREAK,
                EventType::Signal,
                12,
                ABSORPTION_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                Some(close),
                Some(previous.low()),
            );
        } else if close >= previous.low() && close <= previous.high() {
            let direction = extended.absorption_direction.get(bar - 1).unwrap_or(0.0) as i8;
            push(
                events,
                codes::SIGNAL_ABSORPTION_REJECTION,
                EventType::Signal,
                12,
                ABSORPTION_SOURCE_CODE,
                timeframe,
                bar,
                -direction,
                Some(close),
                Some(previous.high()),
            );
        }
    }
}

pub(super) fn participation_divergences(
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    events: &mut Vec<EventRecord>,
) {
    for (source, series) in [
        (RVOL_SOURCE_CODE, &extended.rvol_slow),
        (CLIMAX_SOURCE_CODE, &extended.rvol_fast),
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
                _ => continue,
            };
            let event_code = match (source, subtype) {
                (RVOL_SOURCE_CODE, 1) => codes::DIVERGENCE_RVOL_BULLISH,
                (RVOL_SOURCE_CODE, _) => codes::DIVERGENCE_RVOL_BEARISH,
                (CLIMAX_SOURCE_CODE, 1) => codes::DIVERGENCE_CLIMAX_BULLISH,
                _ => codes::DIVERGENCE_CLIMAX_BEARISH,
            };
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
