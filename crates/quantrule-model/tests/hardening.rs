//! Cross-cutting hardening tests for the model layer.
//!
//! These tests cover invariants no single sub-issue owns: output lengths, value
//! ranges, event availability, anti-double-counting, determinism across
//! reordered timeframes, and a long-history smoke benchmark.

use std::time::Instant;

use quantrule_data::{Candle, CandleValues, Timeframe, TimeframeSet};
use quantrule_model::context::ContextSeries;
use quantrule_model::events::divergence::{DivergenceConfig, DivergenceEventSeries};
use quantrule_model::events::level::{LevelEventConfig, LevelEventSeries};
use quantrule_model::events::signal::{SignalConfig, SignalEventSeries};
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::progression::{ProgressionConfig, ProgressionSeries};
use quantrule_model::readings::{PivotScale, Readings};
use quantrule_model::run::ModelRun;

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

#[test]
fn every_prefix_keeps_the_output_lengths_and_ranges() {
    let candles = load_candles();
    for length in 0..=candles.len() {
        let prefix = &candles[..length];
        let readings = Readings::compute(prefix);
        let progression =
            ProgressionSeries::compute_with_config(prefix, &readings, ProgressionConfig::new(60));
        let context = ContextSeries::compute(&readings);

        assert_eq!(readings.len(), length);
        assert_eq!(progression.len(), length);
        assert_eq!(context.len(), length);

        for bar in 0..length {
            let progression_bar = progression.get(bar).expect("progression bar");
            let context_bar = context.get(bar).expect("context bar");

            assert_finite(progression_bar.progress, "progress");
            assert_finite(context_bar.atr_ratio, "ATR ratio");
            assert_finite(context_bar.rvol_ratio, "RVOL ratio");

            if let Some(percentile) = progression_bar.size_percentile {
                assert!(
                    (0.0..=100.0).contains(&percentile),
                    "percentile {percentile} is outside 0..=100"
                );
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
        &quantrule_model::readings::MacdSeries::empty(readings.len()),
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
fn event_detection_changes_no_reading_or_progression_value() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
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
        &quantrule_model::readings::MacdSeries::empty(readings.len()),
        &SignalConfig::from_secs(60).expect("timeframe"),
    );

    assert_eq!(
        progression_before,
        ProgressionSeries::compute_with_config(&candles, &readings, ProgressionConfig::new(60))
    );
}

#[test]
fn the_model_run_is_independent_of_timeframe_input_order() {
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

    let forward = ModelRun::compute(
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
    let reversed = ModelRun::compute(
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
fn the_model_run_is_deterministic_across_repeated_runs() {
    let candles = load_candles();
    let input = TimeframeInput {
        timeframe_secs: 60,
        candles: &candles,
    };
    let first = ModelRun::compute(&[input], &MarketStateConfig::default());
    let second = ModelRun::compute(&[input], &MarketStateConfig::default());
    assert_eq!(first, second);
}

fn long_history(len: usize) -> ModelRun {
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
    ModelRun::compute(
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
    let run = long_history(len);
    assert_eq!(run.timeframes().len(), 1);
    assert_eq!(run.timeframes()[0].len(), len);
}

#[test]
#[ignore = "100k-bar benchmark; run explicitly with --release"]
fn a_100k_history_benchmark() {
    let len = 100_000;
    let started = Instant::now();
    let run = long_history(len);
    let elapsed = started.elapsed();
    eprintln!(
        "long-history benchmark: {len} bars in {:.3} s",
        elapsed.as_secs_f64()
    );
    assert_eq!(run.timeframes().len(), 1);
    assert_eq!(run.timeframes()[0].len(), len);
}
