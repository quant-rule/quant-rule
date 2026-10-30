use quantrule_data::Candle;

use super::{
    ChurnEvidence, agreement, axis_agreement, churn_class, churn_evidence, code_bearing_counts,
    summarize_churn,
};
use crate::aggregate::CrossTimeframeAggregates;
use crate::dashboard::{
    AxisValue, ChurnClass, ChurnSource, Coverage, DashboardConfig, DashboardState, Domain,
    MajorityClass, TimeframeState,
};
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::readings::PivotScale;
use crate::readings::roles::AxisCode;
use crate::run::ModelRun;
use crate::timeframe::TimeframeModel;

fn view(run: &ModelRun, ladder: &[u64]) -> DashboardState {
    let aggregates = CrossTimeframeAggregates::compute(&run.snapshots());
    DashboardState::from_run(run, ladder, &DashboardConfig::default(), &aggregates)
}

fn state(timeframe_secs: u64, present: bool, positions: [Option<f64>; 3]) -> TimeframeState {
    let mut state = TimeframeState::absent(timeframe_secs, PivotScale::Major.code());
    state.present = present;
    if present {
        state.latest_bar = Some(0);
        state.window_len = 1;
    }

    for (axis, position) in state.axes.iter_mut().zip(positions) {
        let Some(value) = position else {
            continue;
        };
        let domain = match axis.axis {
            AxisCode::Direction => Domain::DirectionalBias,
            AxisCode::Momentum => Domain::Acceleration,
            AxisCode::Structure => Domain::StructureRelation,
            AxisCode::Progression => continue,
        };
        axis.position = AxisValue::signed(value, domain);
    }

    state
}

fn coverage(present: u32, configured: u32) -> Coverage {
    Coverage {
        present,
        configured,
    }
}

#[test]
fn a_strict_majority_needs_more_than_half() {
    let view = axis_agreement(
        AxisCode::Direction,
        [Some(0.20), Some(0.30), Some(-0.50)],
        2,
    );

    assert_eq!(view.valid_timeframes, 3);
    assert_eq!(view.bullish_count, 2);
    assert_eq!(view.bearish_count, 1);
    assert_eq!(view.neutral_count, 0);
    assert_eq!(view.majority, Some(MajorityClass::Bullish));
    assert_eq!(view.majority_share, Some(2.0 / 3.0));
    assert!(view.agrees);
    assert!(!view.insufficient_coverage);
}

#[test]
fn an_exact_half_is_not_a_majority() {
    let view = axis_agreement(AxisCode::Momentum, [Some(0.5), Some(-0.5)], 2);

    assert_eq!(view.valid_timeframes, 2);
    assert_eq!(view.majority, None);
    assert_eq!(view.majority_share, None);
    assert!(!view.agrees);
    assert!(!view.insufficient_coverage);
}

#[test]
fn a_three_way_split_is_not_a_majority() {
    let view = axis_agreement(AxisCode::Structure, [Some(0.5), Some(-0.5), Some(0.0)], 2);

    assert_eq!(view.valid_timeframes, 3);
    assert_eq!(view.majority, None);
    assert!(!view.agrees);
}

#[test]
fn neutral_is_a_measured_class_and_the_corridor_is_inclusive() {
    let view = axis_agreement(AxisCode::Structure, [Some(0.1), Some(-0.1), Some(0.0)], 2);

    assert_eq!(view.valid_timeframes, 3);
    assert_eq!(view.neutral_count, 3);
    assert_eq!(view.majority, Some(MajorityClass::Neutral));
    assert_eq!(view.majority_share, Some(1.0));
    assert!(view.agrees);
}

#[test]
fn missing_positions_are_excluded_not_neutral() {
    let view = axis_agreement(AxisCode::Direction, [Some(0.5), None, Some(0.6)], 2);

    assert_eq!(view.valid_timeframes, 2);
    assert_eq!(view.bullish_count, 2);
    assert_eq!(view.neutral_count, 0);
    assert_eq!(view.majority, Some(MajorityClass::Bullish));
    assert!(view.agrees);
}

#[test]
fn insufficient_coverage_has_no_majority_even_when_one_class_is_unanimous() {
    let view = axis_agreement(AxisCode::Direction, [Some(0.5), None], 2);

    assert_eq!(view.valid_timeframes, 1);
    assert_eq!(view.bullish_count, 1);
    assert_eq!(view.majority, None);
    assert_eq!(view.majority_share, None);
    assert!(!view.agrees);
    assert!(view.insufficient_coverage);
}

#[test]
fn non_finite_positions_are_not_valid_votes() {
    let view = axis_agreement(
        AxisCode::Momentum,
        [Some(f64::NAN), Some(0.5), Some(-0.5)],
        2,
    );

    assert_eq!(view.valid_timeframes, 2);
    assert_eq!(view.neutral_count, 0);
    assert_eq!(view.majority, None);
}

#[test]
fn agreement_publishes_three_axes_and_counts_agrees() {
    let timeframes = vec![
        state(60, true, [Some(0.5), Some(0.2), Some(-0.05)]),
        state(300, true, [Some(0.4), None, Some(0.0)]),
        state(900, false, [Some(-0.5), Some(-0.5), Some(-0.5)]),
    ];
    let view = agreement(&timeframes, &DashboardConfig::default(), coverage(2, 3));

    assert_eq!(view.of, 3);
    assert_eq!(view.agreeing_axes, 2);
    assert_eq!(view.axes.len(), 3);
    assert_eq!(view.axes[0].axis, AxisCode::Direction);
    assert_eq!(view.axes[1].axis, AxisCode::Momentum);
    assert_eq!(view.axes[2].axis, AxisCode::Structure);

    assert_eq!(view.axes[0].majority, Some(MajorityClass::Bullish));
    assert!(view.axes[0].agrees);

    assert_eq!(view.axes[1].valid_timeframes, 1);
    assert!(view.axes[1].insufficient_coverage);
    assert!(!view.axes[1].agrees);

    assert_eq!(view.axes[2].majority, Some(MajorityClass::Neutral));
    assert!(view.axes[2].agrees);
    assert_eq!(view.coverage, coverage(2, 3));
}

#[test]
fn an_absent_timeframe_never_votes() {
    let timeframes = vec![
        state(60, true, [Some(0.5), Some(0.5), Some(0.5)]),
        state(300, false, [Some(0.5), Some(0.5), Some(0.5)]),
    ];
    let view = agreement(&timeframes, &DashboardConfig::default(), coverage(1, 2));

    assert_eq!(view.coverage, coverage(1, 2));
    for axis in &view.axes {
        assert_eq!(axis.valid_timeframes, 1);
        assert_eq!(axis.bullish_count, 1);
        assert_eq!(axis.neutral_count, 0);
        assert!(axis.insufficient_coverage);
        assert!(!axis.agrees);
    }
}

#[test]
fn a_minimum_above_the_valid_count_removes_every_majority() {
    let timeframes = vec![
        state(60, true, [Some(0.5), Some(0.2), Some(0.0)]),
        state(300, true, [Some(0.4), Some(0.3), Some(0.0)]),
    ];
    let view = agreement(&timeframes, &DashboardConfig::new(6, 3), coverage(2, 2));

    assert_eq!(view.min_valid_timeframes, 3);
    assert_eq!(view.agreeing_axes, 0);
    assert!(view.axes.iter().all(|axis| axis.insufficient_coverage));
    assert!(view.axes.iter().all(|axis| axis.majority.is_none()));
}

#[test]
fn churn_ties_publish_every_source_sorted_by_timeframe_then_axis() {
    let mut evidence = Vec::new();
    evidence.extend_from_slice(&full_evidence(60, [3, 3, 1, 0]));
    evidence.extend_from_slice(&full_evidence(300, [2, 3, 0, 0]));

    let view = summarize_churn(
        &evidence,
        &[60, 300],
        &DashboardConfig::default(),
        coverage(2, 2),
    );

    assert_eq!(view.max_changes_50, 3);
    assert_eq!(
        view.sources,
        vec![
            ChurnSource {
                timeframe_secs: 60,
                axis: AxisCode::Direction,
            },
            ChurnSource {
                timeframe_secs: 60,
                axis: AxisCode::Momentum,
            },
            ChurnSource {
                timeframe_secs: 300,
                axis: AxisCode::Momentum,
            },
        ]
    );
    assert!(view.history_complete);
    assert_eq!(view.churn_class, ChurnClass::Holding);
    assert_eq!(view.window, 50);
}

#[test]
fn no_code_bearing_history_has_no_sources() {
    let evidence = [
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Direction,
            changes_50: 0,
            code_bearing_bars: 0,
        },
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Momentum,
            changes_50: 0,
            code_bearing_bars: 0,
        },
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Structure,
            changes_50: 0,
            code_bearing_bars: 0,
        },
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Progression,
            changes_50: 0,
            code_bearing_bars: 0,
        },
    ];
    let view = summarize_churn(
        &evidence,
        &[60],
        &DashboardConfig::default(),
        coverage(1, 1),
    );

    assert_eq!(view.max_changes_50, 0);
    assert!(view.sources.is_empty());
    assert!(!view.history_complete);
    assert_eq!(view.churn_class, ChurnClass::Building);
}

#[test]
fn an_axis_without_code_bearing_history_is_not_a_source() {
    let evidence = [
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Direction,
            changes_50: 0,
            code_bearing_bars: 50,
        },
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Momentum,
            // A counter without any code-bearing bar is not a measurement, so
            // it must not raise the maximum or enter the source list.
            changes_50: 99,
            code_bearing_bars: 0,
        },
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Structure,
            changes_50: 0,
            code_bearing_bars: 50,
        },
        ChurnEvidence {
            timeframe_secs: 60,
            axis: AxisCode::Progression,
            changes_50: 0,
            code_bearing_bars: 50,
        },
    ];
    let view = summarize_churn(
        &evidence,
        &[60],
        &DashboardConfig::default(),
        coverage(1, 1),
    );

    assert_eq!(view.max_changes_50, 0);
    assert_eq!(
        view.sources,
        vec![
            ChurnSource {
                timeframe_secs: 60,
                axis: AxisCode::Direction,
            },
            ChurnSource {
                timeframe_secs: 60,
                axis: AxisCode::Structure,
            },
            ChurnSource {
                timeframe_secs: 60,
                axis: AxisCode::Progression,
            },
        ]
    );
    assert!(!view.history_complete);
    assert_eq!(view.churn_class, ChurnClass::Building);
}

#[test]
fn history_requires_every_configured_timeframe_and_axis() {
    let mut evidence = Vec::new();
    evidence.extend_from_slice(&full_evidence(60, [0, 0, 0, 0]));
    evidence.extend_from_slice(&full_evidence(300, [0, 0, 0, 0]));
    let config = DashboardConfig::default();

    let complete = summarize_churn(&evidence, &[60, 300], &config, coverage(2, 2));
    assert!(complete.history_complete);

    let without_axis: Vec<ChurnEvidence> = evidence
        .iter()
        .copied()
        .filter(|item| !(item.timeframe_secs == 300 && item.axis == AxisCode::Structure))
        .collect();
    let missing_axis = summarize_churn(&without_axis, &[60, 300], &config, coverage(2, 2));
    assert!(!missing_axis.history_complete);

    let without_timeframe: Vec<ChurnEvidence> = evidence
        .iter()
        .copied()
        .filter(|item| item.timeframe_secs == 60)
        .collect();
    let missing_timeframe =
        summarize_churn(&without_timeframe, &[60, 300], &config, coverage(1, 2));
    assert!(!missing_timeframe.history_complete);

    let one_bar_short: Vec<ChurnEvidence> = evidence
        .iter()
        .copied()
        .map(|mut item| {
            if item.timeframe_secs == 300 && item.axis == AxisCode::Progression {
                item.code_bearing_bars = 49;
            }
            item
        })
        .collect();
    let short = summarize_churn(&one_bar_short, &[60, 300], &config, coverage(2, 2));
    assert!(!short.history_complete);

    let empty = summarize_churn(&evidence, &[], &config, coverage(0, 0));
    assert!(!empty.history_complete);
}

#[test]
fn a_short_present_series_is_building() {
    let run = run(&candles(10), &[60]);
    let view = view(&run, &[60, 300]);

    assert_eq!(view.cross_tf_state.present_timeframes, vec![60]);
    assert_eq!(view.cross_tf_state.agreement.coverage, coverage(1, 2));
    assert!(
        view.cross_tf_state
            .agreement
            .axes
            .iter()
            .all(|axis| axis.insufficient_coverage && !axis.agrees)
    );
    assert!(!view.cross_tf_state.churn.history_complete);
    assert_eq!(view.cross_tf_state.churn.churn_class, ChurnClass::Building);
}

#[test]
fn the_threshold_is_at_or_above_and_the_default_is_six() {
    let default = DashboardConfig::default();
    assert_eq!(default.churn_threshold, 6);
    assert_eq!(default.min_cross_tf_members, 2);
    assert_eq!(churn_class(false, 10, 6), ChurnClass::Building);
    assert_eq!(churn_class(true, 5, 6), ChurnClass::Holding);
    assert_eq!(churn_class(true, 6, 6), ChurnClass::Unstable);
    assert_eq!(churn_class(true, 7, 6), ChurnClass::Unstable);
    assert_eq!(churn_class(true, 0, 0), ChurnClass::Unstable);
}

fn full_evidence(timeframe_secs: u64, changes: [u64; 4]) -> Vec<ChurnEvidence> {
    AxisCode::ALL
        .into_iter()
        .zip(changes)
        .map(|(axis, changes_50)| ChurnEvidence {
            timeframe_secs,
            axis,
            changes_50,
            code_bearing_bars: 50,
        })
        .collect()
}

fn candles(count: usize) -> Vec<Candle> {
    (0..count)
        .map(|index| {
            let wave = (index as f64 / 7.0).sin() * 2.0;
            let drift = index as f64 * 0.02;
            let open = 100.0 + wave + drift;
            let close = open + (index % 5) as f64 * 0.05 - 0.1;
            let high = open.max(close) + 0.4;
            let low = open.min(close) - 0.4;
            Candle::new(
                index as u64,
                1_700_000_000 + index as i64 * 60,
                open,
                high,
                low,
                close,
                Some(1_000.0 + index as f64),
            )
            .expect("a valid synthetic candle")
        })
        .collect()
}

fn model(candles: &[Candle]) -> TimeframeModel {
    TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: 60,
            candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("a non-empty synthetic series")
}

fn run(candles: &[Candle], timeframes: &[u64]) -> ModelRun {
    let inputs: Vec<TimeframeInput<'_>> = timeframes
        .iter()
        .map(|&timeframe_secs| TimeframeInput {
            timeframe_secs,
            candles,
        })
        .collect();
    ModelRun::compute(&inputs, &MarketStateConfig::default())
}

#[test]
fn the_view_summaries_follow_the_published_values() {
    let candles = candles(200);
    let run = run(&candles, &[60, 300]);
    let view = view(&run, &[60, 300]);

    let agreement = &view.cross_tf_state.agreement;
    assert_eq!(agreement.of, 3);
    assert_eq!(
        agreement
            .axes
            .iter()
            .map(|axis| axis.axis)
            .collect::<Vec<_>>(),
        vec![AxisCode::Direction, AxisCode::Momentum, AxisCode::Structure]
    );
    for axis in &agreement.axes {
        assert_eq!(
            u32::from(axis.valid_timeframes),
            u32::from(axis.bullish_count)
                + u32::from(axis.bearish_count)
                + u32::from(axis.neutral_count)
        );
        assert_eq!(axis.agrees, axis.majority.is_some());
        assert_eq!(
            axis.insufficient_coverage,
            axis.valid_timeframes < agreement.min_valid_timeframes
        );
        match axis.majority {
            Some(class) => {
                let count = match class {
                    MajorityClass::Bullish => axis.bullish_count,
                    MajorityClass::Bearish => axis.bearish_count,
                    MajorityClass::Neutral => axis.neutral_count,
                };
                assert!(2 * u32::from(count) > u32::from(axis.valid_timeframes));
                assert_eq!(
                    axis.majority_share,
                    Some(f64::from(count) / f64::from(axis.valid_timeframes))
                );
            }
            None => assert_eq!(axis.majority_share, None),
        }
    }
    assert_eq!(
        u32::from(agreement.agreeing_axes),
        agreement.axes.iter().filter(|axis| axis.agrees).count() as u32
    );

    let churn = &view.cross_tf_state.churn;
    let mut expected_max = 0_u64;
    let mut expected_sources = Vec::new();
    for timeframe_secs in &view.cross_tf_state.present_timeframes {
        let timeframe = run.timeframe(*timeframe_secs).expect("a present timeframe");
        let bar = timeframe
            .latest_bar()
            .expect("a present timeframe has a bar");
        // The summary excludes an axis only when its whole loaded history has
        // no code, not merely when the latest bar withholds one.
        let has_code = |axis: AxisCode| {
            (0..=bar.bar_index).any(|index| {
                let prior = timeframe.bar(index).expect("an in-range bar");
                match axis {
                    AxisCode::Direction => prior.direction.state_code.is_some(),
                    AxisCode::Momentum => prior.momentum.state_code.is_some(),
                    AxisCode::Structure => prior.structure.state_code.is_some(),
                    AxisCode::Progression => prior.progression.stage_code.is_some(),
                }
            })
        };
        let axes = [
            (
                AxisCode::Direction,
                bar.direction.state_changes_50,
                has_code(AxisCode::Direction),
            ),
            (
                AxisCode::Momentum,
                bar.momentum.state_changes_50,
                has_code(AxisCode::Momentum),
            ),
            (
                AxisCode::Structure,
                bar.structure.state_changes_50,
                has_code(AxisCode::Structure),
            ),
            (
                AxisCode::Progression,
                bar.progression.state_changes_50,
                has_code(AxisCode::Progression),
            ),
        ];
        for (axis, changes, code_bearing) in axes {
            if !code_bearing {
                continue;
            }
            if changes > expected_max {
                expected_max = changes;
                expected_sources.clear();
                expected_sources.push(ChurnSource {
                    timeframe_secs: *timeframe_secs,
                    axis,
                });
            } else if changes == expected_max {
                expected_sources.push(ChurnSource {
                    timeframe_secs: *timeframe_secs,
                    axis,
                });
            }
        }
    }
    expected_sources.sort_by_key(|source| (source.timeframe_secs, source.axis));

    assert_eq!(u64::from(churn.max_changes_50), expected_max);
    assert_eq!(churn.sources, expected_sources);
    assert_eq!(churn.window, 50);
    assert_eq!(churn.threshold, DashboardConfig::default().churn_threshold);
    assert_eq!(
        churn.churn_class == ChurnClass::Building,
        !churn.history_complete
    );
}

#[test]
fn agreement_at_an_earlier_bar_ignores_later_bars() {
    let all = candles(140);
    let cut = 100;
    let full = model(&all);
    let truncated = model(&all[..=cut]);
    let config = DashboardConfig::default();
    let coverage = coverage(1, 1);

    let full_state = TimeframeState::at(&full, cut).expect("the full model has the bar");
    let truncated_state =
        TimeframeState::at(&truncated, cut).expect("the truncated model has the bar");
    let full_agreement = agreement(std::slice::from_ref(&full_state), &config, coverage);
    let truncated_agreement = agreement(std::slice::from_ref(&truncated_state), &config, coverage);

    assert_eq!(full_agreement, truncated_agreement);
    assert!(
        full_agreement
            .axes
            .iter()
            .any(|axis| axis.valid_timeframes > 0),
        "the fixture must exercise at least one valid axis"
    );
}

#[test]
fn churn_evidence_at_an_earlier_bar_ignores_later_bars() {
    let all = candles(140);
    let cut = 100;
    let full = model(&all);
    let truncated = model(&all[..=cut]);
    let config = DashboardConfig::default();
    let coverage = coverage(1, 1);

    let full_evidence = churn_evidence(&full, cut).expect("the full model has the bar");
    let truncated_evidence =
        churn_evidence(&truncated, cut).expect("the truncated model has the bar");
    assert_eq!(full_evidence, truncated_evidence);

    let full_churn = summarize_churn(&full_evidence, &[60], &config, coverage);
    let truncated_churn = summarize_churn(&truncated_evidence, &[60], &config, coverage);
    assert_eq!(full_churn, truncated_churn);
}

#[test]
fn code_bearing_counts_match_a_manual_count() {
    let all = candles(120);
    let full = model(&all);
    let view = 90;
    let counts = code_bearing_counts(&full, view).expect("the view is in range");

    for axis in AxisCode::ALL {
        let expected = (0..=view)
            .filter(|index| {
                let bar = full.bar(*index).expect("an in-range bar");
                match axis {
                    AxisCode::Direction => bar.direction.state_code.is_some(),
                    AxisCode::Momentum => bar.momentum.state_code.is_some(),
                    AxisCode::Structure => bar.structure.state_code.is_some(),
                    AxisCode::Progression => bar.progression.stage_code.is_some(),
                }
            })
            .count();
        assert_eq!(counts[axis as usize - 1], expected, "axis {axis:?}");
    }
}

#[test]
fn an_empty_run_has_no_agreement_or_complete_churn() {
    let run = ModelRun::compute(&[], &MarketStateConfig::default());
    let view = view(&run, &[60, 300]);

    assert_eq!(view.cross_tf_state.agreement.axes.len(), 3);
    assert_eq!(view.cross_tf_state.agreement.agreeing_axes, 0);
    assert!(
        view.cross_tf_state
            .agreement
            .axes
            .iter()
            .all(|axis| axis.valid_timeframes == 0 && !axis.agrees)
    );
    assert_eq!(view.cross_tf_state.churn.max_changes_50, 0);
    assert!(view.cross_tf_state.churn.sources.is_empty());
    assert!(!view.cross_tf_state.churn.history_complete);
    assert_eq!(view.cross_tf_state.churn.churn_class, ChurnClass::Building);
}
