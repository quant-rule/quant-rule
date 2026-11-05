//! Golden, causality, and determinism tests for the panel readings.
//!
//! The fixture is a checked-in CSV of three synthetic instruments and the five
//! cross-asset readings. It is a regression lock generated from the
//! implementation, not an independent oracle.

use quantrule_data::Candle;
use quantrule_model::panel::{PanelConfig, PanelInstrument, PanelReadings};

const READINGS_FIXTURE: &str = include_str!("fixtures/panel_readings_golden.csv");
const HEADER: &str =
    "instrument,bar,relative_strength,leader_return,lead_lag,breadth,co_movement,dispersion";
const BARS: usize = 80;
const CUTOFF: usize = 60;

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

fn panel() -> (Vec<Candle>, Vec<Candle>, Vec<Candle>) {
    (
        (0..BARS)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.2).sin() * 5.0))
            .collect(),
        (0..BARS)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.25).sin() * 6.0))
            .collect(),
        (0..BARS)
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

fn optional(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn render(readings: &PanelReadings) -> String {
    let mut csv = String::from(HEADER);
    csv.push('\n');
    for instrument in ["BTC", "ETH", "XAU"] {
        let Some(series) = readings.instruments.get(instrument) else {
            continue;
        };
        for bar in 0..readings.timestamps.len() {
            let values = [
                instrument.to_string(),
                bar.to_string(),
                optional(series.relative_strength.get(bar).copied().flatten()),
                optional(series.leader_return.get(bar).copied().flatten()),
                optional(series.lead_lag.get(bar).copied().flatten()),
                optional(series.breadth.get(bar).copied().flatten()),
                optional(series.co_movement.get(bar).copied().flatten()),
                optional(series.dispersion.get(bar).copied().flatten()),
            ];
            csv.push_str(&values.join(","));
            csv.push('\n');
        }
    }
    csv
}

#[test]
fn the_panel_readings_match_the_golden_fixture() {
    let (btc, eth, xau) = panel();
    let readings = compute(&btc, &eth, &xau);
    assert_eq!(render(&readings), READINGS_FIXTURE);
}

#[test]
fn appending_bars_does_not_change_past_panel_readings() {
    let (btc, eth, xau) = panel();
    let full = render(&compute(&btc, &eth, &xau));
    let prefix = render(&compute(&btc[..CUTOFF], &eth[..CUTOFF], &xau[..CUTOFF]));
    let expected: Vec<&str> = full.lines().take(1 + CUTOFF).collect();
    let observed: Vec<&str> = prefix.lines().take(1 + CUTOFF).collect();
    assert_eq!(expected, observed, "a past panel reading changed");
}

#[test]
fn repeated_panel_computation_is_identical() {
    let (btc, eth, xau) = panel();
    assert_eq!(compute(&btc, &eth, &xau), compute(&btc, &eth, &xau));
}

#[test]
fn an_empty_panel_has_only_the_header() {
    let readings = PanelReadings::compute(&[], &PanelConfig::DEFAULT);
    assert_eq!(render(&readings), format!("{HEADER}\n"));
}

#[test]
#[ignore = "regenerates the panel readings fixture deliberately"]
fn regenerate_panel_readings_fixture() {
    let (btc, eth, xau) = panel();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/panel_readings_golden.csv"
    );
    std::fs::write(path, render(&compute(&btc, &eth, &xau))).expect("the fixture is writable");
}
