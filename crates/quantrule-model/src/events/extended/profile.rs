//! Volume-profile event detection.

use super::{
    Candle, EventRecord, EventType, ExtendedEventConfig, ExtendedReadings, PROFILE_SOURCE_CODE,
    Readings, codes, push,
};
pub(super) fn profile_events(
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
            codes::LEVEL_PROFILE_VAH_TEST,
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
            codes::LEVEL_PROFILE_VAL_TEST,
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
            codes::LEVEL_PROFILE_POC_TEST,
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
                    codes::LEVEL_PROFILE_VA_BREAK,
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
                    codes::LEVEL_PROFILE_VA_BREAK,
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
                    codes::LEVEL_PROFILE_VA_REENTRY,
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
                    codes::LEVEL_PROFILE_VA_REENTRY,
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
                codes::SIGNAL_PROFILE_VA_BREAKOUT,
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
                    codes::SIGNAL_PROFILE_POC_CROSS,
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
                codes::SIGNAL_PROFILE_SKEW,
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
                codes::SIGNAL_PROFILE_CONCENTRATION,
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
