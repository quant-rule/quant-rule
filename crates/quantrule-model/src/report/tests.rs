//! Unit, causality, determinism, and edge-case tests for the market report.

use quantrule_data::Candle;

use super::MarketReport;
use super::order::event_cmp;
use crate::readings::roles::AxisCode;
use crate::run::ModelRun;
use crate::state::{MarketStateConfig, TimeframeInput};

const READINGS_FIXTURE: &str = include_str!("../../tests/fixtures/readings_golden.csv");
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

fn fixture_candles() -> Vec<Candle> {
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
                .expect("valid fixture candle"),
        );
    }
    candles
}

fn build_run(candles: &[Candle], timeframes: &[u64]) -> ModelRun {
    let inputs: Vec<TimeframeInput<'_>> = timeframes
        .iter()
        .map(|&timeframe_secs| TimeframeInput {
            timeframe_secs,
            candles,
        })
        .collect();
    ModelRun::compute(&inputs, &MarketStateConfig::default())
}

fn run_inputs(inputs: &[(u64, &[Candle])]) -> ModelRun {
    let inputs: Vec<TimeframeInput<'_>> = inputs
        .iter()
        .map(|(timeframe_secs, candles)| TimeframeInput {
            timeframe_secs: *timeframe_secs,
            candles,
        })
        .collect();
    ModelRun::compute(&inputs, &MarketStateConfig::default())
}

fn build_report(run: &ModelRun, ladder: &[u64]) -> MarketReport {
    MarketReport::compute(run, ladder)
}

fn candle(index: usize, timestamp: i64, volume: Option<f64>) -> Candle {
    Candle::new(index as u64, timestamp, 100.0, 101.0, 99.0, 100.5, volume).expect("valid candle")
}

#[test]
fn coverage_period_and_absent_timeframes_are_reported() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[300, 60, 300]);

    assert_eq!(report.ladder, vec![60, 300]);
    assert_eq!(report.timeframes.len(), 2);

    let absent = &report.timeframes[0];
    assert_eq!(absent.timeframe_secs, 60);
    assert_eq!(absent.bars, 0);
    assert_eq!(absent.expected_bars, 0);
    assert_eq!(absent.missing_bars, 0);
    assert_eq!(absent.coverage_share, None);
    assert_eq!(absent.first_open_time, None);
    assert_eq!(absent.last_open_time, None);
    assert!(absent.latest.is_none());
    assert_eq!(absent.axes.len(), 4);

    let present = &report.timeframes[1];
    assert_eq!(present.timeframe_secs, 300);
    assert_eq!(present.bars, 260);
    assert_eq!(present.expected_bars, 260);
    assert_eq!(present.missing_bars, 0);
    assert_eq!(present.coverage_share, Some(1.0));
    assert_eq!(present.first_open_time, Some(1_700_000_000));
    assert_eq!(present.last_open_time, Some(1_700_000_000 + 259 * 300));

    assert_eq!(report.period.first_open_time, Some(1_700_000_000));
    assert_eq!(
        report.period.last_open_time,
        Some(1_700_000_000 + 259 * 300)
    );
    assert_eq!(report.cross_timeframe.ladder, vec![300]);
}

#[test]
fn axis_reports_have_the_fixed_shape_and_consistent_counts() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[60, 300]);
    let report = build_report(&run, &[60, 300]);

    for timeframe in &report.timeframes {
        assert_eq!(timeframe.axes.len(), 4);
        for (index, axis) in timeframe.axes.iter().enumerate() {
            assert_eq!(axis.axis, AxisCode::ALL[index]);
            let histogram_len = match axis.axis {
                AxisCode::Direction | AxisCode::Momentum => 3,
                AxisCode::Structure => 5,
                AxisCode::Progression => 6,
            };
            assert_eq!(axis.code_counts.len(), histogram_len);
            assert_eq!(axis.present_count + axis.null_count, timeframe.bars);
            assert_eq!(
                axis.code_counts.iter().sum::<u32>() as usize + axis.code_null_count,
                timeframe.bars
            );
            if axis.present_count == 0 {
                assert_eq!(axis.min, None);
                assert_eq!(axis.max, None);
                assert_eq!(axis.mean, None);
            } else {
                let min = axis.min.expect("a present axis has a minimum");
                let max = axis.max.expect("a present axis has a maximum");
                let mean = axis.mean.expect("a present axis has a mean");
                assert!(min <= max);
                assert!(mean >= min && mean <= max);
            }
        }
    }
}

#[test]
fn the_latest_state_maps_the_last_model_bar() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[300]);
    let timeframe = run.timeframe(300).expect("the timeframe is present");
    let bar = timeframe
        .bar(timeframe.len() - 1)
        .expect("the latest bar exists");
    let state = report.timeframes[0]
        .latest
        .as_ref()
        .expect("the latest state exists");

    assert_eq!(state.bar_index, bar.bar_index);
    assert_eq!(state.open_time, bar.candle.timestamp());
    assert_eq!(state.readings.ema20, bar.readings.ema20);
    assert_eq!(state.readings.ema50, bar.readings.ema50);
    assert_eq!(state.readings.ema200, bar.readings.ema200);
    assert_eq!(state.readings.rsi14, bar.readings.rsi14);
    assert_eq!(state.readings.atr14, bar.readings.atr14);
    assert_eq!(state.readings.rvol20, bar.readings.rvol20);

    assert_eq!(state.direction.direction, bar.direction.direction);
    assert_eq!(state.direction.state_code, bar.direction.state_code);
    assert_eq!(
        state.direction.readiness_code,
        bar.direction.readiness.code()
    );
    assert_eq!(
        state.direction.reason_code,
        bar.direction.unavailable_reason.map(|reason| reason.code())
    );
    assert_eq!(state.momentum.force, bar.momentum.force);
    assert_eq!(state.momentum.readiness_code, bar.momentum.readiness.code());
    assert_eq!(state.structure.score, bar.structure.score);
    assert_eq!(state.structure.scale_code, bar.structure.scale_code);
    assert_eq!(
        state.structure.readiness_code,
        bar.structure.readiness.code()
    );
    assert_eq!(state.progression.progress, bar.progression.progress);
    assert_eq!(state.progression.stage_code, bar.progression.stage_code);
    assert_eq!(
        state.progression.readiness_code,
        bar.progression.readiness.code()
    );
    assert_eq!(
        state.progression.leg.is_some(),
        bar.progression.published_leg().is_some()
    );
    assert_eq!(state.context.context_code, bar.context.context_code);
    assert_eq!(
        state.consensus.bullish_axis_count,
        bar.consensus.bullish_axis_count
    );
    assert_eq!(
        state.consensus.bearish_axis_count,
        bar.consensus.bearish_axis_count
    );
    assert_eq!(
        state.consensus.neutral_axis_count,
        bar.consensus.neutral_axis_count
    );
    assert_eq!(
        state.consensus.weakest_directional_axis_code,
        bar.consensus.weakest_directional_axis_code
    );
}

#[test]
fn every_event_is_counted_once_and_the_totals_agree() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[60, 300]);
    let report = build_report(&run, &[60, 300]);

    let expected_total: usize = run
        .timeframes()
        .iter()
        .map(|timeframe| timeframe.events().len())
        .sum();
    assert_eq!(report.events.total as usize, expected_total);
    assert_eq!(report.events.by_type.total(), report.events.total);

    let subtype_total: u32 = report
        .events
        .by_subtype
        .level
        .iter()
        .chain(report.events.by_subtype.divergence.iter())
        .chain(report.events.by_subtype.signal.iter())
        .sum();
    assert_eq!(subtype_total, report.events.total);

    for timeframe_report in &report.events.timeframes {
        let timeframe = run
            .timeframe(timeframe_report.timeframe_secs)
            .expect("the ladder member is present");
        assert_eq!(timeframe_report.total as usize, timeframe.events().len());
        assert_eq!(timeframe_report.by_type.total(), timeframe_report.total);
        let subtype_total: u32 = timeframe_report
            .by_subtype
            .level
            .iter()
            .chain(timeframe_report.by_subtype.divergence.iter())
            .chain(timeframe_report.by_subtype.signal.iter())
            .sum();
        assert_eq!(subtype_total, timeframe_report.total);

        let expected_latest = timeframe
            .events()
            .iter()
            .max_by(|left, right| event_cmp(left, right));
        assert_eq!(
            timeframe_report
                .latest
                .map(|event| event.available_bar_index),
            expected_latest.map(|event| event.available_bar_index)
        );
        assert_eq!(
            timeframe_report
                .latest
                .map(|event| event.occurred_bar_index),
            expected_latest.map(|event| event.occurred_bar_index)
        );
        assert_eq!(
            timeframe_report.latest.map(|event| event.event_type_code),
            expected_latest.map(|event| event.event_type.code())
        );
    }
}

#[test]
fn the_chart_prefers_h1_then_the_slowest_present_timeframe() {
    let candles = fixture_candles();
    let with_h1 = build_run(&candles, &[60, 300, 3_600]);
    let report = build_report(&with_h1, &[60, 300, 3_600]);
    assert_eq!(report.charts.timeframe_secs, Some(3_600));
    assert_eq!(report.charts.source_bars, 260);
    assert_eq!(report.charts.bucket_size, 2);
    assert_eq!(report.charts.bars.len(), 130);

    let without_h1 = build_run(&candles, &[60, 300]);
    let report = build_report(&without_h1, &[60, 300, 3_600]);
    assert_eq!(report.charts.timeframe_secs, Some(300));
    assert_eq!(report.charts.source_bars, 260);
    assert_eq!(report.charts.bucket_size, 2);

    let empty = ModelRun::compute(&[], &MarketStateConfig::default());
    let report = build_report(&empty, &[60]);
    assert_eq!(report.charts.timeframe_secs, None);
    assert_eq!(report.charts.source_bars, 0);
    assert_eq!(report.charts.bucket_size, 1);
    assert!(report.charts.bars.is_empty());
    assert!(report.charts.events.is_empty());
}

#[test]
fn chart_buckets_copy_the_first_open_last_close_and_extremes() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[300]);
    let chart = &report.charts;
    let timeframe = run.timeframe(300).expect("the timeframe is present");

    assert_eq!(chart.bucket_size, 2);
    let first = chart.bars[0];
    assert_eq!(first.first_bar_index, 0);
    assert_eq!(first.last_bar_index, 1);
    assert_eq!(first.open_time, candles[0].timestamp());
    assert_eq!(first.last_open_time, candles[1].timestamp());
    assert_eq!(first.open, candles[0].open());
    assert_eq!(first.close, candles[1].close());
    assert_eq!(first.high, candles[0].high().max(candles[1].high()));
    assert_eq!(first.low, candles[0].low().min(candles[1].low()));
    assert_eq!(
        first.volume,
        Some(
            candles[0].volume().expect("fixture volume")
                + candles[1].volume().expect("fixture volume")
        )
    );

    let bar0 = timeframe.bar(0).expect("bar 0");
    let bar1 = timeframe.bar(1).expect("bar 1");
    assert_eq!(
        first.direction,
        bar1.direction.direction.or(bar0.direction.direction)
    );
    assert_eq!(first.momentum, bar1.momentum.force.or(bar0.momentum.force));
    assert_eq!(
        first.structure,
        bar1.structure.score.or(bar0.structure.score)
    );
    assert_eq!(
        first.progression_stage_code,
        bar1.progression.stage_code.or(bar0.progression.stage_code)
    );

    let last = chart
        .bars
        .last()
        .expect("the chart has at least one bucket");
    assert_eq!(last.first_bar_index, 258);
    assert_eq!(last.last_bar_index, 259);
    assert_eq!(last.last_open_time, candles[259].timestamp());
    assert_eq!(last.close, candles[259].close());
}

#[test]
fn a_missing_volume_nulls_only_its_bucket_and_a_zero_volume_stays_zero() {
    let candles: Vec<Candle> = (0..241)
        .map(|index| {
            let volume = if index == 1 { None } else { Some(1.0) };
            candle(index, 1_700_000_000 + index as i64 * 300, volume)
        })
        .collect();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[300]);
    let chart = &report.charts;

    assert_eq!(chart.bucket_size, 2);
    assert_eq!(chart.bars.len(), 121);
    assert_eq!(chart.bars[0].volume, None);
    assert_eq!(chart.bars[1].volume, Some(2.0));
    assert_eq!(chart.bars.last().map(|bar| bar.first_bar_index), Some(240));
    assert_eq!(chart.bars.last().map(|bar| bar.last_bar_index), Some(240));

    let zero_volume: Vec<Candle> = (0..3)
        .map(|index| candle(index, 1_700_000_000 + index as i64 * 300, Some(0.0)))
        .collect();
    let run = build_run(&zero_volume, &[300]);
    let report = build_report(&run, &[300]);
    assert_eq!(report.charts.bars.len(), 3);
    assert!(
        report.charts.bars.iter().all(|bar| bar.volume == Some(0.0)),
        "a zero volume is a valid sum, not a missing value"
    );
}

#[test]
fn chart_events_are_sorted_and_drawn_at_their_availability_bar() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[300]);
    let timeframe = run.timeframe(300).expect("the timeframe is present");
    let mut expected = timeframe.events().to_vec();
    expected.sort_by(event_cmp);

    assert_eq!(report.charts.events.len(), expected.len());
    for (actual, event) in report.charts.events.iter().zip(&expected) {
        assert_eq!(actual.available_bar_index, event.available_bar_index);
        assert_eq!(actual.occurred_bar_index, event.occurred_bar_index);
        assert_eq!(actual.event_type_code, event.event_type.code());
        assert_eq!(actual.event_subtype_code, event.event_subtype_code);
        assert_eq!(actual.direction_code, event.direction_code);
        assert_eq!(
            actual.bucket_index,
            event.available_bar_index / report.charts.bucket_size
        );
    }

    if let (Some(latest), Some(last)) = (
        report.events.timeframes[0].latest,
        report.charts.events.last(),
    ) {
        assert_eq!(latest.available_bar_index, last.available_bar_index);
        assert_eq!(latest.occurred_bar_index, last.occurred_bar_index);
        assert_eq!(latest.event_type_code, last.event_type_code);
    }
}

#[test]
fn ladder_order_and_duplicates_do_not_change_the_report() {
    let candles = fixture_candles();
    let forward = build_run(&candles, &[60, 300]);
    let reversed = build_run(&candles, &[300, 60]);
    assert_eq!(forward, reversed);

    let first = build_report(&forward, &[60, 300]);
    let second = build_report(&reversed, &[300, 60, 300]);
    assert_eq!(first, second);
    assert_eq!(first.ladder, vec![60, 300]);

    let repeated = build_report(&forward, &[60, 300]);
    assert_eq!(first, repeated);
}

#[test]
fn a_prefix_report_reads_only_bars_at_or_before_its_last_bar() {
    let candles = fixture_candles();
    let full_run = build_run(&candles, &[300]);
    let prefix_run = build_run(&candles[..=199], &[300]);
    let prefix_report = build_report(&prefix_run, &[300]);
    let full_timeframe = full_run.timeframe(300).expect("the timeframe is present");
    let full_bar = full_timeframe.bar(199).expect("bar 199");

    let latest = prefix_report.timeframes[0]
        .latest
        .as_ref()
        .expect("the prefix has a latest state");
    assert_eq!(latest.bar_index, 199);
    assert_eq!(latest.open_time, full_bar.candle.timestamp());
    assert_eq!(latest.direction.direction, full_bar.direction.direction);
    assert_eq!(latest.momentum.force, full_bar.momentum.force);
    assert_eq!(latest.structure.score, full_bar.structure.score);
    assert_eq!(latest.progression.progress, full_bar.progression.progress);
    assert_eq!(latest.context.context_code, full_bar.context.context_code);
    assert_eq!(
        latest.consensus.bullish_axis_count,
        full_bar.consensus.bullish_axis_count
    );
    assert_eq!(
        latest.consensus.bearish_axis_count,
        full_bar.consensus.bearish_axis_count
    );
    assert_eq!(
        latest.consensus.neutral_axis_count,
        full_bar.consensus.neutral_axis_count
    );

    let mut altered = candles.clone();
    for original in altered.iter_mut().skip(200) {
        *original = Candle::new(
            original.series_index(),
            original.timestamp(),
            original.open(),
            original.high() + 50.0,
            original.low() - 50.0,
            original.close() + 10.0,
            original.volume(),
        )
        .expect("a valid altered future candle");
    }
    let altered_prefix = build_run(&altered[..=199], &[300]);
    assert_eq!(
        prefix_report,
        build_report(&altered_prefix, &[300]),
        "changing later bars must not change the prefix report"
    );
}

#[test]
fn a_single_candle_report_has_one_bar_and_no_invented_values() {
    let candle = Candle::new(0, 1_700_000_000, 100.0, 100.5, 99.5, 100.0, Some(10.0))
        .expect("a valid candle");
    let run = build_run(&[candle], &[60]);
    let report = build_report(&run, &[60]);
    let timeframe = &report.timeframes[0];

    assert_eq!(timeframe.bars, 1);
    assert_eq!(timeframe.expected_bars, 1);
    assert_eq!(timeframe.missing_bars, 0);
    assert_eq!(timeframe.coverage_share, Some(1.0));
    let latest = timeframe.latest.as_ref().expect("the latest state exists");
    assert_eq!(latest.bar_index, 0);
    assert_eq!(latest.direction.readiness_code, 0);
    assert_eq!(latest.direction.reason_code, Some(1));
    assert_eq!(latest.structure.readiness_code, 0);
    assert_eq!(latest.structure.reason_code, Some(2));

    assert_eq!(report.charts.source_bars, 1);
    assert_eq!(report.charts.bucket_size, 1);
    assert_eq!(report.charts.bars.len(), 1);
    assert_eq!(report.charts.bars[0].volume, Some(10.0));
    assert_eq!(report.events.total, 0);
    for axis in &timeframe.axes {
        assert_eq!(axis.present_count + axis.null_count, 1);
    }
}

#[test]
fn a_short_history_publishes_explicit_nulls() {
    let candles: Vec<Candle> = (0..10)
        .map(|index| candle(index, 1_700_000_000 + index as i64 * 300, Some(1.0)))
        .collect();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[300]);
    let timeframe = &report.timeframes[0];

    assert_eq!(timeframe.bars, 10);
    assert_eq!(timeframe.coverage_share, Some(1.0));
    for axis in &timeframe.axes {
        assert_eq!(axis.present_count, 0);
        assert_eq!(axis.null_count, 10);
        assert_eq!(axis.code_null_count, 10);
        assert!(axis.code_counts.iter().all(|count| *count == 0));
        assert_eq!(axis.min, None);
        assert_eq!(axis.max, None);
        assert_eq!(axis.mean, None);
    }
}

#[test]
fn a_gap_is_counted_without_fabricating_a_bar() {
    let candles = [
        candle(0, 0, Some(1.0)),
        candle(1, 60, Some(1.0)),
        candle(2, 180, Some(1.0)),
        candle(3, 240, Some(1.0)),
    ];
    let run = build_run(&candles, &[60]);
    let report = build_report(&run, &[60]);
    let timeframe = &report.timeframes[0];

    assert_eq!(timeframe.bars, 4);
    assert_eq!(timeframe.expected_bars, 5);
    assert_eq!(timeframe.missing_bars, 1);
    assert_eq!(timeframe.coverage_share, Some(0.8));
    for axis in &timeframe.axes {
        assert_eq!(axis.present_count + axis.null_count, 4);
        assert_eq!(
            axis.code_counts.iter().sum::<u32>() as usize + axis.code_null_count,
            4
        );
    }
    assert_eq!(report.charts.bars.len(), 4);
    assert_eq!(report.charts.bars[3].last_open_time, 240);
}

#[test]
fn unaligned_timeframe_bounds_are_reported_independently() {
    let fast = [
        candle(0, 0, Some(1.0)),
        candle(1, 60, Some(1.0)),
        candle(2, 120, Some(1.0)),
    ];
    let slow = [candle(0, 300, Some(1.0)), candle(1, 600, Some(1.0))];
    let run = run_inputs(&[(60, &fast), (300, &slow)]);
    let report = build_report(&run, &[60, 300]);

    assert_eq!(report.period.first_open_time, Some(0));
    assert_eq!(report.period.last_open_time, Some(600));
    assert_eq!(report.timeframes[0].first_open_time, Some(0));
    assert_eq!(report.timeframes[0].last_open_time, Some(120));
    assert_eq!(report.timeframes[1].first_open_time, Some(300));
    assert_eq!(report.timeframes[1].last_open_time, Some(600));
    assert_eq!(report.timeframes[0].coverage_share, Some(1.0));
    assert_eq!(report.timeframes[1].coverage_share, Some(1.0));
    assert_eq!(report.cross_timeframe.ladder, vec![60, 300]);
}

#[test]
fn an_empty_ladder_is_an_empty_report() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[]);

    assert!(report.ladder.is_empty());
    assert!(report.timeframes.is_empty());
    assert_eq!(report.period.first_open_time, None);
    assert_eq!(report.period.last_open_time, None);
    assert_eq!(report.events.total, 0);
    assert!(report.events.timeframes.is_empty());
    assert_eq!(report.events.by_type.total(), 0);
    assert!(report.cross_timeframe.ladder.is_empty());
    assert_eq!(report.charts.timeframe_secs, None);
    assert!(report.charts.bars.is_empty());
    assert!(report.charts.events.is_empty());
}

#[test]
fn the_view_contains_no_trade_or_pnl_field() {
    let candles = fixture_candles();
    let run = build_run(&candles, &[300]);
    let report = build_report(&run, &[300]);
    let debug = format!("{report:?}").to_ascii_lowercase();

    for forbidden in [
        "order", "fill", "position", "trade", "equity", "pnl", "drawdown", "strategy",
    ] {
        assert!(
            !debug.contains(forbidden),
            "the market report must not publish a {forbidden:?} field"
        );
    }
}
