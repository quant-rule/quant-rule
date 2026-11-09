//! Tests for the published volatility and structure context states.
//!
//! Both states discretise an existing extended reading. These tests pin the
//! pass-through, prove that a bar's code never changes when later bars are
//! appended, and cover the empty, short, gap, and zero-volume cases.

use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::{ExtendedReadings, Readings};
use quantrule_model::timeframe::TimeframeModel;

const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

fn load_fixture_candles() -> Vec<Candle> {
    let mut lines = READINGS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(READINGS_HEADER));
    let mut candles = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23);
        let index = fields[0].parse().expect("bar");
        let timestamp = fields[1].parse().expect("timestamp");
        let open = fields[2].parse().expect("open");
        let high = fields[3].parse().expect("high");
        let low = fields[4].parse().expect("low");
        let close = fields[5].parse().expect("close");
        let volume = fields[6].parse().expect("volume");
        candles.push(
            Candle::new(index, timestamp, open, high, low, close, Some(volume))
                .expect("fixture candle"),
        );
    }
    assert_eq!(candles.len(), 260);
    candles
}

fn model(candles: &[Candle]) -> TimeframeModel {
    TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: 900,
            candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("a non-empty timeframe")
}

fn extended(candles: &[Candle]) -> ExtendedReadings {
    let config = MarketStateConfig::default();
    let readings = Readings::compute_with_config(candles, &config.readings_for(900));
    ExtendedReadings::compute(candles, &readings, &config.extended())
}

fn code(value: Option<f64>) -> Option<i8> {
    match value {
        Some(-1.0) => Some(-1),
        Some(0.0) => Some(0),
        Some(1.0) => Some(1),
        Some(2.0) => Some(2),
        _ => None,
    }
}

fn states(model: &TimeframeModel, bar: usize) -> (Option<i8>, Option<i8>) {
    let states = model.states_at(bar).expect("a bar inside the series");
    (states.vol_regime, states.structure_state)
}

#[test]
fn the_published_states_match_the_extended_codes() {
    let candles = load_fixture_candles();
    let timeframe = model(&candles);
    let extended = extended(&candles);
    for bar in 0..candles.len() {
        assert_eq!(
            timeframe.states_at(bar).expect("a bar").vol_regime,
            code(extended.vol_regime.get(bar)),
            "vol_regime at bar {bar}"
        );
        assert_eq!(
            timeframe.states_at(bar).expect("a bar").structure_state,
            code(extended.structure_state.get(bar)),
            "structure_state at bar {bar}"
        );
    }
}

#[test]
fn later_bars_never_change_an_earlier_state() {
    let candles = load_fixture_candles();
    let full = model(&candles);
    for length in 2..=candles.len() {
        let prefix = model(&candles[..length]);
        for bar in 0..length {
            assert_eq!(
                states(&prefix, bar),
                states(&full, bar),
                "bar {bar} changed when the series was truncated to {length}"
            );
        }
    }
}

#[test]
fn the_states_are_deterministic_across_repeated_runs() {
    let candles = load_fixture_candles();
    let first = model(&candles);
    let second = model(&candles);
    for bar in 0..candles.len() {
        assert_eq!(states(&first, bar), states(&second, bar));
    }
}

#[test]
fn an_empty_series_has_no_model() {
    assert!(
        TimeframeModel::compute(
            &TimeframeInput {
                timeframe_secs: 900,
                candles: &[],
            },
            &MarketStateConfig::default(),
        )
        .is_none()
    );
}

#[test]
fn a_single_candle_has_no_context_states() {
    let timeframe = model(&load_fixture_candles()[..1]);
    assert_eq!(states(&timeframe, 0), (None, None));
}

#[test]
fn a_short_series_has_no_context_states() {
    let timeframe = model(&load_fixture_candles()[..10]);
    for bar in 0..10 {
        assert_eq!(states(&timeframe, bar), (None, None));
    }
}

#[test]
fn a_time_gap_keeps_the_state_aligned_with_the_extended_reading() {
    let mut candles = load_fixture_candles();
    let timestamp = candles[120].timestamp();
    candles[120] = Candle::new(
        candles[120].series_index(),
        timestamp + 7 * 900,
        candles[120].open(),
        candles[120].high(),
        candles[120].low(),
        candles[120].close(),
        candles[120].volume(),
    )
    .expect("the gapped candle is valid");
    let timeframe = model(&candles);
    let extended = extended(&candles);
    for bar in 0..candles.len() {
        assert_eq!(
            timeframe.states_at(bar).expect("a bar").vol_regime,
            code(extended.vol_regime.get(bar)),
            "vol_regime at bar {bar}"
        );
        assert_eq!(
            timeframe.states_at(bar).expect("a bar").structure_state,
            code(extended.structure_state.get(bar)),
            "structure_state at bar {bar}"
        );
    }
}

#[test]
fn zero_volume_does_not_change_the_volatility_or_structure_state() {
    let candles: Vec<Candle> = load_fixture_candles()
        .into_iter()
        .map(|candle| {
            Candle::new(
                candle.series_index(),
                candle.timestamp(),
                candle.open(),
                candle.high(),
                candle.low(),
                candle.close(),
                Some(0.0),
            )
            .expect("the zero-volume candle is valid")
        })
        .collect();
    let timeframe = model(&candles);
    let extended = extended(&candles);
    for bar in 0..candles.len() {
        assert_eq!(
            timeframe.states_at(bar).expect("a bar").vol_regime,
            code(extended.vol_regime.get(bar)),
            "vol_regime at bar {bar}"
        );
        assert_eq!(
            timeframe.states_at(bar).expect("a bar").structure_state,
            code(extended.structure_state.get(bar)),
            "structure_state at bar {bar}"
        );
    }
}
