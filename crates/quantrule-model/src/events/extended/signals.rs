//! Signal events for the extended single-instrument readings.
//!
//! These are the indicator signals that do not need a panel or a session
//! calendar: ATR, range, MAR, Keltner, Donchian, structure, strength, Aroon,
//! candle anatomy, patterns, gaps, path, extremes, drawdown, and range
//! position. The module is pure and deterministic.

use crate::events::EventRecord;
use crate::events::EventType;
use crate::events::codes;
use crate::readings::{ExtendedReadings, Readings};
use quantrule_data::Candle;

use super::{
    AROON_SOURCE_CODE, ATR_SOURCE_CODE, CANDLE_SOURCE_CODE, DIRECTION_SOURCE_CODE,
    DONCHIAN_SOURCE_CODE, DRAWDOWN_SOURCE_CODE, EXTREME_SOURCE_CODE, ExtendedEventConfig,
    GAP_SOURCE_CODE, KELTNER_SOURCE_CODE, MAR_SOURCE_CODE, PATH_SOURCE_CODE, PATTERN_SOURCE_CODE,
    RANGE_POSITION_SOURCE_CODE, RANGE_SOURCE_CODE, STRENGTH_SOURCE_CODE, SWING_SOURCE_CODE,
    cross_down_value, cross_up_value, push,
};

/// Emits the single-instrument signal events.
pub(crate) fn compute(
    candles: &[Candle],
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    events: &mut Vec<EventRecord>,
) {
    let timeframe = config.timeframe_secs();
    for bar in 0..candles.len() {
        let candle = candles[bar];
        let direction = (candle.close() - candle.open()).signum() as i8;
        let atr = readings.atr14.get(bar).filter(|atr| *atr > 0.0);
        if cross_down_value(&extended.atr_ratio, bar, 0.7) {
            push(
                events,
                codes::SIGNAL_ATR_COMPRESSION,
                EventType::Signal,
                16,
                ATR_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.atr_ratio.get(bar),
                Some(0.7),
            );
        } else if cross_up_value(&extended.atr_ratio, bar, 1.3) {
            push(
                events,
                codes::SIGNAL_ATR_EXPANSION,
                EventType::Signal,
                16,
                ATR_SOURCE_CODE,
                timeframe,
                bar,
                direction,
                extended.atr_ratio.get(bar),
                Some(1.3),
            );
        }
        if bar > 0
            && let (Some(current), Some(previous)) = (
                extended.vol_regime.get(bar),
                extended.vol_regime.get(bar - 1),
            )
            && current != previous
        {
            push(
                events,
                codes::SIGNAL_ATR_REGIME,
                EventType::Signal,
                16,
                ATR_SOURCE_CODE,
                timeframe,
                bar,
                if current > previous { 1 } else { -1 },
                Some(current),
                Some(previous),
            );
        }
        if cross_up_value(&extended.range_expansion, bar, 1.5) {
            push(
                events,
                codes::SIGNAL_RANGE_EXPANSION,
                EventType::Signal,
                17,
                RANGE_SOURCE_CODE,
                timeframe,
                bar,
                direction,
                extended.range_expansion.get(bar),
                Some(1.5),
            );
        } else if cross_down_value(&extended.range_expansion, bar, 0.7) {
            push(
                events,
                codes::SIGNAL_RANGE_CONTRACTION,
                EventType::Signal,
                17,
                RANGE_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.range_expansion.get(bar),
                Some(0.7),
            );
        }
        if cross_up_value(&extended.mar_ratio, bar, 1.5) {
            push(
                events,
                codes::SIGNAL_MAR_REGIME,
                EventType::Signal,
                16,
                MAR_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.mar_ratio.get(bar),
                Some(1.5),
            );
        } else if cross_down_value(&extended.mar_ratio, bar, 0.7) {
            push(
                events,
                codes::SIGNAL_MAR_REGIME,
                EventType::Signal,
                16,
                MAR_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.mar_ratio.get(bar),
                Some(0.7),
            );
        }
        if let (Some(upper), Some(lower)) = (
            extended.keltner_upper.get(bar),
            extended.keltner_lower.get(bar),
        ) {
            if bar > 0
                && let (Some(previous_close), Some(previous_upper), Some(previous_lower)) = (
                    candles.get(bar - 1).map(Candle::close),
                    extended.keltner_upper.get(bar - 1),
                    extended.keltner_lower.get(bar - 1),
                )
            {
                if candle.close() > upper && previous_close <= previous_upper {
                    push(
                        events,
                        codes::SIGNAL_KELTNER_BREAKOUT,
                        EventType::Signal,
                        18,
                        KELTNER_SOURCE_CODE,
                        timeframe,
                        bar,
                        1,
                        Some(candle.close()),
                        Some(upper),
                    );
                } else if candle.close() < lower && previous_close >= previous_lower {
                    push(
                        events,
                        codes::SIGNAL_KELTNER_BREAKOUT,
                        EventType::Signal,
                        18,
                        KELTNER_SOURCE_CODE,
                        timeframe,
                        bar,
                        -1,
                        Some(candle.close()),
                        Some(lower),
                    );
                }
            }
            if let Some(position) = extended.keltner_position.get(bar)
                && let Some(previous) = extended.keltner_position.get(bar - 1)
            {
                if previous <= 0.8 && position > 0.8 {
                    push(
                        events,
                        codes::SIGNAL_KELTNER_POSITION,
                        EventType::Signal,
                        18,
                        KELTNER_SOURCE_CODE,
                        timeframe,
                        bar,
                        1,
                        Some(position),
                        Some(0.8),
                    );
                } else if previous >= 0.2 && position < 0.2 {
                    push(
                        events,
                        codes::SIGNAL_KELTNER_POSITION,
                        EventType::Signal,
                        18,
                        KELTNER_SOURCE_CODE,
                        timeframe,
                        bar,
                        -1,
                        Some(position),
                        Some(0.2),
                    );
                }
            }
        }
        {
            if bar > 0
                && let (Some(previous_close), Some(previous_upper), Some(previous_lower)) = (
                    candles.get(bar - 1).map(Candle::close),
                    extended.donchian_upper.get(bar - 1),
                    extended.donchian_lower.get(bar - 1),
                )
            {
                if candle.close() > previous_upper && previous_close <= previous_upper {
                    push(
                        events,
                        codes::SIGNAL_DONCHIAN_BREAKOUT,
                        EventType::Signal,
                        18,
                        DONCHIAN_SOURCE_CODE,
                        timeframe,
                        bar,
                        1,
                        Some(candle.close()),
                        Some(previous_upper),
                    );
                } else if candle.close() < previous_lower && previous_close >= previous_lower {
                    push(
                        events,
                        codes::SIGNAL_DONCHIAN_BREAKOUT,
                        EventType::Signal,
                        18,
                        DONCHIAN_SOURCE_CODE,
                        timeframe,
                        bar,
                        -1,
                        Some(candle.close()),
                        Some(previous_lower),
                    );
                }
            }
            if let Some(position) = extended.donchian_position.get(bar)
                && let Some(previous) = extended.donchian_position.get(bar - 1)
            {
                if previous <= 0.8 && position > 0.8 {
                    push(
                        events,
                        codes::SIGNAL_DONCHIAN_POSITION,
                        EventType::Signal,
                        18,
                        DONCHIAN_SOURCE_CODE,
                        timeframe,
                        bar,
                        1,
                        Some(position),
                        Some(0.8),
                    );
                } else if previous >= 0.2 && position < 0.2 {
                    push(
                        events,
                        codes::SIGNAL_DONCHIAN_POSITION,
                        EventType::Signal,
                        18,
                        DONCHIAN_SOURCE_CODE,
                        timeframe,
                        bar,
                        -1,
                        Some(position),
                        Some(0.2),
                    );
                }
            }
        }
        if bar > 0
            && let (Some(current), Some(previous)) = (
                extended.structure_state.get(bar),
                extended.structure_state.get(bar - 1),
            )
            && current != previous
        {
            push(
                events,
                codes::SIGNAL_STRUCTURE_CHANGE,
                EventType::Signal,
                19,
                SWING_SOURCE_CODE,
                timeframe,
                bar,
                current as i8,
                Some(current),
                Some(previous),
            );
        }
        if cross_up_value(&extended.level_touches, bar, 2.0) {
            push(
                events,
                codes::SIGNAL_STRENGTH_CHANGE,
                EventType::Signal,
                19,
                STRENGTH_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.level_touches.get(bar),
                Some(2.0),
            );
        }
        if cross_up_value(&extended.aroon_up, bar, 70.0) {
            push(
                events,
                codes::SIGNAL_AROON_CROSS,
                EventType::Signal,
                19,
                AROON_SOURCE_CODE,
                timeframe,
                bar,
                1,
                extended.aroon_up.get(bar),
                Some(70.0),
            );
        }
        if cross_down_value(&extended.aroon_down, bar, 30.0) {
            push(
                events,
                codes::SIGNAL_AROON_CROSS,
                EventType::Signal,
                19,
                AROON_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                extended.aroon_down.get(bar),
                Some(30.0),
            );
        }
        if cross_up_value(&extended.aroon_osc, bar, 0.0) {
            push(
                events,
                codes::SIGNAL_AROON_OSC,
                EventType::Signal,
                19,
                AROON_SOURCE_CODE,
                timeframe,
                bar,
                1,
                extended.aroon_osc.get(bar),
                Some(0.0),
            );
        } else if cross_down_value(&extended.aroon_osc, bar, 0.0) {
            push(
                events,
                codes::SIGNAL_AROON_OSC,
                EventType::Signal,
                19,
                AROON_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                extended.aroon_osc.get(bar),
                Some(0.0),
            );
        }
        if let (Some(body), Some(position), Some(atr)) = (
            extended.body_ratio.get(bar),
            extended.close_position.get(bar),
            atr,
        ) {
            let span = candle.high() - candle.low();
            if span >= 1.5 * atr && body >= 0.7 {
                if position >= 0.8 {
                    push(
                        events,
                        codes::SIGNAL_CANDLE_MOMENTUM,
                        EventType::Signal,
                        10,
                        CANDLE_SOURCE_CODE,
                        timeframe,
                        bar,
                        1,
                        Some(body),
                        Some(position),
                    );
                } else if position <= 0.2 {
                    push(
                        events,
                        codes::SIGNAL_CANDLE_MOMENTUM,
                        EventType::Signal,
                        10,
                        CANDLE_SOURCE_CODE,
                        timeframe,
                        bar,
                        -1,
                        Some(body),
                        Some(position),
                    );
                }
            }
            if body >= 0.9 {
                push(
                    events,
                    codes::SIGNAL_CANDLE_MARUBOZU,
                    EventType::Signal,
                    10,
                    CANDLE_SOURCE_CODE,
                    timeframe,
                    bar,
                    direction,
                    Some(body),
                    Some(0.9),
                );
            } else if body <= 0.1 {
                push(
                    events,
                    codes::SIGNAL_CANDLE_DOJI,
                    EventType::Signal,
                    10,
                    CANDLE_SOURCE_CODE,
                    timeframe,
                    bar,
                    0,
                    Some(body),
                    Some(0.1),
                );
            }
        }
        if let (Some(upper), Some(lower)) = (
            extended.upper_wick_ratio.get(bar),
            extended.lower_wick_ratio.get(bar),
        ) {
            if upper >= 0.6 && candle.close() < candle.open() {
                push(
                    events,
                    codes::SIGNAL_CANDLE_REJECTION,
                    EventType::Signal,
                    10,
                    CANDLE_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(upper),
                    Some(0.6),
                );
            } else if lower >= 0.6 && candle.close() > candle.open() {
                push(
                    events,
                    codes::SIGNAL_CANDLE_REJECTION,
                    EventType::Signal,
                    10,
                    CANDLE_SOURCE_CODE,
                    timeframe,
                    bar,
                    1,
                    Some(lower),
                    Some(0.6),
                );
            }
        }
        if let Some(code) = extended.pattern_code.get(bar)
            && code != 0.0
        {
            let direction = extended.pattern_direction.get(bar).unwrap_or(0.0) as i8;
            let event_code = match code as u16 {
                1 | 2 => codes::SIGNAL_PATTERN_ENGULFING,
                3 => codes::SIGNAL_PATTERN_INSIDE,
                4 => codes::SIGNAL_PATTERN_OUTSIDE,
                5 => codes::SIGNAL_PATTERN_HAMMER,
                6 => codes::SIGNAL_PATTERN_STAR,
                7 => codes::SIGNAL_PATTERN_THREE_SOLDIERS,
                _ => codes::SIGNAL_PATTERN_THREE_CROWS,
            };
            push(
                events,
                event_code,
                EventType::Signal,
                10,
                PATTERN_SOURCE_CODE,
                timeframe,
                bar,
                direction,
                Some(code),
                Some(0.0),
            );
        }
        if let Some(gap) = extended.gap_atr.get(bar) {
            if gap >= 0.5 && candle.close() > candle.open() {
                push(
                    events,
                    codes::SIGNAL_GAP_CONTINUATION,
                    EventType::Signal,
                    20,
                    GAP_SOURCE_CODE,
                    timeframe,
                    bar,
                    1,
                    Some(gap),
                    Some(0.5),
                );
            } else if gap <= -0.5 && candle.close() < candle.open() {
                push(
                    events,
                    codes::SIGNAL_GAP_FADE,
                    EventType::Signal,
                    20,
                    GAP_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(gap),
                    Some(-0.5),
                );
            }
        }
        if cross_up_value(&extended.path_efficiency, bar, 0.5) {
            push(
                events,
                codes::SIGNAL_PATH_EFFICIENCY,
                EventType::Signal,
                21,
                PATH_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.path_efficiency.get(bar),
                Some(0.5),
            );
        } else if cross_down_value(&extended.path_efficiency, bar, 0.2) {
            push(
                events,
                codes::SIGNAL_PATH_EFFICIENCY,
                EventType::Signal,
                21,
                PATH_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.path_efficiency.get(bar),
                Some(0.2),
            );
        }
        if cross_up_value(&extended.direction_changes, bar, 0.5) {
            push(
                events,
                codes::SIGNAL_DIRECTION_CHANGES,
                EventType::Signal,
                21,
                DIRECTION_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.direction_changes.get(bar),
                Some(0.5),
            );
        } else if cross_down_value(&extended.direction_changes, bar, 0.3) {
            push(
                events,
                codes::SIGNAL_DIRECTION_CHANGES,
                EventType::Signal,
                21,
                DIRECTION_SOURCE_CODE,
                timeframe,
                bar,
                0,
                extended.direction_changes.get(bar),
                Some(0.3),
            );
        }
        if extended.bars_since_high.get(bar) == Some(0.0) {
            push(
                events,
                codes::SIGNAL_EXTREME_NEW,
                EventType::Signal,
                21,
                EXTREME_SOURCE_CODE,
                timeframe,
                bar,
                1,
                Some(0.0),
                Some(0.0),
            );
        } else if extended.bars_since_low.get(bar) == Some(0.0) {
            push(
                events,
                codes::SIGNAL_EXTREME_NEW,
                EventType::Signal,
                21,
                EXTREME_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                Some(0.0),
                Some(0.0),
            );
        } else if config.path_window() > 0 {
            let stale_at = (config.path_window() - 1) as f64;
            let stale_high = extended.bars_since_high.get(bar) == Some(stale_at);
            let stale_low = extended.bars_since_low.get(bar) == Some(stale_at);
            if stale_high || stale_low {
                push(
                    events,
                    codes::SIGNAL_EXTREME_STALE,
                    EventType::Signal,
                    21,
                    EXTREME_SOURCE_CODE,
                    timeframe,
                    bar,
                    0,
                    if stale_high {
                        extended.bars_since_high.get(bar)
                    } else {
                        extended.bars_since_low.get(bar)
                    },
                    Some(stale_at),
                );
            }
        }
        if cross_down_value(&extended.drawdown_atr, bar, -1.0) {
            push(
                events,
                codes::SIGNAL_DRAWDOWN_THRESHOLD,
                EventType::Signal,
                21,
                DRAWDOWN_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                extended.drawdown_atr.get(bar),
                Some(-1.0),
            );
        } else if cross_down_value(&extended.drawdown_atr, bar, -2.0) {
            push(
                events,
                codes::SIGNAL_DRAWDOWN_THRESHOLD,
                EventType::Signal,
                21,
                DRAWDOWN_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                extended.drawdown_atr.get(bar),
                Some(-2.0),
            );
        } else if cross_up_value(&extended.drawdown_atr, bar, -0.5) {
            push(
                events,
                codes::SIGNAL_DRAWDOWN_RECOVERY,
                EventType::Signal,
                21,
                DRAWDOWN_SOURCE_CODE,
                timeframe,
                bar,
                1,
                extended.drawdown_atr.get(bar),
                Some(-0.5),
            );
        }
        if cross_up_value(&extended.range_position, bar, 0.8) {
            push(
                events,
                codes::SIGNAL_RANGE_POSITION,
                EventType::Signal,
                21,
                RANGE_POSITION_SOURCE_CODE,
                timeframe,
                bar,
                1,
                extended.range_position.get(bar),
                Some(0.8),
            );
        } else if cross_down_value(&extended.range_position, bar, 0.2) {
            push(
                events,
                codes::SIGNAL_RANGE_POSITION,
                EventType::Signal,
                21,
                RANGE_POSITION_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                extended.range_position.get(bar),
                Some(0.2),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compute;
    use crate::events::extended::ExtendedEventConfig;
    use crate::events::{EventRecord, codes};
    use crate::readings::{
        ExtendedConfig, ExtendedReadings, ReadingSeries, Readings, ReadingsConfig,
    };
    use quantrule_data::Candle;

    fn candle(index: u64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle::new(index, index as i64, open, high, low, close, Some(10.0)).expect("valid candle")
    }

    fn series() -> Vec<Candle> {
        (0..30)
            .map(|index| candle(index, 100.0, 101.0, 99.0, 100.5))
            .collect()
    }

    fn wide_series() -> Vec<Candle> {
        let mut candles = series();
        candles[29] = candle(29, 101.0, 110.0, 100.0, 109.0);
        candles
    }

    fn rejection_series() -> Vec<Candle> {
        let mut candles = series();
        candles[29] = candle(29, 105.0, 110.0, 100.0, 101.0);
        candles
    }

    fn gap_series(close: f64) -> Vec<Candle> {
        let mut candles = series();
        candles[29] = candle(29, 102.0, 103.0, 101.0, close);
        candles
    }

    fn series_at(len: usize, values: &[(usize, f64)]) -> ReadingSeries {
        let mut data = vec![None; len];
        for (bar, value) in values {
            data[*bar] = Some(*value);
        }
        ReadingSeries::new(data)
    }

    fn events_with(
        candles: &[Candle],
        overrides: impl FnOnce(&mut ExtendedReadings),
    ) -> Vec<EventRecord> {
        let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
        let mut extended = ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT);
        overrides(&mut extended);
        let mut events = Vec::new();
        compute(
            candles,
            &readings,
            &extended,
            &ExtendedEventConfig::from_secs(3600).expect("timeframe"),
            &mut events,
        );
        events
    }

    fn has(events: &[EventRecord], code: u16) -> bool {
        events.iter().any(|event| event.event_code == code)
    }

    #[test]
    fn atr_candle_pattern_and_gap_events_are_exercised() {
        let candles = series();
        let len = candles.len();

        let compression = events_with(&candles, |extended| {
            extended.atr_ratio = series_at(len, &[(28, 0.8), (29, 0.6)]);
        });
        assert!(has(&compression, codes::SIGNAL_ATR_COMPRESSION));

        let expansion = events_with(&candles, |extended| {
            extended.atr_ratio = series_at(len, &[(28, 1.2), (29, 1.4)]);
        });
        assert!(has(&expansion, codes::SIGNAL_ATR_EXPANSION));

        let regime = events_with(&candles, |extended| {
            extended.vol_regime = series_at(len, &[(28, 0.0), (29, 1.0)]);
        });
        assert!(has(&regime, codes::SIGNAL_ATR_REGIME));

        let momentum = events_with(&wide_series(), |extended| {
            extended.body_ratio = series_at(len, &[(29, 0.8)]);
            extended.close_position = series_at(len, &[(29, 0.9)]);
        });
        assert!(has(&momentum, codes::SIGNAL_CANDLE_MOMENTUM));

        let marubozu = events_with(&wide_series(), |extended| {
            extended.body_ratio = series_at(len, &[(29, 0.95)]);
        });
        assert!(has(&marubozu, codes::SIGNAL_CANDLE_MARUBOZU));

        let rejection = events_with(&rejection_series(), |extended| {
            extended.upper_wick_ratio = series_at(len, &[(29, 0.7)]);
            extended.lower_wick_ratio = series_at(len, &[(29, 0.2)]);
        });
        assert!(has(&rejection, codes::SIGNAL_CANDLE_REJECTION));

        for (pattern, code) in [
            (5.0, codes::SIGNAL_PATTERN_HAMMER),
            (6.0, codes::SIGNAL_PATTERN_STAR),
            (7.0, codes::SIGNAL_PATTERN_THREE_SOLDIERS),
            (8.0, codes::SIGNAL_PATTERN_THREE_CROWS),
        ] {
            let patterns = events_with(&candles, |extended| {
                extended.pattern_code = series_at(len, &[(29, pattern)]);
                extended.pattern_direction = series_at(len, &[(29, 1.0)]);
            });
            assert!(has(&patterns, code), "pattern {pattern} did not emit");
        }

        let continuation = events_with(&gap_series(102.5), |extended| {
            extended.gap_atr = series_at(len, &[(29, 0.6)]);
        });
        assert!(has(&continuation, codes::SIGNAL_GAP_CONTINUATION));

        let fade = events_with(&gap_series(101.5), |extended| {
            extended.gap_atr = series_at(len, &[(29, -0.6)]);
        });
        assert!(has(&fade, codes::SIGNAL_GAP_FADE));
    }
}
