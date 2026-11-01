//! Tests for the as-of ladder context.

use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

use super::{
    AgreementCode, AxisAccumulator, ContextTimeframe, LadderConfig, LadderContextBuilder,
    LadderContextError, LadderWeights, build, compute_axes,
};

fn candle(timestamp: i64, close: f64) -> Candle {
    Candle::new(
        timestamp as u64,
        timestamp,
        close,
        close + 1.0,
        close - 1.0,
        close,
        Some(10.0),
    )
    .expect("the test candle is valid")
}

fn make_run(
    execution_secs: u64,
    execution_bars: usize,
    context_secs: u64,
    context_bars: usize,
) -> ModelRun {
    let execution: Vec<Candle> = (0..execution_bars)
        .map(|index| {
            candle(
                index as i64 * execution_secs as i64,
                100.0 + index as f64 * 0.1,
            )
        })
        .collect();
    let context: Vec<Candle> = (0..context_bars)
        .map(|index| {
            candle(
                index as i64 * context_secs as i64,
                100.0 + index as f64 * 0.2,
            )
        })
        .collect();
    ModelRun::compute(
        &[
            TimeframeInput {
                timeframe_secs: execution_secs,
                candles: &execution,
            },
            TimeframeInput {
                timeframe_secs: context_secs,
                candles: &context,
            },
        ],
        &MarketStateConfig::default(),
    )
}

#[test]
fn context_uses_the_latest_bar_closed_by_the_execution_close() {
    let run = make_run(300, 7, 900, 3);
    let ladder = LadderConfig::new(300, vec![900]).expect("the ladder is valid");
    let contexts = build(&run, &ladder, &LadderWeights::equal(1)).expect("context builds");

    assert_eq!(contexts.len(), 7);
    assert_eq!(contexts[0].missing_timeframes, vec![900]);
    assert_eq!(contexts[1].missing_timeframes, vec![900]);
    assert_eq!(
        contexts[2].missing_timeframes,
        Vec::<u64>::new(),
        "the 15m bar closes exactly at the 5m close"
    );
    assert_eq!(contexts[2].context[0].bar_index, 0);
    assert_eq!(contexts[2].context[0].close_time, 900);
    assert_eq!(contexts[6].context[0].bar_index, 1);
    assert_eq!(contexts[6].execution_close_time, 2_100);
    assert!(!contexts[1].context_complete());
    assert!(contexts[2].context_complete());
}

#[test]
fn future_bars_do_not_change_an_earlier_context() {
    let prefix_run = make_run(300, 8, 900, 4);
    let full_run = make_run(300, 16, 900, 8);
    let ladder = LadderConfig::new(300, vec![900]).expect("the ladder is valid");
    let weights = LadderWeights::equal(1);

    let prefix = build(&prefix_run, &ladder, &weights).expect("prefix builds");
    let full = build(&full_run, &ladder, &weights).expect("full builds");

    assert_eq!(&prefix[..], &full[..prefix.len()]);
}

#[test]
fn repeated_runs_are_identical() {
    let run = make_run(300, 12, 900, 4);
    let ladder = LadderConfig::new(300, vec![900]).expect("the ladder is valid");
    let weights = LadderWeights::equal(1);

    let first = build(&run, &ladder, &weights).expect("first builds");
    let second = build(&run, &ladder, &weights).expect("second builds");
    assert_eq!(first, second);
}

#[test]
fn weights_change_the_weighted_context_not_the_counts() {
    let mut equal = AxisAccumulator::default();
    equal.push(300, Some(0.8), 1.0);
    equal.push(900, Some(-0.8), 1.0);
    let equal = equal.finish();

    assert_eq!(equal.weighted_value, Some(0.0));
    assert_eq!(equal.bullish_count, 1);
    assert_eq!(equal.bearish_count, 1);
    assert_eq!(equal.bullish_share, Some(0.5));
    assert_eq!(equal.bearish_share, Some(0.5));
    assert_eq!(equal.agreement, AgreementCode::Mixed);

    let mut weighted = AxisAccumulator::default();
    weighted.push(300, Some(0.8), 3.0);
    weighted.push(900, Some(-0.8), 1.0);
    let weighted = weighted.finish();

    assert!((weighted.weighted_value.expect("weighted value") - 0.4).abs() < 1e-12);
    assert_eq!(weighted.bullish_share, Some(0.75));
    assert_eq!(weighted.bearish_share, Some(0.25));
    assert_eq!(weighted.agreement, AgreementCode::Bullish);
}

#[test]
fn structure_conflicts_and_progression_stages_are_counted() {
    let run = make_run(300, 2, 900, 1);
    let context = run
        .timeframe(900)
        .expect("the context timeframe exists")
        .bar(0)
        .expect("the context bar exists");
    let mut bar = context;
    bar.structure.high_relation_code = Some(1);
    bar.structure.low_relation_code = Some(-1);
    bar.structure.score = Some(0.0);
    bar.progression.leg_direction = Some(1);
    bar.progression.stage_code = Some(1);

    let selected = vec![(
        ContextTimeframe {
            timeframe_secs: 900,
            bar_index: 0,
            open_time: 0,
            close_time: 900,
        },
        bar,
    )];
    let weights = LadderWeights::equal(1);
    let (_, _, structure, progression) = compute_axes(&selected, &weights);

    assert_eq!(structure.conflict_count, 1);
    assert_eq!(structure.score.bullish_count, 0);
    assert_eq!(structure.score.neutral_count, 1);
    assert_eq!(progression.stage_histogram[1], 1.0);
    assert_eq!(progression.late_or_terminal_count, 0);
    assert_eq!(progression.direction.bullish_count, 1);
}

#[test]
fn level_context_uses_a_confirmed_level_at_or_before_the_execution_bar() {
    let execution: Vec<Candle> = (0..46)
        .map(|index| {
            let timestamp = index as i64 * 300;
            if index == 45 {
                Candle::new(
                    timestamp as u64,
                    timestamp,
                    91.5,
                    93.0,
                    89.5,
                    91.5,
                    Some(10.0),
                )
                .expect("the execution candle is valid")
            } else {
                candle(timestamp, 92.5)
            }
        })
        .collect();
    let closes = [
        100.0, 99.0, 98.0, 97.0, 96.0, 95.0, 94.0, 93.0, 92.0, 91.0, 92.0, 93.0, 94.0, 95.0, 96.0,
        97.0, 98.0, 99.0, 100.0, 101.0,
    ];
    let context: Vec<Candle> = closes
        .iter()
        .enumerate()
        .map(|(index, close)| candle(index as i64 * 900, *close))
        .collect();
    let run = ModelRun::compute(
        &[
            TimeframeInput {
                timeframe_secs: 300,
                candles: &execution,
            },
            TimeframeInput {
                timeframe_secs: 900,
                candles: &context,
            },
        ],
        &MarketStateConfig::default(),
    );
    let context_model = run.timeframe(900).expect("the context timeframe exists");
    assert!(
        !context_model.levels().is_empty(),
        "the V-shaped context must confirm at least one pivot"
    );

    let ladder = LadderConfig::new(300, vec![900]).expect("the ladder is valid");
    let contexts = build(&run, &ladder, &LadderWeights::equal(1)).expect("context builds");
    let level = contexts[45]
        .levels
        .support
        .expect("the confirmed low is a support");
    assert!((level.price - 90.0).abs() < 1e-12);
    assert!(level.available_at <= 14);
    assert!(contexts[45].levels.retest_support);
    assert!(contexts[45].levels.support_distance_fraction.is_some());
}

#[test]
fn two_context_timeframes_use_their_own_weights_and_order() {
    let run = make_run(300, 2, 900, 1);
    let base = run
        .timeframe(900)
        .expect("the context timeframe exists")
        .bar(0)
        .expect("the context bar exists");

    let mut fast = base;
    fast.timeframe_secs = 900;
    fast.direction.direction = Some(0.8);
    fast.momentum.force = Some(0.4);
    fast.structure.score = Some(0.2);
    fast.structure.high_relation_code = None;
    fast.structure.low_relation_code = None;
    fast.progression.leg_direction = Some(1);
    fast.progression.stage_code = Some(1);

    let mut slow = base;
    slow.timeframe_secs = 3_600;
    slow.direction.direction = Some(-0.2);
    slow.momentum.force = Some(0.6);
    slow.structure.score = Some(0.6);
    slow.structure.high_relation_code = None;
    slow.structure.low_relation_code = None;
    slow.progression.leg_direction = Some(-1);
    slow.progression.stage_code = Some(4);

    let selected = vec![
        (
            ContextTimeframe {
                timeframe_secs: 900,
                bar_index: 0,
                open_time: 0,
                close_time: 900,
            },
            fast,
        ),
        (
            ContextTimeframe {
                timeframe_secs: 3_600,
                bar_index: 0,
                open_time: 0,
                close_time: 3_600,
            },
            slow,
        ),
    ];
    let weights = LadderWeights::new(
        vec![3.0, 1.0],
        vec![1.0, 3.0],
        vec![1.0, 1.0],
        vec![2.0, 1.0],
    )
    .expect("the weights are valid");

    let (direction, momentum, structure, progression) = compute_axes(&selected, &weights);

    assert_eq!(direction.timeframes, vec![900, 3_600]);
    assert!((direction.weighted_value.expect("direction") - 0.55).abs() < 1e-12);
    assert_eq!(direction.bullish_share, Some(0.75));
    assert_eq!(direction.bearish_share, Some(0.25));

    assert!((momentum.weighted_value.expect("momentum") - 0.55).abs() < 1e-12);
    assert_eq!(momentum.bullish_count, 2);
    assert_eq!(momentum.neutral_count, 0);

    assert!((structure.score.weighted_value.expect("structure") - 0.4).abs() < 1e-12);
    assert_eq!(structure.conflict_count, 0);

    assert!(progression.direction.weighted_value.expect("progression") > 0.0);
    assert_eq!(progression.stage_histogram[1], 2.0 / 3.0);
    assert_eq!(progression.stage_histogram[4], 1.0 / 3.0);
    assert_eq!(progression.late_or_terminal_count, 1);
}

#[test]
fn a_missing_context_timeframe_rejects_the_axis_context() {
    let run = make_run(300, 1, 900, 1);
    let ladder = LadderConfig::new(300, vec![900]).expect("the ladder is valid");
    let context = build(&run, &ladder, &LadderWeights::equal(1)).expect("context builds");

    assert_eq!(context.len(), 1);
    assert_eq!(context[0].missing_timeframes, vec![900]);
    assert_eq!(context[0].direction.agreement, AgreementCode::Unavailable);
    assert_eq!(context[0].momentum.agreement, AgreementCode::Unavailable);
    assert_eq!(
        context[0].structure.score.agreement,
        AgreementCode::Unavailable
    );
    assert_eq!(
        context[0].progression.direction.agreement,
        AgreementCode::Unavailable
    );
}

#[test]
#[ignore = "manual 600k-bar context benchmark"]
fn context_build_benchmark() {
    use std::time::Instant;

    let run = make_run(60, 600_000, 3_600, 10_000);
    let ladder = LadderConfig::new(60, vec![3_600]).expect("the ladder is valid");
    let weights = LadderWeights::equal(1);
    let mut builder = LadderContextBuilder::new(&run, &ladder, &weights).expect("builder");
    let started = Instant::now();
    let mut count = 0;
    while builder.next_context().expect("context builds").is_some() {
        count += 1;
    }
    eprintln!(
        "context benchmark: {count} execution bars in {:.3}s",
        started.elapsed().as_secs_f64()
    );
    assert_eq!(count, 600_000);
}

#[test]
fn invalid_ladders_and_weights_are_rejected() {
    assert_eq!(
        LadderConfig::new(0, vec![900]),
        Err(LadderContextError::ZeroExecutionTimeframe)
    );
    assert_eq!(
        LadderConfig::new(300, Vec::new()),
        Err(LadderContextError::EmptyContext)
    );
    assert!(LadderConfig::new(900, vec![300]).is_err());
    assert!(LadderConfig::new(300, vec![300]).is_err());
    assert_eq!(
        LadderWeights::new(Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        Err(LadderContextError::EmptyWeights)
    );
    assert!(LadderWeights::new(vec![1.0], vec![1.0, 2.0], vec![1.0], vec![1.0]).is_err());
    assert!(LadderWeights::new(vec![0.0], vec![1.0], vec![1.0], vec![1.0]).is_err());
    assert!(LadderWeights::new(vec![f64::NAN], vec![1.0], vec![1.0], vec![1.0]).is_err());
    assert_eq!(
        LadderWeights::new(
            vec![1e308, 1e308],
            vec![1.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 1.0]
        ),
        Err(LadderContextError::InvalidWeightSum { axis: 0 })
    );
}
