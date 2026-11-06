//! VWAP event detection.

use super::*;

pub(super) fn vwap_events(
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
            codes::LEVEL_VWAP_TEST,
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
                codes::LEVEL_VWAP_CROSS,
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
                codes::LEVEL_VWAP_BREAK,
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
    // The mirror is a downward reclaim after the same number of bars above.
    if bar >= config.reclaim_bars {
        let mut below = true;
        let mut above = true;
        for (index, prior) in candles
            .iter()
            .enumerate()
            .take(bar)
            .skip(bar - config.reclaim_bars)
        {
            let Some(prior_vwap) = extended.vwap.get(index) else {
                below = false;
                above = false;
                break;
            };
            if prior.close() >= prior_vwap {
                below = false;
            }
            if prior.close() <= prior_vwap {
                above = false;
            }
            if !below && !above {
                break;
            }
        }
        if let Some(prior_vwap) = extended.vwap.get(bar - 1) {
            let previous_close = candles[bar - 1].close();
            if below && previous_close <= prior_vwap && close > vwap {
                push(
                    events,
                    codes::SIGNAL_VWAP_RECLAIM,
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
            if above && previous_close >= prior_vwap && close < vwap {
                push(
                    events,
                    codes::SIGNAL_VWAP_RECLAIM,
                    EventType::Signal,
                    14,
                    VWAP_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(distance),
                    Some(vwap),
                );
            }
        }
    }

    // Rejection: a wick through VWAP that closes back on the original side.
    if candle.high() >= vwap && close < vwap && close < candle.open() {
        push(
            events,
            codes::SIGNAL_VWAP_REJECTION,
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
            codes::SIGNAL_VWAP_REJECTION,
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
            codes::SIGNAL_VWAP_SLOPE,
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
