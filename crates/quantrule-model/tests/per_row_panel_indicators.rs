//! Row-level contracts for the panel indicator readings.
//!
//! Each test names one indicator and compares every instrument against the
//! checked-in panel readings fixture, then checks prefix causality, determinism,
//! and panel edge cases. The fixture is a regression lock generated from the
//! implementation, not an independent oracle.

use quantrule_data::Candle;
use quantrule_model::panel::{
    PanelConfig, PanelInstrument, PanelInstrumentReadings, PanelReadings,
};

const READINGS_FIXTURE: &str = include_str!("fixtures/panel_readings_golden.csv");
const BARS: usize = 80;
const CUTOFF: usize = 60;

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

fn candle(index: usize, close: f64) -> Candle {
    Candle::new(
        index as u64,
        1_700_000_000 + index as i64 * 3_600,
        close,
        close + 0.5,
        close - 0.5,
        close,
        Some(10.0),
    )
    .expect("the generated panel candle is valid")
}

fn panel(bars: usize) -> (Vec<Candle>, Vec<Candle>, Vec<Candle>) {
    (
        (0..bars)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.2).sin() * 5.0))
            .collect(),
        (0..bars)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.25).sin() * 6.0))
            .collect(),
        (0..bars)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.15).sin() * 3.0))
            .collect(),
    )
}

fn compute(btc: &[Candle], eth: &[Candle], xau: &[Candle]) -> PanelReadings {
    let panel = [
        PanelInstrument {
            name: "BTC",
            candles: btc,
        },
        PanelInstrument {
            name: "ETH",
            candles: eth,
        },
        PanelInstrument {
            name: "XAU",
            candles: xau,
        },
    ];
    PanelReadings::compute(&panel, &PanelConfig::DEFAULT)
}

fn series<'a>(readings: &'a PanelInstrumentReadings, name: &str) -> &'a [Option<f64>] {
    match name {
        "relative_strength" => &readings.relative_strength,
        "lead_lag" => &readings.lead_lag,
        "breadth" => &readings.breadth,
        "co_movement" => &readings.co_movement,
        "dispersion" => &readings.dispersion,
        _ => panic!("unknown panel indicator {name}"),
    }
}

fn assert_panel_indicator(id: &str) {
    let (btc, eth, xau) = panel(BARS);
    let fixture = load_fixture(READINGS_FIXTURE);
    let column = fixture.column(id);
    let full = compute(&btc, &eth, &xau);

    for instrument in ["BTC", "ETH", "XAU"] {
        let readings = full
            .instruments
            .get(instrument)
            .unwrap_or_else(|| panic!("missing instrument {instrument}"));
        let values = series(readings, id);
        let rows: Vec<&Vec<String>> = fixture
            .rows
            .iter()
            .filter(|row| row[0] == instrument)
            .collect();
        assert_eq!(values.len(), rows.len(), "{id} {instrument} length");
        for (bar, row) in rows.iter().enumerate() {
            assert_eq!(
                values[bar].map(f64::to_bits),
                number(&row[column]).map(f64::to_bits),
                "{id} {instrument} changed at bar {bar}"
            );
        }
    }

    let again = compute(&btc, &eth, &xau);
    for instrument in ["BTC", "ETH", "XAU"] {
        assert_eq!(
            full.instruments.get(instrument),
            again.instruments.get(instrument),
            "{id} {instrument} is not deterministic"
        );
    }

    let (prefix_btc, prefix_eth, prefix_xau) = panel(CUTOFF);
    let prefix = compute(&prefix_btc, &prefix_eth, &prefix_xau);
    for instrument in ["BTC", "ETH", "XAU"] {
        let full_values = series(full.instruments.get(instrument).expect("instrument"), id);
        let prefix_values = series(prefix.instruments.get(instrument).expect("instrument"), id);
        for bar in 0..CUTOFF {
            assert_eq!(
                prefix_values[bar].map(f64::to_bits),
                full_values[bar].map(f64::to_bits),
                "{id} {instrument} prefix changed at bar {bar}"
            );
        }
    }

    let empty = PanelReadings::compute(&[], &PanelConfig::DEFAULT);
    assert!(empty.instruments.is_empty());
    assert!(empty.timestamps.is_empty());
    let one = PanelReadings::compute(
        &[PanelInstrument {
            name: "BTC",
            candles: &btc,
        }],
        &PanelConfig::DEFAULT,
    );
    assert!(one.instruments.is_empty());
    let (short_btc, short_eth, short_xau) = panel(10);
    let short = compute(&short_btc, &short_eth, &short_xau);
    for instrument in ["BTC", "ETH", "XAU"] {
        assert_eq!(
            series(short.instruments.get(instrument).expect("instrument"), id).len(),
            10,
            "{id} {instrument} short length"
        );
    }
}

#[test]
fn relative_strength_row_contract() {
    assert_panel_indicator("relative_strength");
}

#[test]
fn lead_lag_row_contract() {
    assert_panel_indicator("lead_lag");
}

#[test]
fn breadth_row_contract() {
    assert_panel_indicator("breadth");
}

#[test]
fn co_movement_row_contract() {
    assert_panel_indicator("co_movement");
}

#[test]
fn dispersion_row_contract() {
    assert_panel_indicator("dispersion");
}
