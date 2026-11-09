//! Tests for the `rvol_regime` context state.
//!
//! The state is a pure discretisation of the existing RVOL20 reading. These
//! tests pin the contract thresholds, prove that a bar's code never changes
//! when later bars are appended, and cover the empty, short, zero-volume,
//! missing-volume, and gap cases.

use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
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

fn candle(index: usize, timestamp: i64, volume: Option<f64>) -> Candle {
    Candle::new(index as u64, timestamp, 10.0, 10.0, 10.0, 10.0, volume)
        .expect("the test candle is valid")
}

fn candles_with_volumes(volumes: &[Option<f64>]) -> Vec<Candle> {
    volumes
        .iter()
        .enumerate()
        .map(|(index, volume)| candle(index, index as i64 * 3_600, *volume))
        .collect()
}

fn model(candles: &[Candle]) -> TimeframeModel {
    TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: 3_600,
            candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("a non-empty timeframe")
}

fn states(model: &TimeframeModel) -> Vec<Option<i8>> {
    (0..model.len())
        .map(|bar| {
            model
                .states_at(bar)
                .expect("a bar inside the series")
                .rvol_regime
        })
        .collect()
}

#[test]
fn the_fixture_codes_match_the_contract() {
    let timeframe = model(&load_fixture_candles());
    let cases = [
        (0, None),
        (18, None),
        (19, Some(-1)),
        (22, Some(1)),
        (90, Some(0)),
        (104, Some(0)),
        (106, Some(0)),
        (241, Some(0)),
        (259, Some(-1)),
    ];
    for (bar, expected) in cases {
        assert_eq!(
            timeframe
                .states_at(bar)
                .expect("a bar inside the series")
                .rvol_regime,
            expected,
            "unexpected rvol_regime at bar {bar}"
        );
    }
}

#[test]
fn later_bars_never_change_an_earlier_code() {
    let candles = load_fixture_candles();
    let full = states(&model(&candles));
    for length in 20..=candles.len() {
        let prefix = states(&model(&candles[..length]));
        for bar in 0..length {
            assert_eq!(
                prefix[bar], full[bar],
                "bar {bar} changed when the series was truncated to {length}"
            );
        }
    }
}

#[test]
fn the_state_is_deterministic_across_repeated_runs() {
    let candles = load_fixture_candles();
    assert_eq!(states(&model(&candles)), states(&model(&candles)));
}

#[test]
fn an_empty_series_has_no_model() {
    assert!(
        TimeframeModel::compute(
            &TimeframeInput {
                timeframe_secs: 3_600,
                candles: &[],
            },
            &MarketStateConfig::default(),
        )
        .is_none()
    );
}

#[test]
fn a_single_candle_has_no_rvol_regime() {
    let timeframe = model(&candles_with_volumes(&[Some(1.0)]));
    assert_eq!(
        timeframe.states_at(0).expect("the only bar").rvol_regime,
        None
    );
}

#[test]
fn a_short_series_has_no_rvol_regime() {
    let volumes = vec![Some(1.0); 19];
    let timeframe = model(&candles_with_volumes(&volumes));
    assert!(states(&timeframe).iter().all(Option::is_none));
}

#[test]
fn the_twentieth_bar_of_a_constant_volume_series_is_low() {
    let volumes = vec![Some(1.0); 20];
    let timeframe = model(&candles_with_volumes(&volumes));
    assert_eq!(
        timeframe
            .states_at(19)
            .expect("the twentieth bar")
            .rvol_regime,
        Some(-1)
    );
}

#[test]
fn a_modest_volume_expansion_is_normal() {
    let mut volumes = vec![Some(1.0); 20];
    volumes[19] = Some(1.5);
    let timeframe = model(&candles_with_volumes(&volumes));
    assert_eq!(
        timeframe
            .states_at(19)
            .expect("the twentieth bar")
            .rvol_regime,
        Some(0)
    );
}

#[test]
fn a_zero_volume_bar_against_a_positive_baseline_is_low() {
    let mut volumes = vec![Some(1.0); 20];
    volumes[19] = Some(0.0);
    let timeframe = model(&candles_with_volumes(&volumes));
    assert_eq!(
        timeframe
            .states_at(19)
            .expect("the twentieth bar")
            .rvol_regime,
        Some(-1)
    );
}

#[test]
fn an_all_zero_volume_series_has_no_rvol_regime() {
    let volumes = vec![Some(0.0); 20];
    let timeframe = model(&candles_with_volumes(&volumes));
    assert_eq!(
        timeframe
            .states_at(19)
            .expect("the twentieth bar")
            .rvol_regime,
        None
    );
}

#[test]
fn a_missing_volume_has_no_rvol_regime() {
    let mut volumes = vec![Some(1.0); 20];
    volumes[19] = None;
    let timeframe = model(&candles_with_volumes(&volumes));
    assert_eq!(
        timeframe
            .states_at(19)
            .expect("the twentieth bar")
            .rvol_regime,
        None
    );
}

#[test]
fn a_time_gap_does_not_remove_the_state() {
    let mut candles: Vec<Candle> = (0..25)
        .map(|index| candle(index, index as i64 * 3_600, Some(1.0)))
        .collect();
    candles[20] = candle(20, 20 * 3_600 + 7 * 3_600, Some(1.0));
    candles[24] = candle(24, 24 * 3_600 + 7 * 3_600, Some(1.5));
    let timeframe = model(&candles);
    assert_eq!(
        timeframe.states_at(24).expect("the last bar").rvol_regime,
        Some(0)
    );
}
