//! Cross-cutting hardening tests for the model layer.
//!
//! These tests cover invariants no single sub-issue owns: output lengths, value
//! ranges, event availability, anti-double-counting, determinism across
//! reordered timeframes, and a long-history smoke benchmark.

use std::time::Instant;

use quantrule_data::{Candle, CandleValues, Timeframe, TimeframeSet};
use quantrule_model::axes::direction::DirectionSeries;
use quantrule_model::axes::momentum::MomentumSeries;
use quantrule_model::axes::progression::{ProgressionConfig, ProgressionSeries};
use quantrule_model::axes::structure::StructureSeries;
use quantrule_model::context::ContextSeries;
use quantrule_model::events::divergence::{DivergenceConfig, DivergenceEventSeries};
use quantrule_model::events::level::{LevelEventConfig, LevelEventSeries};
use quantrule_model::events::signal::{SignalConfig, SignalEventSeries};
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::{PivotScale, ReadingSeries, Readings};
use quantrule_model::state::MarketState;

const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(
            field
                .parse::<f64>()
                .unwrap_or_else(|error| panic!("invalid number {field:?}: {error}")),
        )
    }
}

fn load_candles() -> Vec<Candle> {
    let mut lines = READINGS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(READINGS_HEADER));
    let mut candles = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23);
        let series_index = fields[0].parse().expect("bar");
        let timestamp = fields[1].parse().expect("timestamp");
        let open = number(fields[2]).expect("open");
        let high = number(fields[3]).expect("high");
        let low = number(fields[4]).expect("low");
        let close = number(fields[5]).expect("close");
        let volume = number(fields[6]);
        candles.push(
            Candle::new(series_index, timestamp, open, high, low, close, volume)
                .expect("valid candle"),
        );
    }
    candles
}

fn assert_finite(value: Option<f64>, context: &str) {
    if let Some(value) = value {
        assert!(value.is_finite(), "{context}: {value} is not finite");
    }
}

fn assert_unit(value: Option<f64>, context: &str) {
    if let Some(value) = value {
        assert!(
            (-1.0..=1.0).contains(&value),
            "{context}: {value} is outside [-1, 1]"
        );
    }
}

#[test]
fn every_prefix_keeps_the_output_lengths_and_ranges() {
    let candles = load_candles();
    for length in 0..=candles.len() {
        let prefix = &candles[..length];
        let readings = Readings::compute(prefix);
        let direction = DirectionSeries::compute(&readings);
        let momentum = MomentumSeries::compute(&readings);
        let structure = StructureSeries::compute(prefix, &readings);
        let progression =
            ProgressionSeries::compute_with_config(prefix, &readings, ProgressionConfig::new(60));
        let context = ContextSeries::compute(&readings);

        assert_eq!(readings.len(), length);
        assert_eq!(direction.len(), length);
        assert_eq!(momentum.len(), length);
        assert_eq!(structure.len(), length);
        assert_eq!(progression.len(), length);
        assert_eq!(context.len(), length);

        for bar in 0..length {
            let direction_bar = direction.get(bar).expect("direction bar");
            let momentum_bar = momentum.get(bar).expect("momentum bar");
            let structure_bar = structure.get(bar).expect("structure bar");
            let progression_bar = progression.get(bar).expect("progression bar");
            let context_bar = context.get(bar).expect("context bar");

            assert_unit(direction_bar.direction, "direction");
            assert_unit(momentum_bar.force, "momentum force");
            assert_unit(momentum_bar.acceleration, "momentum acceleration");
            assert_unit(momentum_bar.intensification, "momentum intensification");
            assert_unit(structure_bar.score, "structure score");
            assert_finite(direction_bar.direction, "direction");
            assert_finite(momentum_bar.force, "momentum force");
            assert_finite(structure_bar.score, "structure score");
            assert_finite(progression_bar.progress, "progress");
            assert_finite(context_bar.atr_ratio, "ATR ratio");
            assert_finite(context_bar.rvol_ratio, "RVOL ratio");

            for percentile in [
                direction_bar.size_percentile,
                momentum_bar.size_percentile,
                structure_bar.size_percentile,
                progression_bar.size_percentile,
            ]
            .into_iter()
            .flatten()
            {
                assert!(
                    (0.0..=100.0).contains(&percentile),
                    "percentile {percentile} is outside 0..=100"
                );
            }

            if let Some(code) = direction_bar.state_code {
                assert!((-1..=1).contains(&code), "direction state {code}");
            }
            if let Some(code) = momentum_bar.state_code {
                assert!((-1..=1).contains(&code), "momentum state {code}");
            }
            if let Some(code) = structure_bar.state_code {
                assert!((-2..=2).contains(&code), "structure code {code}");
            }
            if let Some(code) = progression_bar.stage_code {
                assert!((0..=5).contains(&code), "progression stage {code}");
            }
            if let Some(code) = context_bar.context_code {
                assert!((1..=4).contains(&code), "context code {code}");
            }
        }
    }
}

#[test]
fn every_event_is_available_no_earlier_than_it_occurs() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let level = LevelEventSeries::compute(
        &candles,
        &readings,
        &LevelEventConfig::from_secs(60).expect("timeframe"),
    );
    let divergence = DivergenceEventSeries::compute(
        &readings,
        &DivergenceConfig::from_secs(60, PivotScale::Major).expect("timeframe"),
    );
    let signal = SignalEventSeries::compute(
        &readings,
        &DirectionSeries::compute(&readings),
        &MomentumSeries::compute(&readings),
        &StructureSeries::compute(&candles, &readings),
        &SignalConfig::from_secs(60).expect("timeframe"),
    );

    for event in level
        .events()
        .iter()
        .chain(divergence.events())
        .chain(signal.events())
    {
        assert!(
            event.available_bar_index >= event.occurred_bar_index,
            "event available before occurrence: {event:?}"
        );
        assert!(event.occurred_bar_index < candles.len());
        assert!(event.available_bar_index < candles.len());
    }
    for event in level.events() {
        assert_eq!(event.occurred_bar_index, event.available_bar_index);
    }
    for event in signal.events() {
        assert_eq!(event.occurred_bar_index, event.available_bar_index);
    }
}

#[test]
fn the_ema_features_never_enter_the_structure_score() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let baseline = StructureSeries::compute(&candles, &readings);
    let mut altered = readings.clone();
    let len = candles.len();
    altered.ema20 = ReadingSeries::new(vec![Some(1.0); len]);
    altered.ema50 = ReadingSeries::new(vec![Some(2.0); len]);
    altered.ema200 = ReadingSeries::new(vec![Some(3.0); len]);
    altered.atr14 = ReadingSeries::new(vec![Some(0.5); len]);
    let changed = StructureSeries::compute(&candles, &altered);

    for bar in 0..len {
        let before = baseline.get(bar).expect("baseline");
        let after = changed.get(bar).expect("changed");
        assert_eq!(before.score, after.score, "score changed at bar {bar}");
        assert_eq!(before.state_code, after.state_code);
        assert_eq!(before.high_relation_code, after.high_relation_code);
        assert_eq!(before.low_relation_code, after.low_relation_code);
        assert_ne!(
            before.ema_distance_atr_20, after.ema_distance_atr_20,
            "the EMA feature did not change at bar {bar}"
        );
    }
}

#[test]
fn event_detection_changes_no_axis_value() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let direction_before = DirectionSeries::compute(&readings);
    let momentum_before = MomentumSeries::compute(&readings);
    let structure_before = StructureSeries::compute(&candles, &readings);
    let progression_before =
        ProgressionSeries::compute_with_config(&candles, &readings, ProgressionConfig::new(60));

    let _ = LevelEventSeries::compute(
        &candles,
        &readings,
        &LevelEventConfig::from_secs(60).expect("timeframe"),
    );
    let _ = DivergenceEventSeries::compute(
        &readings,
        &DivergenceConfig::from_secs(60, PivotScale::Major).expect("timeframe"),
    );
    let _ = SignalEventSeries::compute(
        &readings,
        &direction_before,
        &momentum_before,
        &structure_before,
        &SignalConfig::from_secs(60).expect("timeframe"),
    );

    assert_eq!(direction_before, DirectionSeries::compute(&readings));
    assert_eq!(momentum_before, MomentumSeries::compute(&readings));
    assert_eq!(
        structure_before,
        StructureSeries::compute(&candles, &readings)
    );
    assert_eq!(
        progression_before,
        ProgressionSeries::compute_with_config(&candles, &readings, ProgressionConfig::new(60))
    );
}

#[test]
fn the_consensus_counts_exactly_the_present_axes() {
    let candles = load_candles();
    let state = MarketState::compute(
        &[TimeframeInput {
            timeframe_secs: 60,
            candles: &candles,
        }],
        &MarketStateConfig::default(),
    );
    let timeframe = &state.timeframes[0];
    let present = [
        timeframe.snapshot.direction.direction,
        timeframe.snapshot.momentum.force,
        timeframe.snapshot.structure.score,
    ]
    .into_iter()
    .filter(Option::is_some)
    .count();
    let consensus = state.consensus[0];
    assert_eq!(
        usize::from(consensus.bullish_axis_count)
            + usize::from(consensus.bearish_axis_count)
            + usize::from(consensus.neutral_axis_count),
        present
    );
    assert!(present <= 3);
}

#[test]
fn the_market_state_is_independent_of_timeframe_input_order() {
    let candles = load_candles();
    let mut set = TimeframeSet::new();
    let base = 1_700_000_000_i64.div_euclid(60) * 60;
    for (index, candle) in candles.iter().enumerate() {
        set.push_values(CandleValues {
            timestamp: base + index as i64 * 60,
            open: candle.open(),
            high: candle.high(),
            low: candle.low(),
            close: candle.close(),
            volume: candle.volume(),
        })
        .expect("fixture candle");
    }
    set.finish().expect("set finishes");
    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();
    let config = MarketStateConfig::default();

    let forward = MarketState::compute(
        &[
            TimeframeInput {
                timeframe_secs: 60,
                candles: &m1,
            },
            TimeframeInput {
                timeframe_secs: 300,
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: 900,
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: 3600,
                candles: &h1,
            },
        ],
        &config,
    );
    let reversed = MarketState::compute(
        &[
            TimeframeInput {
                timeframe_secs: 3600,
                candles: &h1,
            },
            TimeframeInput {
                timeframe_secs: 900,
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: 300,
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: 60,
                candles: &m1,
            },
        ],
        &config,
    );
    assert_eq!(forward, reversed);
}

#[test]
fn the_market_state_is_deterministic_across_repeated_runs() {
    let candles = load_candles();
    let input = TimeframeInput {
        timeframe_secs: 60,
        candles: &candles,
    };
    let first = MarketState::compute(&[input], &MarketStateConfig::default());
    let second = MarketState::compute(&[input], &MarketStateConfig::default());
    assert_eq!(first, second);
}

fn long_history(len: usize) -> MarketState {
    let mut candles = Vec::with_capacity(len);
    for index in 0..len {
        let price = 100.0 + (index % 200) as f64 * 0.01 + (index % 17) as f64 * 0.003;
        candles.push(
            Candle::new(
                index as u64,
                index as i64 * 60,
                price,
                price + 0.001,
                price - 0.001,
                price,
                Some(1.0),
            )
            .expect("benchmark candle"),
        );
    }
    MarketState::compute(
        &[TimeframeInput {
            timeframe_secs: 60,
            candles: &candles,
        }],
        &MarketStateConfig::default(),
    )
}

#[test]
fn a_long_history_run_completes() {
    let len = 20_000;
    let state = long_history(len);
    assert_eq!(state.timeframes.len(), 1);
    assert_eq!(state.timeframes[0].snapshot.latest_bar_index, len - 1);
}

#[test]
#[ignore = "100k-bar benchmark; run explicitly with --release"]
fn a_100k_history_benchmark() {
    let len = 100_000;
    let started = Instant::now();
    let state = long_history(len);
    let elapsed = started.elapsed();
    eprintln!(
        "long-history benchmark: {len} bars in {:.3} s",
        elapsed.as_secs_f64()
    );
    assert_eq!(state.timeframes.len(), 1);
    assert_eq!(state.timeframes[0].snapshot.latest_bar_index, len - 1);
}
