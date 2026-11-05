//! Golden, causality, and determinism tests for the session readings.
//!
//! The fixture is a checked-in CSV of three London sessions and the ten session
//! readings. It is a regression lock generated from the implementation, not an
//! independent oracle.

use quantrule_data::Candle;
use quantrule_model::readings::{Readings, ReadingsConfig};
use quantrule_model::sessions::{SessionConfig, SessionReadings};

const READINGS_FIXTURE: &str = include_str!("fixtures/session_readings_golden.csv");
const HEADER: &str = "bar,timestamp,session_code,session_age,opening_range_high,opening_range_low,opening_range_atr,session_range_atr,session_vwap,overnight_return,intraday_return,calendar_bucket";
const BASE: i64 = 1_705_276_800;
const BARS: usize = 72;
const CUTOFF: usize = 40;

fn series() -> Vec<Candle> {
    let mut candles = Vec::new();
    let mut previous: f64 = 100.0;
    for index in 0..BARS {
        let close = 100.0 + index as f64 * 0.5;
        let open = previous;
        let high = open.max(close) + 0.5;
        let low = open.min(close) - 0.5;
        candles.push(
            Candle::new(
                index as u64,
                BASE + index as i64 * 3_600,
                open,
                high,
                low,
                close,
                Some(10.0),
            )
            .expect("the generated session candle is valid"),
        );
        previous = close;
    }
    candles
}

fn compute(candles: &[Candle]) -> SessionReadings {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    SessionReadings::compute(candles, &readings, &SessionConfig::london())
}

fn optional(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn render(candles: &[Candle], sessions: &SessionReadings) -> String {
    let mut csv = String::from(HEADER);
    csv.push('\n');
    for (bar, candle) in candles.iter().enumerate() {
        let values = [
            bar.to_string(),
            candle.timestamp().to_string(),
            optional(sessions.session_code.get(bar)),
            optional(sessions.session_age.get(bar)),
            optional(sessions.opening_range_high.get(bar)),
            optional(sessions.opening_range_low.get(bar)),
            optional(sessions.opening_range_atr.get(bar)),
            optional(sessions.session_range_atr.get(bar)),
            optional(sessions.session_vwap.get(bar)),
            optional(sessions.overnight_return.get(bar)),
            optional(sessions.intraday_return.get(bar)),
            optional(sessions.calendar_bucket.get(bar)),
        ];
        csv.push_str(&values.join(","));
        csv.push('\n');
    }
    csv
}

#[test]
fn the_session_readings_match_the_golden_fixture() {
    let candles = series();
    assert_eq!(render(&candles, &compute(&candles)), READINGS_FIXTURE);
}

#[test]
fn appending_bars_does_not_change_past_session_readings() {
    let candles = series();
    let full = render(&candles, &compute(&candles));
    let prefix = render(&candles[..CUTOFF], &compute(&candles[..CUTOFF]));
    let expected: Vec<&str> = full.lines().take(CUTOFF + 1).collect();
    let observed: Vec<&str> = prefix.lines().collect();
    assert_eq!(expected, observed, "a past session reading changed");
}

#[test]
fn repeated_session_computation_is_identical() {
    let candles = series();
    assert_eq!(compute(&candles), compute(&candles));
}

#[test]
fn an_empty_series_has_only_the_header() {
    let readings = Readings::compute(&[]);
    let sessions = SessionReadings::compute(&[], &readings, &SessionConfig::london());
    assert_eq!(render(&[], &sessions), format!("{HEADER}\n"));
}

#[test]
#[ignore = "regenerates the session readings fixture deliberately"]
fn regenerate_session_readings_fixture() {
    let candles = series();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/session_readings_golden.csv"
    );
    std::fs::write(path, render(&candles, &compute(&candles))).expect("the fixture is writable");
}
