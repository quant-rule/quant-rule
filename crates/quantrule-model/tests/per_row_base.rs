//! Row-level contracts for the base EMA, RSI, and ATR readings.
//!
//! Each test covers formula/golden values, causality, determinism, and the
//! empty/single/short edge cases for one indicator. The fixture is the checked-in
//! scalar readings golden table.

use quantrule_data::Candle;
use quantrule_model::readings::{ReadingSeries, Readings};

const FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

struct Row {
    candle: Candle,
    ema20: Option<f64>,
    ema50: Option<f64>,
    ema200: Option<f64>,
    rsi14: Option<f64>,
    atr14: Option<f64>,
}

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(field.parse().expect("a fixture number"))
    }
}

fn load() -> Vec<Row> {
    let mut lines = FIXTURE.lines();
    assert_eq!(lines.next(), Some(HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 23);
            Row {
                candle: Candle::new(
                    fields[0].parse().expect("bar"),
                    fields[1].parse().expect("timestamp"),
                    number(fields[2]).expect("open"),
                    number(fields[3]).expect("high"),
                    number(fields[4]).expect("low"),
                    number(fields[5]).expect("close"),
                    number(fields[6]),
                )
                .expect("valid candle"),
                ema20: number(fields[7]),
                ema50: number(fields[8]),
                ema200: number(fields[9]),
                rsi14: number(fields[10]),
                atr14: number(fields[11]),
            }
        })
        .collect()
}

fn candles(rows: &[Row]) -> Vec<Candle> {
    rows.iter().map(|row| row.candle).collect()
}

fn assert_close(actual: Option<f64>, expected: Option<f64>, label: &str) {
    match (actual, expected) {
        (None, None) => {}
        (Some(actual), Some(expected)) => {
            assert!(
                (actual - expected).abs() <= 1e-12,
                "{label}: {actual} != {expected}"
            );
        }
        _ => panic!("{label}: {actual:?} != {expected:?}"),
    }
}

fn assert_series(actual: &ReadingSeries, expected: &[Option<f64>], label: &str) {
    assert_eq!(actual.len(), expected.len());
    for (bar, expected) in expected.iter().enumerate() {
        assert_close(actual.get(bar), *expected, &format!("{label} at {bar}"));
    }
}

fn assert_prefix_and_determinism(
    rows: &[Row],
    series: impl Fn(&Readings) -> ReadingSeries,
    label: &str,
) {
    let all = candles(rows);
    let full = Readings::compute(&all);
    let full_series = series(&full);
    for cutoff in [61, 131, 201] {
        let prefix = Readings::compute(&all[..cutoff]);
        let prefix_series = series(&prefix);
        for bar in 0..cutoff {
            assert_close(
                prefix_series.get(bar),
                full_series.get(bar),
                &format!("{label} prefix at {bar}/{cutoff}"),
            );
        }
    }
    assert_eq!(
        Readings::compute(&all),
        full,
        "{label} is not deterministic"
    );
}

#[test]
fn ema_row_contract() {
    let rows = load();
    let all = candles(&rows);
    let readings = Readings::compute(&all);
    assert_series(
        &readings.ema20,
        &rows.iter().map(|row| row.ema20).collect::<Vec<_>>(),
        "ema20",
    );
    assert_series(
        &readings.ema50,
        &rows.iter().map(|row| row.ema50).collect::<Vec<_>>(),
        "ema50",
    );
    assert_series(
        &readings.ema200,
        &rows.iter().map(|row| row.ema200).collect::<Vec<_>>(),
        "ema200",
    );
    assert_prefix_and_determinism(&rows, |readings| readings.ema20.clone(), "ema20");
    assert_prefix_and_determinism(&rows, |readings| readings.ema50.clone(), "ema50");
    assert_prefix_and_determinism(&rows, |readings| readings.ema200.clone(), "ema200");

    let empty = Readings::compute(&[]);
    assert!(empty.ema20.is_empty());
    let one = Readings::compute(&all[..1]);
    assert_eq!(one.ema20.get(0), None);
    let short = Readings::compute(&all[..19]);
    assert!(short.ema20.values().iter().all(Option::is_none));
}

#[test]
fn rsi_row_contract() {
    let rows = load();
    let all = candles(&rows);
    let readings = Readings::compute(&all);
    assert_series(
        &readings.rsi14,
        &rows.iter().map(|row| row.rsi14).collect::<Vec<_>>(),
        "rsi14",
    );
    assert_prefix_and_determinism(&rows, |readings| readings.rsi14.clone(), "rsi14");

    let empty = Readings::compute(&[]);
    assert!(empty.rsi14.is_empty());
    let one = Readings::compute(&all[..1]);
    assert_eq!(one.rsi14.get(0), None);
    let short = Readings::compute(&all[..14]);
    assert!(short.rsi14.values().iter().all(Option::is_none));
}

#[test]
fn atr_row_contract() {
    let rows = load();
    let all = candles(&rows);
    let readings = Readings::compute(&all);
    assert_series(
        &readings.atr14,
        &rows.iter().map(|row| row.atr14).collect::<Vec<_>>(),
        "atr14",
    );
    assert_prefix_and_determinism(&rows, |readings| readings.atr14.clone(), "atr14");

    let empty = Readings::compute(&[]);
    assert!(empty.atr14.is_empty());
    let one = Readings::compute(&all[..1]);
    assert_eq!(one.atr14.get(0), None);
    let short = Readings::compute(&all[..14]);
    assert!(short.atr14.values().iter().all(Option::is_none));
}
