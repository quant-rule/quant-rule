//! Session signal events.
//!
//! The five calendar and session readings produce eight signal families and
//! three level families: the session VWAP test, the opening-range test, and the
//! opening-range break, plus the session open and close, the opening-range
//! breakout and fade, the overnight continuation and fade, and the session-range
//! expansion and contraction. The module is pure and deterministic and needs no
//! panel.
//!
//! A session close becomes knowable only on the first bar outside the session,
//! because the last in-session bar cannot be identified from the bar itself. The
//! record therefore occurs on the last in-session bar and is available on the
//! next one.

use crate::events::EventRecord;
use crate::events::EventType;
use crate::events::codes;
use crate::events::extended::push;
use crate::readings::Readings;
use crate::sessions::{SessionConfig, SessionReadings};
use quantrule_data::Candle;

/// Source code for the session state.
pub const SESSION_SOURCE_CODE: u16 = 50;
/// Source code for the opening range.
pub const OPENING_RANGE_SOURCE_CODE: u16 = 51;
/// Source code for the overnight and intraday returns.
pub const OVERNIGHT_SOURCE_CODE: u16 = 52;
/// Source code for the session range.
pub const SESSION_RANGE_SOURCE_CODE: u16 = 53;

/// Signal subtype for a session signal.
const SESSION_SIGNAL_SUBTYPE: u8 = 23;
/// The overnight move that qualifies as a continuation or fade, in ATR.
const OVERNIGHT_THRESHOLD: f64 = 0.5;
/// The session-range expansion threshold, in ATR.
const SESSION_RANGE_EXPANSION: f64 = 1.5;
/// The session-range contraction threshold, in ATR.
const SESSION_RANGE_CONTRACTION: f64 = 0.7;

/// The session signal events over one candle series.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionEventSeries {
    events: Vec<EventRecord>,
}

impl SessionEventSeries {
    /// Computes the session signal events.
    ///
    /// The session readings must have been computed with the same configuration,
    /// so the opening-range width and the session boundaries agree.
    #[must_use]
    pub fn compute(
        candles: &[Candle],
        readings: &Readings,
        sessions: &SessionReadings,
        config: &SessionConfig,
        timeframe_secs: u64,
    ) -> Self {
        let mut events = Vec::new();
        let mut last_inside_range: Option<f64> = None;
        for bar in 0..candles.len() {
            let inside = in_session(sessions, bar);
            let previous_inside = bar > 0 && in_session(sessions, bar - 1);
            let atr = readings.atr14.get(bar).filter(|atr| *atr > 0.0);
            if inside && !previous_inside {
                push(
                    &mut events,
                    codes::SIGNAL_SESSION_OPEN,
                    EventType::Signal,
                    SESSION_SIGNAL_SUBTYPE,
                    SESSION_SOURCE_CODE,
                    timeframe_secs,
                    bar,
                    0,
                    sessions.session_age.get(bar),
                    None,
                );
            }
            if !inside && previous_inside {
                // The last in-session bar is only identifiable here, so the
                // record occurs on that bar and becomes available on this one.
                events.push(EventRecord {
                    event_code: codes::SIGNAL_SESSION_CLOSE,
                    event_type: EventType::Signal,
                    event_subtype_code: SESSION_SIGNAL_SUBTYPE,
                    source_code: SESSION_SOURCE_CODE,
                    timeframe_secs,
                    occurred_bar_index: bar - 1,
                    available_bar_index: bar,
                    direction_code: 0,
                    reference_code: None,
                    primary_value: Some(candles[bar - 1].close()),
                    reference_value: None,
                });
            }
            let complete = sessions
                .session_age
                .get(bar)
                .is_some_and(|age| age >= config.opening_range_bars as f64);
            if inside {
                session_vwap_level(
                    candles[bar].close(),
                    sessions,
                    atr,
                    timeframe_secs,
                    bar,
                    &mut events,
                );
            }
            if inside && complete {
                opening_range_events(candles, sessions, config, timeframe_secs, bar, &mut events);
                opening_range_levels(
                    candles,
                    sessions,
                    atr,
                    config,
                    timeframe_secs,
                    bar,
                    &mut events,
                );
            }
            if inside && sessions.session_age.get(bar) == Some(0.0) {
                overnight_events(sessions, timeframe_secs, bar, &mut events);
            }
            if inside {
                let previous_range = if previous_inside {
                    sessions.session_range_atr.get(bar - 1)
                } else {
                    last_inside_range
                };
                if let (Some(current), Some(previous)) =
                    (sessions.session_range_atr.get(bar), previous_range)
                {
                    if current > SESSION_RANGE_EXPANSION && previous <= SESSION_RANGE_EXPANSION {
                        push(
                            &mut events,
                            codes::SIGNAL_SESSION_RANGE_EXPANSION,
                            EventType::Signal,
                            SESSION_SIGNAL_SUBTYPE,
                            SESSION_RANGE_SOURCE_CODE,
                            timeframe_secs,
                            bar,
                            1,
                            Some(current),
                            Some(SESSION_RANGE_EXPANSION),
                        );
                    } else if current < SESSION_RANGE_CONTRACTION
                        && previous >= SESSION_RANGE_CONTRACTION
                    {
                        push(
                            &mut events,
                            codes::SIGNAL_SESSION_RANGE_CONTRACTION,
                            EventType::Signal,
                            SESSION_SIGNAL_SUBTYPE,
                            SESSION_RANGE_SOURCE_CODE,
                            timeframe_secs,
                            bar,
                            -1,
                            Some(current),
                            Some(SESSION_RANGE_CONTRACTION),
                        );
                    }
                }
                last_inside_range = sessions.session_range_atr.get(bar);
            }
        }
        events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));
        Self { events }
    }

    /// The detected events.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
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
}

fn in_session(sessions: &SessionReadings, bar: usize) -> bool {
    sessions.session_code.get(bar) == Some(1.0)
}

fn session_vwap_level(
    close: f64,
    sessions: &SessionReadings,
    atr: Option<f64>,
    timeframe_secs: u64,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let (Some(vwap), Some(atr)) = (sessions.session_vwap.get(bar), atr) else {
        return;
    };
    if (close - vwap).abs() <= 0.2 * atr {
        push(
            events,
            codes::LEVEL_SESSION_VWAP_TEST,
            EventType::Level,
            1,
            SESSION_SOURCE_CODE,
            timeframe_secs,
            bar,
            0,
            Some(close),
            Some(vwap),
        );
    }
}

fn opening_range_levels(
    candles: &[Candle],
    sessions: &SessionReadings,
    atr: Option<f64>,
    config: &SessionConfig,
    timeframe_secs: u64,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let Some(age) = sessions.session_age.get(bar) else {
        return;
    };
    if age < config.opening_range_bars as f64 {
        return;
    }
    let (Some(high), Some(low), Some(atr)) = (
        sessions.opening_range_high.get(bar),
        sessions.opening_range_low.get(bar),
        atr,
    ) else {
        return;
    };
    let candle = candles[bar];
    if (candle.high() - high).abs() <= 0.2 * atr {
        push(
            events,
            codes::LEVEL_OPENING_RANGE_TEST,
            EventType::Level,
            1,
            OPENING_RANGE_SOURCE_CODE,
            timeframe_secs,
            bar,
            1,
            Some(candle.high()),
            Some(high),
        );
    }
    if (candle.low() - low).abs() <= 0.2 * atr {
        push(
            events,
            codes::LEVEL_OPENING_RANGE_TEST,
            EventType::Level,
            1,
            OPENING_RANGE_SOURCE_CODE,
            timeframe_secs,
            bar,
            -1,
            Some(candle.low()),
            Some(low),
        );
    }
    if bar == 0 {
        return;
    }
    let (Some(previous), Some(previous_high), Some(previous_low)) = (
        candles.get(bar - 1).map(Candle::close),
        sessions.opening_range_high.get(bar - 1),
        sessions.opening_range_low.get(bar - 1),
    ) else {
        return;
    };
    if candle.close() > high && previous <= previous_high {
        push(
            events,
            codes::LEVEL_OPENING_RANGE_BREAK,
            EventType::Level,
            3,
            OPENING_RANGE_SOURCE_CODE,
            timeframe_secs,
            bar,
            1,
            Some(candle.close()),
            Some(high),
        );
    } else if candle.close() < low && previous >= previous_low {
        push(
            events,
            codes::LEVEL_OPENING_RANGE_BREAK,
            EventType::Level,
            3,
            OPENING_RANGE_SOURCE_CODE,
            timeframe_secs,
            bar,
            -1,
            Some(candle.close()),
            Some(low),
        );
    }
}

fn opening_range_events(
    candles: &[Candle],
    sessions: &SessionReadings,
    config: &SessionConfig,
    timeframe_secs: u64,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let Some(age) = sessions.session_age.get(bar) else {
        return;
    };
    if age < config.opening_range_bars as f64 {
        return;
    }
    if let (Some(high), Some(low)) = (
        sessions.opening_range_high.get(bar),
        sessions.opening_range_low.get(bar),
    ) && let (Some(previous), Some(previous_high), Some(previous_low)) = (
        candles.get(bar - 1).map(Candle::close),
        sessions.opening_range_high.get(bar - 1),
        sessions.opening_range_low.get(bar - 1),
    ) {
        let close = candles[bar].close();
        if close > high && previous <= previous_high {
            push(
                events,
                codes::SIGNAL_OPENING_RANGE_BREAKOUT,
                EventType::Signal,
                SESSION_SIGNAL_SUBTYPE,
                OPENING_RANGE_SOURCE_CODE,
                timeframe_secs,
                bar,
                1,
                Some(close),
                Some(high),
            );
        } else if close < low && previous >= previous_low {
            push(
                events,
                codes::SIGNAL_OPENING_RANGE_FADE,
                EventType::Signal,
                SESSION_SIGNAL_SUBTYPE,
                OPENING_RANGE_SOURCE_CODE,
                timeframe_secs,
                bar,
                -1,
                Some(close),
                Some(low),
            );
        }
    }
}

fn overnight_events(
    sessions: &SessionReadings,
    timeframe_secs: u64,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let Some(overnight) = sessions.overnight_return.get(bar) else {
        return;
    };
    if overnight < OVERNIGHT_THRESHOLD {
        return;
    }
    let Some(intraday) = sessions.intraday_return.get(bar) else {
        return;
    };
    if intraday > 0.0 {
        push(
            events,
            codes::SIGNAL_OVERNIGHT_CONTINUATION,
            EventType::Signal,
            SESSION_SIGNAL_SUBTYPE,
            OVERNIGHT_SOURCE_CODE,
            timeframe_secs,
            bar,
            1,
            Some(overnight),
            Some(intraday),
        );
    } else if intraday < 0.0 {
        push(
            events,
            codes::SIGNAL_OVERNIGHT_FADE,
            EventType::Signal,
            SESSION_SIGNAL_SUBTYPE,
            OVERNIGHT_SOURCE_CODE,
            timeframe_secs,
            bar,
            -1,
            Some(overnight),
            Some(intraday),
        );
    }
}

#[cfg(test)]
mod row_tests;

#[cfg(test)]
mod tests {
    use super::SessionEventSeries;
    use crate::readings::{Readings, ReadingsConfig};
    use crate::sessions::{SessionConfig, SessionReadings, days_from_civil};
    use quantrule_data::Candle;

    fn timestamp(year: i64, month: u32, day: u32, hour: u32) -> i64 {
        days_from_civil(year, month, day) * 86_400 + i64::from(hour) * 3_600
    }

    fn series(days: usize) -> Vec<Candle> {
        let start = timestamp(2024, 1, 15, 0);
        let mut candles = Vec::new();
        let mut previous: f64 = 100.0;
        for index in 0..days * 24 {
            let close = 100.0 + index as f64 * 0.5;
            let open = previous;
            let high = open.max(close) + 0.5;
            let low = open.min(close) - 0.5;
            candles.push(
                Candle::new(
                    index as u64,
                    start + index as i64 * 3_600,
                    open,
                    high,
                    low,
                    close,
                    Some(10.0),
                )
                .expect("valid candle"),
            );
            previous = close;
        }
        candles
    }

    fn events(candles: &[Candle]) -> SessionEventSeries {
        let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
        let sessions = SessionReadings::compute(candles, &readings, &SessionConfig::london());
        SessionEventSeries::compute(
            candles,
            &readings,
            &sessions,
            &SessionConfig::london(),
            3_600,
        )
    }

    #[test]
    fn session_open_and_close_are_published() {
        let candles = series(3);
        let events = events(&candles);
        let opens: Vec<usize> = events
            .events()
            .iter()
            .filter(|event| {
                event.event_type == crate::events::EventType::Signal
                    && event.event_subtype_code == 23
                    && event.source_code == 50
                    && event.available_bar_index == event.occurred_bar_index
            })
            .map(|event| event.occurred_bar_index)
            .collect();
        assert_eq!(opens, [8, 32, 56]);
        let closes: Vec<(usize, usize)> = events
            .events()
            .iter()
            .filter(|event| {
                event.event_type == crate::events::EventType::Signal
                    && event.event_subtype_code == 23
                    && event.source_code == 50
                    && event.available_bar_index > event.occurred_bar_index
            })
            .map(|event| (event.occurred_bar_index, event.available_bar_index))
            .collect();
        assert_eq!(closes, [(15, 16), (39, 40), (63, 64)]);
    }

    #[test]
    fn opening_range_breakout_and_overnight_continuation_are_published() {
        let candles = series(3);
        let events = events(&candles);
        assert!(events.events().iter().any(|event| {
            event.source_code == 51 && event.direction_code == 1 && event.available_bar_index == 11
        }));
        assert!(events.events().iter().any(|event| {
            event.source_code == 52 && event.direction_code == 1 && event.occurred_bar_index == 32
        }));
    }

    #[test]
    fn session_level_events_are_published() {
        let candles = series(3);
        let events = events(&candles);
        assert!(events.events().iter().any(|event| {
            event.event_type == crate::events::EventType::Level
                && event.source_code == 50
                && event.event_subtype_code == 1
        }));
        assert!(events.events().iter().any(|event| {
            event.event_type == crate::events::EventType::Level
                && event.source_code == 51
                && event.event_subtype_code == 3
        }));
    }

    #[test]
    fn the_opening_range_test_fires_on_a_touch() {
        let mut candles = series(3);
        let opening_high = candles[9].high();
        let timestamp = candles[14].timestamp();
        candles[14] = Candle::new(
            14,
            timestamp,
            opening_high - 1.0,
            opening_high,
            opening_high - 1.5,
            opening_high - 0.5,
            Some(10.0),
        )
        .expect("the touch candle is valid");
        let events = events(&candles);
        assert!(events.events().iter().any(|event| {
            event.event_type == crate::events::EventType::Level
                && event.source_code == 51
                && event.event_subtype_code == 1
        }));
    }

    #[test]
    fn session_events_are_causal() {
        let candles = series(3);
        let full = events(&candles);
        let prefix = events(&candles[..40]);
        let expected: Vec<_> = full
            .events()
            .iter()
            .filter(|event| event.available_bar_index < 40)
            .map(|event| {
                (
                    event.occurred_bar_index,
                    event.available_bar_index,
                    event.source_code,
                    event.direction_code,
                )
            })
            .collect();
        let observed: Vec<_> = prefix
            .events()
            .iter()
            .map(|event| {
                (
                    event.occurred_bar_index,
                    event.available_bar_index,
                    event.source_code,
                    event.direction_code,
                )
            })
            .collect();
        assert_eq!(expected, observed);
    }

    #[test]
    fn repeated_session_event_computation_is_identical() {
        let candles = series(3);
        assert_eq!(events(&candles), events(&candles));
    }

    #[test]
    fn session_fade_and_range_events_are_exercised() {
        let mut candles = series(2);
        let opening_low = candles[8].low().min(candles[9].low());
        let timestamp = candles[11].timestamp();
        candles[11] = Candle::new(
            11,
            timestamp,
            opening_low + 0.5,
            opening_low + 1.0,
            opening_low - 2.0,
            opening_low - 1.0,
            Some(10.0),
        )
        .expect("the fade candle is valid");
        let faded = events(&candles);
        assert!(
            faded
                .events()
                .iter()
                .any(|event| event.event_code == crate::events::codes::SIGNAL_OPENING_RANGE_FADE)
        );

        let previous_close = candles[31].close();
        let timestamp = candles[32].timestamp();
        candles[32] = Candle::new(
            32,
            timestamp,
            previous_close + 5.0,
            previous_close + 6.0,
            previous_close + 1.0,
            previous_close + 2.0,
            Some(10.0),
        )
        .expect("the overnight-fade candle is valid");
        let first = events(&candles);
        assert!(
            first
                .events()
                .iter()
                .any(|event| event.event_code == crate::events::codes::SIGNAL_OVERNIGHT_FADE)
        );

        let base = candles[32].close();
        for (bar, candle) in candles.iter_mut().enumerate().take(34).skip(32) {
            let timestamp = candle.timestamp();
            *candle = Candle::new(
                bar as u64,
                timestamp,
                base,
                base + 0.2,
                base - 0.2,
                base,
                Some(10.0),
            )
            .expect("the narrow candle is valid");
        }
        let timestamp = candles[34].timestamp();
        candles[34] = Candle::new(
            34,
            timestamp,
            base,
            base + 10.0,
            base - 10.0,
            base,
            Some(10.0),
        )
        .expect("the wide candle is valid");
        let expanded = events(&candles);
        assert!(
            expanded
                .events()
                .iter()
                .any(|event| event.event_code
                    == crate::events::codes::SIGNAL_SESSION_RANGE_EXPANSION)
        );
        assert!(expanded.events().iter().any(
            |event| event.event_code == crate::events::codes::SIGNAL_SESSION_RANGE_CONTRACTION
        ));
    }
}
