//! DST-aware session and calendar readings.
//!
//! The calendar uses explicit London and New York DST rules, not an ambient
//! timezone. London uses the last Sunday of March and October at 01:00 UTC; New
//! York uses the second Sunday of March at 07:00 UTC and the first Sunday of
//! November at 06:00 UTC. The UTC zone has no DST. The module is pure.

use std::collections::BTreeMap;

use quantrule_data::Candle;

use crate::readings::{ReadingSeries, Readings};

/// A session zone with explicit DST rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionZone {
    /// UTC, no DST.
    Utc,
    /// Europe/London, BST from the last Sunday of March to the last Sunday of October.
    London,
    /// America/New_York, EDT from the second Sunday of March to the first Sunday of November.
    NewYork,
}

/// The session configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionConfig {
    /// The zone.
    pub zone: SessionZone,
    /// Local session start hour, inclusive.
    pub start_hour: u32,
    /// Local session end hour, exclusive.
    pub end_hour: u32,
    /// Bars in the opening range.
    pub opening_range_bars: usize,
}

impl SessionConfig {
    /// London 08:00–16:00 local, two-bar opening range.
    #[must_use]
    pub const fn london() -> Self {
        Self {
            zone: SessionZone::London,
            start_hour: 8,
            end_hour: 16,
            opening_range_bars: 2,
        }
    }

    /// New York 09:00–16:00 local, two-bar opening range.
    #[must_use]
    pub const fn new_york() -> Self {
        Self {
            zone: SessionZone::NewYork,
            start_hour: 9,
            end_hour: 16,
            opening_range_bars: 2,
        }
    }

    /// The UTC day 00:00–24:00, two-bar opening range.
    #[must_use]
    pub const fn utc() -> Self {
        Self {
            zone: SessionZone::Utc,
            start_hour: 0,
            end_hour: 24,
            opening_range_bars: 2,
        }
    }
}

/// The session and calendar readings for one bar.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionReadings {
    /// One inside the session, zero outside.
    pub session_code: ReadingSeries,
    /// Bars since the session start.
    pub session_age: ReadingSeries,
    /// The opening-range high.
    pub opening_range_high: ReadingSeries,
    /// The opening-range low.
    pub opening_range_low: ReadingSeries,
    /// The opening-range width in ATR.
    pub opening_range_atr: ReadingSeries,
    /// The session range so far in ATR.
    pub session_range_atr: ReadingSeries,
    /// The session-anchored VWAP.
    pub session_vwap: ReadingSeries,
    /// The overnight return in ATR.
    pub overnight_return: ReadingSeries,
    /// The intraday return in ATR.
    pub intraday_return: ReadingSeries,
    /// `day of week * 100 + month`.
    pub calendar_bucket: ReadingSeries,
}

impl SessionReadings {
    /// Computes the session and calendar readings.
    #[must_use]
    pub fn compute(candles: &[Candle], readings: &Readings, config: &SessionConfig) -> Self {
        let len = candles.len();
        let mut session_code = vec![None; len];
        let mut session_age = vec![None; len];
        let mut opening_range_high = vec![None; len];
        let mut opening_range_low = vec![None; len];
        let mut opening_range_atr = vec![None; len];
        let mut session_range_atr = vec![None; len];
        let mut session_vwap = vec![None; len];
        let mut overnight_return = vec![None; len];
        let mut intraday_return = vec![None; len];
        let mut calendar_bucket = vec![None; len];
        let mut days: BTreeMap<i64, DayState> = BTreeMap::new();
        let mut last_session_close: Option<f64> = None;
        for bar in 0..len {
            let candle = candles[bar];
            let local = candle.timestamp() + offset_seconds(candle.timestamp(), config.zone);
            let day = local.div_euclid(86_400);
            let hour = (local.rem_euclid(86_400) / 3_600) as u32;
            let (_, month, _) = civil_from_days(day);
            let weekday = (day + 4).rem_euclid(7) as f64;
            calendar_bucket[bar] = Some(weekday * 100.0 + month as f64);
            if hour < config.start_hour || hour >= config.end_hour {
                continue;
            }
            let atr = readings.atr14.get(bar).filter(|atr| *atr > 0.0);
            let state = days.entry(day).or_insert_with(|| DayState {
                open: candle.open(),
                high: candle.high(),
                low: candle.low(),
                opening_high: f64::NEG_INFINITY,
                opening_low: f64::INFINITY,
                bars: 0,
                overnight: None,
                vwap_weighted: 0.0,
                vwap_volume: 0.0,
            });
            if state.bars == 0 {
                state.open = candle.open();
                state.high = candle.high();
                state.low = candle.low();
                if let (Some(previous), Some(atr)) = (last_session_close, atr) {
                    state.overnight = Some((candle.open() - previous) / atr);
                }
            }
            if state.bars < config.opening_range_bars {
                state.opening_high = state.opening_high.max(candle.high());
                state.opening_low = state.opening_low.min(candle.low());
            }
            state.high = state.high.max(candle.high());
            state.low = state.low.min(candle.low());
            state.bars = state.bars.saturating_add(1);
            let volume = candle.volume().unwrap_or(0.0);
            let typical = (candle.high() + candle.low() + candle.close()) / 3.0;
            state.vwap_weighted += typical * volume;
            state.vwap_volume += volume;
            let values = result_values(state, candle, atr);
            session_code[bar] = Some(1.0);
            session_age[bar] = values.age;
            opening_range_high[bar] = values.opening_high;
            opening_range_low[bar] = values.opening_low;
            opening_range_atr[bar] = values.opening_atr;
            session_range_atr[bar] = values.session_atr;
            if state.vwap_volume > 0.0 {
                session_vwap[bar] = Some(state.vwap_weighted / state.vwap_volume);
            }
            overnight_return[bar] = state.overnight;
            intraday_return[bar] = values.intraday;
            last_session_close = Some(candle.close());
        }
        Self {
            session_code: ReadingSeries::new(session_code),
            session_age: ReadingSeries::new(session_age),
            opening_range_high: ReadingSeries::new(opening_range_high),
            opening_range_low: ReadingSeries::new(opening_range_low),
            opening_range_atr: ReadingSeries::new(opening_range_atr),
            session_range_atr: ReadingSeries::new(session_range_atr),
            session_vwap: ReadingSeries::new(session_vwap),
            overnight_return: ReadingSeries::new(overnight_return),
            intraday_return: ReadingSeries::new(intraday_return),
            calendar_bucket: ReadingSeries::new(calendar_bucket),
        }
    }
}

struct DayState {
    open: f64,
    high: f64,
    low: f64,
    opening_high: f64,
    opening_low: f64,
    bars: usize,
    overnight: Option<f64>,
    vwap_weighted: f64,
    vwap_volume: f64,
}

struct Values {
    age: Option<f64>,
    opening_high: Option<f64>,
    opening_low: Option<f64>,
    opening_atr: Option<f64>,
    session_atr: Option<f64>,
    intraday: Option<f64>,
}

fn result_values(state: &DayState, candle: Candle, atr: Option<f64>) -> Values {
    // The age is bars since the session opened, so a 15-minute series and an
    // hourly series both count bars rather than clock hours.
    let age = Some(state.bars.saturating_sub(1) as f64);
    let opening_high = state.opening_high.is_finite().then_some(state.opening_high);
    let opening_low = state.opening_low.is_finite().then_some(state.opening_low);
    let opening_atr = opening_high
        .zip(opening_low)
        .zip(atr)
        .map(|((high, low), atr)| (high - low) / atr);
    let session_atr = atr.map(|atr| (state.high - state.low) / atr);
    let intraday = atr.map(|atr| (candle.close() - state.open) / atr);
    Values {
        age,
        opening_high,
        opening_low,
        opening_atr,
        session_atr,
        intraday,
    }
}

fn offset_seconds(timestamp: i64, zone: SessionZone) -> i64 {
    match zone {
        SessionZone::Utc => 0,
        SessionZone::London => {
            let day = timestamp.div_euclid(86_400);
            let (year, _, _) = civil_from_days(day);
            let start = days_from_civil(year, 3, last_sunday(year, 3)) * 86_400 + 3_600;
            let end = days_from_civil(year, 10, last_sunday(year, 10)) * 86_400 + 3_600;
            if timestamp >= start && timestamp < end {
                3_600
            } else {
                0
            }
        }
        SessionZone::NewYork => {
            let day = timestamp.div_euclid(86_400);
            let (year, _, _) = civil_from_days(day);
            let start = days_from_civil(year, 3, nth_sunday(year, 3, 2)) * 86_400 + 7 * 3_600;
            let end = days_from_civil(year, 11, nth_sunday(year, 11, 1)) * 86_400 + 6 * 3_600;
            if timestamp >= start && timestamp < end {
                -4 * 3_600
            } else {
                -5 * 3_600
            }
        }
    }
}

fn last_sunday(year: i64, month: u32) -> u32 {
    let last = days_in_month(year, month);
    let day = days_from_civil(year, month, last);
    let weekday = (day + 4).rem_euclid(7) as u32;
    last - weekday
}

fn nth_sunday(year: i64, month: u32, n: u32) -> u32 {
    let first = days_from_civil(year, month, 1);
    let weekday = (first + 4).rem_euclid(7) as u32;
    let offset = (7 - weekday) % 7;
    1 + offset + 7 * (n - 1)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let days = days_from_civil(next_year, next_month, 1) - days_from_civil(year, month, 1);
    days as u32
}

pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (year + i64::from(month <= 2), month, day)
}

pub(crate) fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = (year - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 } as u64;
    let doy = (153 * mp + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

#[cfg(test)]
mod tests {
    use super::{SessionConfig, SessionReadings, SessionZone, days_from_civil, offset_seconds};
    use crate::readings::{Readings, ReadingsConfig};
    use quantrule_data::Candle;

    fn timestamp(year: i64, month: u32, day: u32, hour: u32) -> i64 {
        days_from_civil(year, month, day) * 86_400 + i64::from(hour) * 3_600
    }

    fn candle(index: usize, ts: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle::new(index as u64, ts, open, high, low, close, Some(10.0)).expect("candle")
    }

    #[test]
    fn london_dst_starts_and_ends_at_the_right_utc_hour() {
        assert_eq!(
            offset_seconds(timestamp(2024, 3, 31, 0), SessionZone::London),
            0
        );
        assert_eq!(
            offset_seconds(timestamp(2024, 3, 31, 1), SessionZone::London),
            3_600
        );
        assert_eq!(
            offset_seconds(timestamp(2024, 10, 27, 0), SessionZone::London),
            3_600
        );
        assert_eq!(
            offset_seconds(timestamp(2024, 10, 27, 1), SessionZone::London),
            0
        );
    }

    #[test]
    fn new_york_dst_starts_and_ends_at_the_right_utc_hour() {
        assert_eq!(
            offset_seconds(timestamp(2024, 3, 10, 6), SessionZone::NewYork),
            -5 * 3_600
        );
        assert_eq!(
            offset_seconds(timestamp(2024, 3, 10, 7), SessionZone::NewYork),
            -4 * 3_600
        );
        assert_eq!(
            offset_seconds(timestamp(2024, 11, 3, 5), SessionZone::NewYork),
            -4 * 3_600
        );
        assert_eq!(
            offset_seconds(timestamp(2024, 11, 3, 6), SessionZone::NewYork),
            -5 * 3_600
        );
    }

    #[test]
    fn london_session_and_opening_range() {
        let mut candles = Vec::new();
        for hour in 8..12 {
            let ts = timestamp(2024, 1, 15, hour);
            candles.push(candle(
                candles.len(),
                ts,
                100.0 + f64::from(hour - 8),
                101.0 + f64::from(hour - 8),
                99.0 + f64::from(hour - 8),
                100.5 + f64::from(hour - 8),
            ));
        }
        let readings = Readings::compute_with_config(&candles, &ReadingsConfig::DEFAULT);
        let readings = SessionReadings::compute(&candles, &readings, &SessionConfig::london());
        assert_eq!(readings.session_code.get(0), Some(1.0));
        assert_eq!(readings.session_age.get(0), Some(0.0));
        assert_eq!(readings.opening_range_high.get(1), Some(102.0));
        assert!(readings.session_range_atr.get(3).is_none());
        assert_eq!(readings.calendar_bucket.get(0), Some(101.0));
    }
}
