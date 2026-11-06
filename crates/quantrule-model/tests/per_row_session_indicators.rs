//! Row-level contracts for the session indicator readings.
//!
//! Each test names one indicator and compares its registered outputs against the
//! checked-in session readings fixture, then checks prefix causality,
//! determinism, and empty/single/short safety. The fixture is a regression lock
//! generated from the implementation, not an independent oracle.

use quantrule_data::Candle;
use quantrule_model::readings::{ReadingSeries, Readings, ReadingsConfig};
use quantrule_model::sessions::{SessionConfig, SessionReadings};

const READINGS_FIXTURE: &str = include_str!("fixtures/session_readings_golden.csv");
const BASE: i64 = 1_705_276_800;
const BARS: usize = 72;
const CUTOFF: usize = 40;

struct Fixture {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Fixture {
    fn column(&self, name: &str) -> usize {
        self.header
            .iter()
            .position(|column| column == name)
            .unwrap_or_else(|| panic!("the fixture has no column {name}"))
    }
}

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(field.parse().expect("a fixture number"))
    }
}

fn load_fixture(text: &str) -> Fixture {
    let mut lines = text.lines();
    let header = lines
        .next()
        .expect("the fixture header")
        .split(',')
        .map(str::to_string)
        .collect();
    let rows = lines
        .map(|line| line.split(',').map(str::to_string).collect())
        .collect();
    Fixture { header, rows }
}

fn candles(bars: usize) -> Vec<Candle> {
    let mut candles = Vec::new();
    let mut previous: f64 = 100.0;
    for index in 0..bars {
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

type Columns = Vec<(&'static str, ReadingSeries)>;

fn assert_session_indicator(id: &str, columns: impl Fn(&SessionReadings) -> Columns) {
    let candles = candles(BARS);
    let fixture = load_fixture(READINGS_FIXTURE);
    assert_eq!(fixture.rows.len(), candles.len());

    let full = compute(&candles);
    let full_columns = columns(&full);
    for (name, series) in &full_columns {
        let column = fixture.column(name);
        assert_eq!(series.len(), fixture.rows.len(), "{id}.{name} length");
        for (bar, row) in fixture.rows.iter().enumerate() {
            assert_eq!(
                series.get(bar).map(f64::to_bits),
                number(&row[column]).map(f64::to_bits),
                "{id}.{name} changed at bar {bar}"
            );
        }
    }

    let again = columns(&compute(&candles));
    for ((name, series), (again_name, again_series)) in full_columns.iter().zip(&again) {
        assert_eq!(name, again_name);
        for bar in 0..series.len() {
            assert_eq!(
                again_series.get(bar).map(f64::to_bits),
                series.get(bar).map(f64::to_bits),
                "{id}.{name} is not deterministic at bar {bar}"
            );
        }
    }

    let prefix = columns(&compute(&candles[..CUTOFF]));
    for ((name, series), (prefix_name, prefix_series)) in full_columns.iter().zip(&prefix) {
        assert_eq!(name, prefix_name);
        for bar in 0..CUTOFF {
            assert_eq!(
                prefix_series.get(bar).map(f64::to_bits),
                series.get(bar).map(f64::to_bits),
                "{id}.{name} prefix changed at bar {bar}"
            );
        }
    }

    let empty = columns(&compute(&[]));
    let single = columns(&compute(&candles[..1]));
    let short = columns(&compute(&candles[..10]));
    for (((name, series), (_, empty_series)), ((_, single_series), (_, short_series))) in
        full_columns
            .iter()
            .zip(&empty)
            .zip(single.iter().zip(&short))
    {
        assert_eq!(empty_series.len(), 0, "{id}.{name} is not empty");
        assert_eq!(single_series.len(), 1, "{id}.{name} single length");
        assert_eq!(short_series.len(), 10, "{id}.{name} short length");
        assert_eq!(series.len(), BARS, "{id}.{name} full length");
    }
}

#[test]
fn session_row_contract() {
    assert_session_indicator("session", |sessions| {
        vec![
            ("session_code", sessions.session_code.clone()),
            ("session_age", sessions.session_age.clone()),
        ]
    });
}

#[test]
fn opening_range_row_contract() {
    assert_session_indicator("opening_range", |sessions| {
        vec![
            ("opening_range_high", sessions.opening_range_high.clone()),
            ("opening_range_low", sessions.opening_range_low.clone()),
            ("opening_range_atr", sessions.opening_range_atr.clone()),
        ]
    });
}

#[test]
fn overnight_intraday_row_contract() {
    assert_session_indicator("overnight_intraday", |sessions| {
        vec![
            ("overnight_return", sessions.overnight_return.clone()),
            ("intraday_return", sessions.intraday_return.clone()),
        ]
    });
}

#[test]
fn session_range_row_contract() {
    assert_session_indicator("session_range", |sessions| {
        vec![("session_range_atr", sessions.session_range_atr.clone())]
    });
}

#[test]
fn calendar_bucket_row_contract() {
    assert_session_indicator("calendar_bucket", |sessions| {
        vec![("calendar_bucket", sessions.calendar_bucket.clone())]
    });
}
