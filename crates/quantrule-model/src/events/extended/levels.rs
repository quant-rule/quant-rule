//! Extended level event detection.

use super::*;

pub(super) fn level_extended_events(
    candles: &[Candle],
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let candle = candles[bar];
    let close = candle.close();
    let timeframe = config.timeframe_secs();
    let atr = readings.atr14.get(bar).filter(|atr| *atr > 0.0);
    if let (Some(upper), Some(lower), Some(atr)) = (
        extended.keltner_upper.get(bar),
        extended.keltner_lower.get(bar),
        atr,
    ) {
        if (close - upper).abs() <= config.level_atr * atr
            || (close - lower).abs() <= config.level_atr * atr
        {
            push(
                events,
                codes::LEVEL_KELTNER_TEST,
                EventType::Level,
                1,
                KELTNER_SOURCE_CODE,
                timeframe,
                bar,
                0,
                Some(close),
                Some(if (close - upper).abs() <= (close - lower).abs() {
                    upper
                } else {
                    lower
                }),
            );
        }
        if bar > 0
            && let (Some(previous_close), Some(previous_upper), Some(previous_lower)) = (
                candles.get(bar - 1).map(Candle::close),
                extended.keltner_upper.get(bar - 1),
                extended.keltner_lower.get(bar - 1),
            )
        {
            if close > upper && previous_close <= previous_upper {
                push(
                    events,
                    codes::LEVEL_KELTNER_BREAK,
                    EventType::Level,
                    3,
                    KELTNER_SOURCE_CODE,
                    timeframe,
                    bar,
                    1,
                    Some(close),
                    Some(upper),
                );
            } else if close < lower && previous_close >= previous_lower {
                push(
                    events,
                    codes::LEVEL_KELTNER_BREAK,
                    EventType::Level,
                    3,
                    KELTNER_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(close),
                    Some(lower),
                );
            }
        }
    }
    if let (Some(upper), Some(lower), Some(atr)) = (
        extended.donchian_upper.get(bar),
        extended.donchian_lower.get(bar),
        atr,
    ) {
        if (candle.high() - upper).abs() <= config.level_atr * atr
            || (candle.low() - lower).abs() <= config.level_atr * atr
        {
            push(
                events,
                codes::LEVEL_DONCHIAN_TEST,
                EventType::Level,
                1,
                DONCHIAN_SOURCE_CODE,
                timeframe,
                bar,
                0,
                Some(candle.high()),
                Some(upper),
            );
        }
        if bar > 0
            && let (Some(previous_close), Some(previous_upper), Some(previous_lower)) = (
                candles.get(bar - 1).map(Candle::close),
                extended.donchian_upper.get(bar - 1),
                extended.donchian_lower.get(bar - 1),
            )
        {
            if close > previous_upper && previous_close <= previous_upper {
                push(
                    events,
                    codes::LEVEL_DONCHIAN_BREAK,
                    EventType::Level,
                    3,
                    DONCHIAN_SOURCE_CODE,
                    timeframe,
                    bar,
                    1,
                    Some(close),
                    Some(previous_upper),
                );
            } else if close < previous_lower && previous_close >= previous_lower {
                push(
                    events,
                    codes::LEVEL_DONCHIAN_BREAK,
                    EventType::Level,
                    3,
                    DONCHIAN_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(close),
                    Some(previous_lower),
                );
            }
        }
    }
    if let (Some(high), Some(low)) = (
        extended.last_pivot_high.get(bar),
        extended.last_pivot_low.get(bar),
    ) && bar > 0
        && let (Some(previous_close), Some(previous_high), Some(previous_low)) = (
            candles.get(bar - 1).map(Candle::close),
            extended.last_pivot_high.get(bar - 1),
            extended.last_pivot_low.get(bar - 1),
        )
    {
        if close > high && previous_close <= previous_high {
            push(
                events,
                codes::LEVEL_STRUCTURE_BREAK,
                EventType::Level,
                3,
                SWING_SOURCE_CODE,
                timeframe,
                bar,
                1,
                Some(close),
                Some(high),
            );
        } else if close < low && previous_close >= previous_low {
            push(
                events,
                codes::LEVEL_STRUCTURE_BREAK,
                EventType::Level,
                3,
                SWING_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                Some(close),
                Some(low),
            );
        }
    }
    if bar > 0
        && let (Some(high), Some(low), Some(previous_state)) = (
            extended.last_pivot_high.get(bar),
            extended.last_pivot_low.get(bar),
            extended.structure_state.get(bar - 1),
        )
        && let (Some(previous_close), Some(previous_high), Some(previous_low)) = (
            candles.get(bar - 1).map(Candle::close),
            extended.last_pivot_high.get(bar - 1),
            extended.last_pivot_low.get(bar - 1),
        )
    {
        // The prior structure state classifies the break: with the trend is a
        // break of structure, against it is a change of character. A neutral
        // prior state is unclassified and emits neither.
        if close > high && previous_close <= previous_high && previous_state != 0.0 {
            let subtype = if previous_state > 0.0 { 4 } else { 5 };
            let event_code = if subtype == 4 {
                codes::LEVEL_BOS
            } else {
                codes::LEVEL_CHOCH
            };
            push(
                events,
                event_code,
                EventType::Level,
                subtype,
                BOS_CHOCH_SOURCE_CODE,
                timeframe,
                bar,
                1,
                Some(close),
                Some(high),
            );
        } else if close < low && previous_close >= previous_low && previous_state != 0.0 {
            let subtype = if previous_state < 0.0 { 4 } else { 5 };
            let event_code = if subtype == 4 {
                codes::LEVEL_BOS
            } else {
                codes::LEVEL_CHOCH
            };
            push(
                events,
                event_code,
                EventType::Level,
                subtype,
                BOS_CHOCH_SOURCE_CODE,
                timeframe,
                bar,
                -1,
                Some(close),
                Some(low),
            );
        }
    }
    if let Some(touches) = extended.level_touches.get(bar)
        && touches >= 2.0
    {
        push(
            events,
            codes::LEVEL_STRENGTH_TEST,
            EventType::Level,
            1,
            STRENGTH_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(touches),
            extended.last_pivot_high.get(bar),
        );
    }
    if let (Some(upper_wick), Some(high), Some(atr)) = (
        extended.upper_wick_ratio.get(bar),
        extended.last_pivot_high.get(bar),
        atr,
    ) && upper_wick >= 0.6
        && (candle.high() - high).abs() <= config.level_atr * atr
    {
        push(
            events,
            codes::LEVEL_WICK_TEST,
            EventType::Level,
            1,
            CANDLE_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(upper_wick),
            Some(high),
        );
    }
    if let (Some(lower_wick), Some(low), Some(atr)) = (
        extended.lower_wick_ratio.get(bar),
        extended.last_pivot_low.get(bar),
        atr,
    ) && lower_wick >= 0.6
        && (candle.low() - low).abs() <= config.level_atr * atr
    {
        push(
            events,
            codes::LEVEL_WICK_TEST,
            EventType::Level,
            1,
            CANDLE_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(lower_wick),
            Some(low),
        );
    }
    if extended.gap_filled.get(bar) == Some(1.0) {
        push(
            events,
            codes::LEVEL_GAP_FILL,
            EventType::Level,
            1,
            GAP_SOURCE_CODE,
            timeframe,
            bar,
            0,
            extended.gap_atr.get(bar),
            Some(0.0),
        );
    } else if let Some(gap) = extended.gap_atr.get(bar)
        && gap.abs() <= 0.2
    {
        push(
            events,
            codes::LEVEL_GAP_TEST,
            EventType::Level,
            1,
            GAP_SOURCE_CODE,
            timeframe,
            bar,
            0,
            Some(gap),
            Some(0.0),
        );
    }
}
