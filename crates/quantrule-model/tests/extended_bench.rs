//! Ignored 520k-bar extended-range benchmark.
//!
//! Run explicitly with:
//! `cargo test -p quantrule-model --test extended_bench --release -- --ignored --nocapture`
//!
//! The test mirrors the shell's full-year load: one M1 year is pushed through
//! the extended timeframe set, the four series are copied into the model input
//! vectors, and the model runs over the whole ladder. It reports the observed
//! timings; it does not assert on wall-clock durations.

use std::time::Instant;

use quantrule_data::{Candle, CandleValues, Timeframe, TimeframeSet};
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

const BASE_EPOCH: i64 = 1_735_689_600;
const M1_BARS: usize = 520_000;

fn values(index: usize) -> CandleValues {
    let phase = (index % 1_000) as f64;
    let open = 100.0 + phase * 0.01;
    let close = open + (((index * 53) % 21) as f64 - 10.0) * 0.01;
    let high = open.max(close) + 0.05;
    let low = open.min(close) - 0.05;
    CandleValues {
        timestamp: BASE_EPOCH + index as i64 * 60,
        open,
        high,
        low,
        close,
        volume: Some(1.0),
    }
}

#[test]
#[ignore = "520k-bar extended-range benchmark; run explicitly with --release"]
fn a_520k_m1_extended_range_benchmark() {
    let load_started = Instant::now();
    let mut set = TimeframeSet::extended();
    for index in 0..M1_BARS {
        set.push_values(values(index))
            .expect("the benchmark candle is valid");
    }
    set.finish().expect("the benchmark buckets are valid");
    let load_elapsed = load_started.elapsed();

    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();

    assert_eq!(m1.len(), M1_BARS);
    assert_eq!(m5.len(), 104_000);
    assert_eq!(m15.len(), 34_666);
    assert_eq!(h1.len(), 8_666);

    let model_started = Instant::now();
    let run = ModelRun::compute(
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
                timeframe_secs: 3_600,
                candles: &h1,
            },
        ],
        &MarketStateConfig::default(),
    );
    let model_elapsed = model_started.elapsed();

    let bars: usize = run
        .timeframes()
        .iter()
        .map(|timeframe| timeframe.len())
        .sum();
    eprintln!(
        "extended benchmark: load {M1_BARS} M1 candles in {:.3} s; model {bars} bars in {:.3} s; total {:.3} s",
        load_elapsed.as_secs_f64(),
        model_elapsed.as_secs_f64(),
        (load_elapsed + model_elapsed).as_secs_f64()
    );
    assert_eq!(run.timeframes().len(), 4);
    assert_eq!(run.timeframe(60).expect("M1").len(), M1_BARS);
}
