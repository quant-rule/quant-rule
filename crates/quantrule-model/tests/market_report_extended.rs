//! Ignored 520k-bar extended-range report test.
//!
//! Run explicitly with:
//! `cargo test -p quantrule-model --test market_report_extended --release -- --ignored --nocapture`
//!
//! The test mirrors the shell's full-year load and proves that the report
//! assembles over the whole extended ladder. It reports the observed timings;
//! it does not assert on wall-clock durations.

use std::time::Instant;

use quantrule_data::{Candle, CandleValues, Timeframe, TimeframeSet};
use quantrule_model::aggregate::CrossTimeframeAggregates;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::roles::AxisCode;
use quantrule_model::report::{CHART_BUCKET_LIMIT, MarketReport};
use quantrule_model::run::ModelRun;

const BASE_EPOCH: i64 = 1_735_689_600;
const M1_BARS: usize = 520_000;
const LADDER: [u64; 4] = [60, 300, 900, 3_600];

fn build_report(run: &ModelRun, ladder: &[u64]) -> MarketReport {
    let aggregates = CrossTimeframeAggregates::compute(&run.snapshots());
    MarketReport::compute(run, ladder, &aggregates)
}

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

fn candles(set: &TimeframeSet, timeframe: Timeframe) -> Vec<Candle> {
    set.series(timeframe).iter().copied().collect()
}

#[test]
#[ignore = "520k-bar extended-range report; run explicitly with --release"]
fn a_520k_m1_report_assembles_over_the_extended_ladder() {
    let mut set = TimeframeSet::extended();
    for index in 0..M1_BARS {
        set.push_values(values(index))
            .expect("the benchmark candle is valid");
    }
    set.finish().expect("the benchmark buckets are valid");

    let m1 = candles(&set, Timeframe::M1);
    let m5 = candles(&set, Timeframe::M5);
    let m15 = candles(&set, Timeframe::M15);
    let h1 = candles(&set, Timeframe::H1);
    let inputs = [
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
    ];

    let model_started = Instant::now();
    let run = ModelRun::compute(&inputs, &MarketStateConfig::default());
    let model_elapsed = model_started.elapsed();

    let report_started = Instant::now();
    let report = build_report(&run, &LADDER);
    let report_elapsed = report_started.elapsed();

    assert_eq!(report.ladder, LADDER);
    assert_eq!(report.timeframes.len(), LADDER.len());
    let bars: usize = report
        .timeframes
        .iter()
        .map(|timeframe| timeframe.bars)
        .sum();
    assert_eq!(bars, 667_332);
    assert_eq!(
        report
            .timeframes
            .iter()
            .map(|timeframe| timeframe.timeframe_secs)
            .collect::<Vec<_>>(),
        LADDER
    );
    for timeframe in &report.timeframes {
        assert_eq!(timeframe.axes.len(), AxisCode::ALL.len());
        for axis in &timeframe.axes {
            assert_eq!(axis.present_count + axis.null_count, timeframe.bars);
            assert_eq!(
                axis.code_counts.iter().sum::<u32>() as usize + axis.code_null_count,
                timeframe.bars
            );
        }
        assert!(timeframe.latest.is_some());
    }

    let event_total: usize = run
        .timeframes()
        .iter()
        .map(|timeframe| timeframe.events().len())
        .sum();
    assert_eq!(report.events.total as usize, event_total);
    assert_eq!(report.events.by_type.total(), report.events.total);
    assert_eq!(
        report
            .events
            .timeframes
            .iter()
            .map(|timeframe| timeframe.total as usize)
            .sum::<usize>(),
        event_total
    );

    assert_eq!(report.charts.timeframe_secs, Some(3_600));
    assert_eq!(report.charts.source_bars, 8_666);
    assert_eq!(report.charts.bucket_size, 37);
    assert_eq!(report.charts.bars.len(), 235);
    assert_eq!(report.charts.bars[0].first_bar_index, 0);
    assert_eq!(
        report.charts.bars.last().map(|bar| bar.last_bar_index),
        Some(8_665)
    );
    assert!(report.charts.bars.len() <= CHART_BUCKET_LIMIT);

    let second = build_report(&run, &LADDER);
    assert_eq!(report, second);

    eprintln!(
        "extended report: model {bars} bars in {:.3} s; report in {:.3} s; {} events; {} chart bars; {} chart events",
        model_elapsed.as_secs_f64(),
        report_elapsed.as_secs_f64(),
        report.events.total,
        report.charts.bars.len(),
        report.charts.events.len()
    );
}
