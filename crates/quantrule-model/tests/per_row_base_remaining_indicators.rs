//! Row-level contracts for the remaining base indicators: MACD, ADX, and DMI.
//!
//! The fixture is a checked-in regression lock generated from the 260-bar
//! readings fixture. It is not an independent oracle.

use quantrule_data::Candle;
use quantrule_model::readings::{AdxSeries, MacdSeries, ReadingSeries, Readings, ReadingsConfig};

const CANDLES_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const FLOW_FIXTURE: &str = include_str!("fixtures/base_flow_golden.csv");
const CANDLES_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const FLOW_HEADER: &str = "bar,macd_line,macd_signal,macd_histogram,adx,di_plus,di_minus";
const CUTOFF: usize = 200;

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

fn load_candles() -> Vec<Candle> {
    let mut lines = CANDLES_FIXTURE.lines();
    assert_eq!(lines.next(), Some(CANDLES_HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 23);
            Candle::new(
                fields[0].parse().expect("bar"),
                fields[1].parse().expect("timestamp"),
                number(fields[2]).expect("open"),
                number(fields[3]).expect("high"),
                number(fields[4]).expect("low"),
                number(fields[5]).expect("close"),
                number(fields[6]),
            )
            .expect("valid fixture candle")
        })
        .collect()
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

fn macd(candles: &[Candle]) -> MacdSeries {
    let config = ReadingsConfig::DEFAULT;
    MacdSeries::compute(
        candles,
        config.macd_fast,
        config.macd_slow,
        config.macd_signal,
    )
}

fn adx(candles: &[Candle]) -> AdxSeries {
    AdxSeries::compute(candles, ReadingsConfig::DEFAULT.adx)
}

fn render(candles: &[Candle]) -> String {
    let macd = macd(candles);
    let adx = adx(candles);
    let optional = |value: Option<f64>| value.map(|value| value.to_string()).unwrap_or_default();
    let mut csv = String::from(FLOW_HEADER);
    csv.push('\n');
    for bar in 0..candles.len() {
        let values = [
            bar.to_string(),
            optional(macd.line.get(bar)),
            optional(macd.signal.get(bar)),
            optional(macd.histogram.get(bar)),
            optional(adx.adx.get(bar)),
            optional(adx.di_plus.get(bar)),
            optional(adx.di_minus.get(bar)),
        ];
        csv.push_str(&values.join(","));
        csv.push('\n');
    }
    csv
}

fn assert_column(series: &ReadingSeries, fixture: &Fixture, column: usize, label: &str) {
    assert_eq!(series.len(), fixture.rows.len(), "{label} length changed");
    for (bar, row) in fixture.rows.iter().enumerate() {
        assert_eq!(
            series.get(bar).map(f64::to_bits),
            number(&row[column]).map(f64::to_bits),
            "{label} changed at bar {bar}"
        );
    }
}

fn assert_columns<F>(id: &str, columns: F)
where
    F: Fn(&[Candle]) -> Vec<(&'static str, ReadingSeries)>,
{
    let candles = load_candles();
    let fixture = load_fixture(FLOW_FIXTURE);
    assert_eq!(fixture.rows.len(), candles.len());
    let full = columns(&candles);
    for (name, series) in &full {
        let column = fixture.column(name);
        assert_column(series, &fixture, column, &format!("{id}.{name}"));
    }

    let again = columns(&candles);
    for ((name, series), (again_name, again_series)) in full.iter().zip(&again) {
        assert_eq!(name, again_name);
        assert_eq!(series, again_series, "{id}.{name} is not deterministic");
    }

    let prefix = columns(&candles[..CUTOFF]);
    for ((name, series), (prefix_name, prefix_series)) in full.iter().zip(&prefix) {
        assert_eq!(name, prefix_name);
        for bar in 0..CUTOFF {
            assert_eq!(
                prefix_series.get(bar).map(f64::to_bits),
                series.get(bar).map(f64::to_bits),
                "{id}.{name} prefix changed at bar {bar}"
            );
        }
    }

    let empty = columns(&[]);
    let single = columns(&candles[..1]);
    let short = columns(&candles[..10]);
    for (((name, series), (_, empty_series)), ((_, single_series), (_, short_series))) in
        full.iter().zip(&empty).zip(single.iter().zip(&short))
    {
        assert_eq!(empty_series.len(), 0, "{id}.{name} is not empty");
        assert_eq!(single_series.len(), 1, "{id}.{name} single length");
        assert_eq!(short_series.len(), 10, "{id}.{name} short length");
        assert_eq!(series.len(), candles.len(), "{id}.{name} full length");
    }
}

#[test]
fn macd_row_contract() {
    assert_columns("macd", |candles| {
        let series = macd(candles);
        vec![
            ("macd_line", series.line.clone()),
            ("macd_signal", series.signal.clone()),
            ("macd_histogram", series.histogram.clone()),
        ]
    });
}

#[test]
fn adx_row_contract() {
    assert_columns("adx", |candles| {
        let series = adx(candles);
        vec![("adx", series.adx.clone())]
    });
}

#[test]
fn dmi_row_contract() {
    assert_columns("dmi", |candles| {
        let series = adx(candles);
        vec![
            ("di_plus", series.di_plus.clone()),
            ("di_minus", series.di_minus.clone()),
        ]
    });
}

#[test]
#[ignore = "regenerates the base flow fixture deliberately"]
fn regenerate_base_flow_fixture() {
    let candles = load_candles();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/base_flow_golden.csv"
    );
    std::fs::write(path, render(&candles)).expect("the fixture is writable");
}

#[test]
fn the_base_flow_fixture_is_current() {
    let candles = load_candles();
    assert_eq!(render(&candles), FLOW_FIXTURE);
    assert_eq!(Readings::compute(&candles).len(), candles.len());
}
