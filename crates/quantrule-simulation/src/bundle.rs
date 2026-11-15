//! The recorded model bundle: bars, events, and the catalog.
//!
//! A simulation stage never reruns the model. It reads the readings, events, and
//! levels that one recorded report published and works over them. This module
//! holds the shared value types and the deterministic helpers every stage uses:
//! the bar and event records, the event catalog, UTC timestamp formatting, and
//! the correctly rounded mean.
//!
//! The types mirror the retired Python harness bundle schema field for field;
//! the harness was removed by DIST-105 and `PORT-001` keeps its evidence. A bar
//! carries the readings its stages read; an event carries its identity, both
//! bars, its direction, and its measured reference values. The bundle orders
//! bars by `(timestamp, bar)` and events by `(available_bar, event_index)`, so
//! a stage result does not depend on the order the caller supplied.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against.

// port-parity: PORT-001

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The event-code to registry-name map.
pub type Catalog = BTreeMap<u32, String>;

/// One model reading row for a single bar.
///
/// The optional fields are `None` when the model published no value. A missing
/// reading is never replaced by zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Bar {
    /// The bar identity inside the merged bundle.
    pub bar: u64,
    /// The bar open time, in UTC seconds.
    pub timestamp: i64,
    /// Open.
    pub open: f64,
    /// High.
    pub high: f64,
    /// Low.
    pub low: f64,
    /// Close.
    pub close: f64,
    /// Volume.
    pub volume: f64,
    /// Average true range, when the model published one.
    pub atr: Option<f64>,
    /// The fast exponential moving average.
    pub ema_fast: Option<f64>,
    /// The mid exponential moving average.
    pub ema_mid: Option<f64>,
    /// The slow exponential moving average.
    pub ema_slow: Option<f64>,
    /// Relative strength index.
    pub rsi: Option<f64>,
    /// Relative volume.
    pub rvol: Option<f64>,
    /// Average directional index.
    pub adx: Option<f64>,
    /// The EMA stack code: -1 bear, 0 mixed, 1 bull.
    pub ema_stack: Option<i32>,
    /// The RSI zone code: -1 oversold, 0 middle, 1 overbought.
    pub rsi_zone: Option<i32>,
    /// ATR as a ratio to its reference.
    pub atr_ratio: Option<f64>,
    /// Where the close sat inside the bar, from 0 to 1.
    pub close_position: Option<f64>,
    /// Upper wick as a fraction of the range.
    pub upper_wick: Option<f64>,
    /// Lower wick as a fraction of the range.
    pub lower_wick: Option<f64>,
    /// Body as a fraction of the range.
    pub body_ratio: Option<f64>,
    /// The last confirmed pivot low.
    pub last_pivot_low: Option<f64>,
    /// The last confirmed pivot high.
    pub last_pivot_high: Option<f64>,
}

impl Bar {
    /// Return a published reading by its report column name.
    ///
    /// The simulation configurations filter on these columns. A column the
    /// bundle does not carry is absent, never zero.
    #[must_use]
    pub fn value(&self, column: &str) -> Option<f64> {
        match column {
            "close_position" => self.close_position,
            "upper_wick_ratio" => self.upper_wick,
            "lower_wick_ratio" => self.lower_wick,
            "body_ratio" => self.body_ratio,
            "last_pivot_low" => self.last_pivot_low,
            "last_pivot_high" => self.last_pivot_high,
            "rsi" => self.rsi,
            "rvol" => self.rvol,
            "adx" => self.adx,
            "atr_ratio" => self.atr_ratio,
            _ => None,
        }
    }

    /// Return a bar with every optional reading absent and the given prices.
    ///
    /// This is the shape the parity fixtures use for a sparse projection: the
    /// stage under test reads only the fields it names, and an absent field is
    /// reported as absent rather than as zero.
    #[must_use]
    pub fn from_prices(
        bar: u64,
        timestamp: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    ) -> Self {
        Self {
            bar,
            timestamp,
            open,
            high,
            low,
            close,
            volume: 0.0,
            atr: None,
            ema_fast: None,
            ema_mid: None,
            ema_slow: None,
            rsi: None,
            rvol: None,
            adx: None,
            ema_stack: None,
            rsi_zone: None,
            atr_ratio: None,
            close_position: None,
            upper_wick: None,
            lower_wick: None,
            body_ratio: None,
            last_pivot_low: None,
            last_pivot_high: None,
        }
    }
}

/// One published event record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Event {
    /// The event identity inside its report.
    pub event_index: u64,
    /// The event code.
    pub code: u32,
    /// The event type code.
    pub type_code: u32,
    /// The event subtype code.
    pub subtype_code: u32,
    /// The source code.
    pub source_code: u32,
    /// The bar on which the event occurred.
    pub occurred_bar: u64,
    /// The first bar on which the event became knowable.
    pub available_bar: u64,
    /// The direction code.
    pub direction: i32,
    /// The reference code, when the event names one.
    pub reference_code: Option<i64>,
    /// The measured primary value.
    pub primary: Option<f64>,
    /// The measured reference value.
    pub reference: Option<f64>,
    /// The data completeness at the event bar.
    pub completeness: Option<f64>,
}

impl Event {
    /// The registry name of the event, or a stable `event.<code>` fallback.
    #[must_use]
    pub fn label(&self, catalog: &Catalog) -> String {
        catalog
            .get(&self.code)
            .cloned()
            .unwrap_or_else(|| format!("event.{}", self.code))
    }
}

/// One model bundle, reduced to the bars and events a stage reads.
///
/// Construction sorts and indexes the records, so the same set supplied in any
/// order produces the same bundle. An event whose available bar is not in the
/// bundle is dropped, matching the reference loader.
#[derive(Debug, Clone)]
pub struct Bundle {
    /// The bar timeframe, in seconds.
    pub timeframe_secs: u64,
    /// The bars, ordered by timestamp and bar identity.
    pub bars: Vec<Bar>,
    /// The events, ordered by available bar and event index.
    pub events: Vec<Event>,
    /// The recorded provenance, used for the data version.
    pub provenance: BTreeMap<String, String>,
    bar_index: BTreeMap<u64, usize>,
    events_by_bar: BTreeMap<u64, Vec<Event>>,
}

impl Bundle {
    /// Build a bundle, ordering and indexing its records.
    ///
    /// # Panics
    ///
    /// Does not panic. A bar identity that repeats is reported by the caller's
    /// fixture decoding; the bundle keeps the first occurrence.
    #[must_use]
    pub fn new(timeframe_secs: u64, mut bars: Vec<Bar>, mut events: Vec<Event>) -> Self {
        bars.sort_by_key(|bar| (bar.timestamp, bar.bar));
        let bar_index: BTreeMap<u64, usize> = bars
            .iter()
            .enumerate()
            .map(|(index, bar)| (bar.bar, index))
            .collect();
        events.retain(|event| bar_index.contains_key(&event.available_bar));
        events.sort_by_key(|event| (event.available_bar, event.event_index));
        let mut events_by_bar: BTreeMap<u64, Vec<Event>> = BTreeMap::new();
        for event in &events {
            events_by_bar
                .entry(event.available_bar)
                .or_default()
                .push(*event);
        }
        Self {
            timeframe_secs,
            bars,
            events,
            provenance: BTreeMap::new(),
            bar_index,
            events_by_bar,
        }
    }

    /// Set the recorded data version.
    #[must_use]
    pub fn with_data_version(mut self, version: impl Into<String>) -> Self {
        self.provenance
            .insert("data_version".to_owned(), version.into());
        self
    }

    /// The recorded data version, or an explicit unknown marker.
    #[must_use]
    pub fn data_version(&self) -> &str {
        self.provenance
            .get("data_version")
            .map_or("unknown", String::as_str)
    }

    /// Return the list index for a bar identity.
    #[must_use]
    pub fn index_of(&self, bar: u64) -> Option<usize> {
        self.bar_index.get(&bar).copied()
    }

    /// Return the events available at one bar identity.
    #[must_use]
    pub fn events_at(&self, bar: u64) -> &[Event] {
        self.events_by_bar.get(&bar).map_or(&[], Vec::as_slice)
    }

    /// The first bar timestamp, or `None` for an empty bundle.
    #[must_use]
    pub fn first_timestamp(&self) -> Option<i64> {
        self.bars.first().map(|bar| bar.timestamp)
    }

    /// The last bar timestamp, or `None` for an empty bundle.
    #[must_use]
    pub fn last_timestamp(&self) -> Option<i64> {
        self.bars.last().map(|bar| bar.timestamp)
    }
}

/// Why a UTC string cannot be parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimestampError;

impl std::fmt::Display for TimestampError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a timestamp must be an ISO UTC date or date-time")
    }
}

impl std::error::Error for TimestampError {}

/// Format UTC seconds as the reference's ISO timestamp.
#[must_use]
pub fn utc_from_seconds(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    let second = remainder % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Parse an ISO UTC date or date-time into UTC seconds.
///
/// The accepted forms are `YYYY-MM-DD`, `YYYY-MM-DDTHH:MM`,
/// `YYYY-MM-DDTHH:MM:SS`, optionally followed by `Z` or a `+HH:MM` / `-HH:MM`
/// offset, and the same with a space separator and fractional seconds. This is
/// the subset the simulation configurations and marks use.
///
/// # Errors
///
/// Returns [`TimestampError`] when the text is not one of those forms.
pub fn parse_utc(value: &str) -> Result<i64, TimestampError> {
    let text = value.trim();
    let (date, time) = match text.split_once(['T', ' ']) {
        Some((date, time)) => (date, Some(time)),
        None => (text, None),
    };
    let mut parts = date.split('-');
    let year: i64 = parts
        .next()
        .ok_or(TimestampError)?
        .parse()
        .map_err(|_| TimestampError)?;
    let month: u32 = parts
        .next()
        .ok_or(TimestampError)?
        .parse()
        .map_err(|_| TimestampError)?;
    let day: u32 = parts
        .next()
        .ok_or(TimestampError)?
        .parse()
        .map_err(|_| TimestampError)?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(TimestampError);
    }
    let mut seconds = days_from_civil(year, month, day)
        .checked_mul(86_400)
        .ok_or(TimestampError)?;
    if let Some(time) = time {
        let (clock, offset) = split_offset(time)?;
        let mut fields = clock.split(':');
        let hour: i64 = fields
            .next()
            .ok_or(TimestampError)?
            .parse()
            .map_err(|_| TimestampError)?;
        let minute: i64 = fields
            .next()
            .ok_or(TimestampError)?
            .parse()
            .map_err(|_| TimestampError)?;
        let second: i64 = match fields.next() {
            Some(value) => {
                let whole = value.split('.').next().ok_or(TimestampError)?;
                whole.parse().map_err(|_| TimestampError)?
            }
            None => 0,
        };
        if fields.next().is_some() || hour > 23 || minute > 59 || second > 59 {
            return Err(TimestampError);
        }
        seconds = seconds
            .checked_add(hour * 3_600 + minute * 60 + second - offset)
            .ok_or(TimestampError)?;
    }
    Ok(seconds)
}

fn split_offset(time: &str) -> Result<(&str, i64), TimestampError> {
    if let Some(clock) = time.strip_suffix('Z') {
        return Ok((clock, 0));
    }
    let bytes = time.as_bytes();
    let index = bytes
        .iter()
        .rposition(|byte| *byte == b'+' || *byte == b'-')
        .ok_or(TimestampError)?;
    // The date-time separator is consumed by the split; a leading sign is not
    // an offset, so require at least one clock character before it.
    if index == 0 {
        return Ok((time, 0));
    }
    let (clock, offset) = time.split_at(index);
    let sign = if offset.starts_with('+') { 1 } else { -1 };
    let mut fields = offset[1..].split(':');
    let hours: i64 = fields
        .next()
        .ok_or(TimestampError)?
        .parse()
        .map_err(|_| TimestampError)?;
    let minutes: i64 = fields
        .next()
        .ok_or(TimestampError)?
        .parse()
        .map_err(|_| TimestampError)?;
    if fields.next().is_some() || hours > 23 || minutes > 59 {
        return Err(TimestampError);
    }
    Ok((clock, sign * (hours * 3_600 + minutes * 60)))
}

/// Convert days since the Unix epoch to a civil date.
///
/// Howard Hinnant's `civil_from_days`, exact for every date the system records
/// and free of a clock or calendar dependency.
#[must_use]
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

/// Convert a civil date to days since the Unix epoch.
#[must_use]
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day = i64::from(day);
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The UTC year of a timestamp.
#[must_use]
pub fn utc_year(timestamp: i64) -> i64 {
    civil_from_days(timestamp.div_euclid(86_400)).0
}

/// The UTC half-year bucket of a timestamp, such as `2020H1`.
#[must_use]
pub fn half_year(timestamp: i64) -> String {
    let days = timestamp.div_euclid(86_400);
    let (year, month, _) = civil_from_days(days);
    format!("{year}H{}", if month <= 6 { 1 } else { 2 })
}

/// The correctly rounded sum of a float list.
///
/// This is the partials algorithm the reference's `math.fsum` uses, so a mean
/// does not depend on the order the values were supplied and matches the Python
/// value bit for bit.
#[must_use]
pub fn fsum(values: &[f64]) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for &value in values {
        let mut x = value;
        let mut kept = 0;
        for index in 0..partials.len() {
            let mut y = partials[index];
            if x.abs() < y.abs() {
                std::mem::swap(&mut x, &mut y);
            }
            let hi = x + y;
            let lo = y - (hi - x);
            if lo != 0.0 {
                partials[kept] = lo;
                kept += 1;
            }
            x = hi;
        }
        partials.truncate(kept);
        partials.push(x);
    }
    partials.iter().sum()
}

/// The correctly rounded arithmetic mean, or `None` for an empty list.
#[must_use]
pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(fsum(values) / values.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_round_trip_through_the_utc_format() {
        for seconds in [0, 1_514_764_800, 1_577_750_400, 1_672_530_300] {
            let text = utc_from_seconds(seconds);
            assert_eq!(parse_utc(&text), Ok(seconds), "{text}");
        }
        assert_eq!(utc_from_seconds(1_514_764_800), "2018-01-01T00:00:00Z");
        assert_eq!(utc_from_seconds(1_577_750_400), "2019-12-31T00:00:00Z");
    }

    #[test]
    fn a_date_only_timestamp_is_midnight_utc() {
        assert_eq!(parse_utc("2018-01-01"), Ok(1_514_764_800));
        assert_eq!(parse_utc("2020-01-01"), Ok(1_577_836_800));
        assert_eq!(parse_utc("2022-12-31"), Ok(1_672_444_800));
    }

    #[test]
    fn an_offset_is_applied() {
        assert_eq!(parse_utc("2018-01-01T01:00:00+01:00"), Ok(1_514_764_800));
        assert_eq!(parse_utc("2017-12-31T23:00:00-01:00"), Ok(1_514_764_800));
    }

    #[test]
    fn a_malformed_timestamp_is_refused() {
        assert_eq!(parse_utc("2018-13-01"), Err(TimestampError));
        assert_eq!(parse_utc("not a date"), Err(TimestampError));
    }

    #[test]
    fn the_half_year_bucket_is_utc() {
        assert_eq!(half_year(1_514_764_800), "2018H1");
        assert_eq!(half_year(1_530_403_200), "2018H2");
    }

    #[test]
    fn fsum_is_order_independent() {
        let values = [0.1, 0.2, 0.3, 1e-12, -0.1, 1e12, -1e12];
        let mut reversed = values;
        reversed.reverse();
        assert_eq!(fsum(&values), fsum(&reversed));
        assert_eq!(fsum(&[0.1, 0.2, 0.3]), 0.6);
        assert_eq!(mean(&[]), None);
    }

    #[test]
    fn a_bundle_orders_and_indexes_its_records() {
        let bars = vec![
            Bar::from_prices(2, 1_800, 1.0, 1.0, 1.0, 1.0),
            Bar::from_prices(1, 900, 1.0, 1.0, 1.0, 1.0),
        ];
        let events = vec![Event {
            event_index: 7,
            code: 149,
            type_code: 1,
            subtype_code: 1,
            source_code: 1,
            occurred_bar: 1,
            available_bar: 1,
            direction: 0,
            reference_code: None,
            primary: None,
            reference: None,
            completeness: None,
        }];
        let bundle = Bundle::new(900, bars, events);
        assert_eq!(bundle.bars[0].bar, 1);
        assert_eq!(bundle.index_of(2), Some(1));
        assert_eq!(bundle.events_at(1).len(), 1);
        assert!(bundle.events_at(9).is_empty());
    }

    #[test]
    fn an_event_without_its_bar_is_dropped() {
        let event = Event {
            event_index: 0,
            code: 1,
            type_code: 1,
            subtype_code: 1,
            source_code: 1,
            occurred_bar: 90,
            available_bar: 90,
            direction: 0,
            reference_code: None,
            primary: None,
            reference: None,
            completeness: None,
        };
        let bundle = Bundle::new(
            900,
            vec![Bar::from_prices(1, 900, 1.0, 1.0, 1.0, 1.0)],
            vec![event],
        );
        assert!(bundle.events.is_empty());
    }
}
