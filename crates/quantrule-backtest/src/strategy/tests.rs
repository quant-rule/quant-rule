//! Tests for the unified ladder-context strategy and its configuration.

use crate::context::{
    AgreementCode, AgreementContext, AxisContext, LadderContext, LevelContext, LevelView,
    ProgressionContext, StructureContext,
};
use crate::intent::StrategyPosition;
use crate::side::Side;

use super::*;

fn axis(bullish: f64, bearish: f64, agreement: AgreementCode) -> AxisContext {
    AxisContext {
        timeframes: vec![900, 3_600],
        bullish_count: u32::from(bullish > 0.0),
        bearish_count: u32::from(bearish > 0.0),
        neutral_count: u32::from(bullish == 0.0 && bearish == 0.0),
        weighted_value: Some(bullish - bearish),
        bullish_share: Some(bullish),
        bearish_share: Some(bearish),
        neutral_share: Some((1.0 - bullish - bearish).max(0.0)),
        agreement,
    }
}

fn context(
    complete: bool,
    direction: AxisContext,
    momentum: AxisContext,
    conflict_count: u32,
    progression: AgreementCode,
    late_or_terminal_count: u32,
) -> LadderContext {
    LadderContext {
        execution_timeframe_secs: 300,
        execution_bar_index: 0,
        execution_open_time: 0,
        execution_close_time: 300,
        context: Vec::new(),
        missing_timeframes: if complete { Vec::new() } else { vec![3_600] },
        direction,
        momentum,
        structure: StructureContext {
            score: axis(0.0, 0.0, AgreementCode::Mixed),
            conflict_count,
        },
        progression: ProgressionContext {
            direction: axis(
                if progression == AgreementCode::Bullish {
                    1.0
                } else {
                    0.0
                },
                if progression == AgreementCode::Bearish {
                    1.0
                } else {
                    0.0
                },
                progression,
            ),
            stage_histogram: [0.0; 6],
            late_or_terminal_count,
        },
        levels: LevelContext::unavailable(),
        agreement: AgreementContext::unavailable(),
    }
}

fn bullish_context() -> LadderContext {
    context(
        true,
        axis(1.0, 0.0, AgreementCode::Bullish),
        axis(0.5, 0.0, AgreementCode::Bullish),
        0,
        AgreementCode::Bullish,
        0,
    )
}

fn bearish_context() -> LadderContext {
    context(
        true,
        axis(0.0, 1.0, AgreementCode::Bearish),
        axis(0.0, 0.5, AgreementCode::Bearish),
        0,
        AgreementCode::Bearish,
        0,
    )
}

fn long_input(bar_index: usize) -> StrategyInput {
    StrategyInput::new(
        300,
        bar_index,
        bar_index as i64 * 300,
        Some(0.2),
        Some(0.2),
        Some(0.0),
    )
    .with_structure(Some(1), Some(0))
}

fn short_input(bar_index: usize) -> StrategyInput {
    StrategyInput::new(
        300,
        bar_index,
        bar_index as i64 * 300,
        Some(-0.2),
        Some(-0.2),
        Some(0.0),
    )
    .with_structure(Some(-1), Some(0))
}

fn decide(
    strategy: &LadderContextStrategy,
    input: &StrategyInput,
    context: &LadderContext,
    position: StrategyPosition,
) -> Decision {
    let mut context = context.clone();
    context.execution_timeframe_secs = input.timeframe_secs;
    context.execution_bar_index = input.bar_index;
    context.execution_open_time = input.open_time;
    strategy.decide(input, &context, position)
}

#[test]
fn the_control_config_is_the_default_ladder() {
    let config = StrategyConfig::default();
    assert_eq!(config.timeframe_secs(), 300);
    assert_eq!(config.ladder().context_timeframes(), &[900]);
    assert_eq!(config.weights().direction(), &[1.0]);
    assert_eq!(config.entry().execution().direction_min(), 0.1);
    assert_eq!(config.entry().context().direction_share_min(), 0.5);
    assert!(config.exit().on_context_lost());
    assert!(config.exit().on_structure_lost());
    assert_eq!(
        config
            .risk()
            .expect("the control has a risk policy")
            .stop_atr_multiple(),
        2.0
    );
    assert!(config.require_contiguous_execution());
}

#[test]
fn a_long_entry_needs_every_execution_and_context_condition() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(0);
    let context = bullish_context();

    assert!(matches!(
        decide(&strategy, &input, &context, StrategyPosition::flat()),
        Decision::Enter(entry) if entry.intent.side == Side::Long
    ));

    let cases = [
        ("direction", input.direction, Some(0.1)),
        ("momentum", input.momentum_force, Some(0.1)),
        ("structure", input.structure_score, None),
    ];
    let mut changed = input;
    changed.direction = cases[0].2;
    assert_eq!(
        decide(&strategy, &changed, &context, StrategyPosition::flat()),
        Decision::Hold
    );
    changed = input;
    changed.momentum_force = cases[1].2;
    assert_eq!(
        decide(&strategy, &changed, &context, StrategyPosition::flat()),
        Decision::Hold
    );
    changed = input;
    changed.structure_score = None;
    assert_eq!(
        decide(&strategy, &changed, &context, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_short_entry_mirrors_every_condition() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = short_input(0);
    let context = bearish_context();

    assert!(matches!(
        decide(&strategy, &input, &context, StrategyPosition::flat()),
        Decision::Enter(entry) if entry.intent.side == Side::Short
    ));

    let mut changed = input;
    changed.direction = Some(-0.1);
    assert_eq!(
        decide(&strategy, &changed, &context, StrategyPosition::flat()),
        Decision::Hold
    );
    changed = input;
    changed.structure_state_code = Some(0);
    assert_eq!(
        decide(&strategy, &changed, &context, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_structure_shift_needs_both_state_codes() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let context = bullish_context();

    let no_previous = long_input(0).with_structure(Some(1), None);
    assert_eq!(
        decide(&strategy, &no_previous, &context, StrategyPosition::flat()),
        Decision::Hold
    );

    let no_current = long_input(0).with_structure(None, Some(0));
    assert_eq!(
        decide(&strategy, &no_current, &context, StrategyPosition::flat()),
        Decision::Hold
    );

    let no_shift = long_input(0).with_structure(Some(0), Some(0));
    assert_eq!(
        decide(&strategy, &no_shift, &context, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn missing_or_conflicting_context_blocks_an_entry() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(0);

    let missing = context(
        false,
        axis(1.0, 0.0, AgreementCode::Bullish),
        axis(0.5, 0.0, AgreementCode::Bullish),
        0,
        AgreementCode::Bullish,
        0,
    );
    assert_eq!(
        decide(&strategy, &input, &missing, StrategyPosition::flat()),
        Decision::Hold
    );

    let conflict = context(
        true,
        axis(1.0, 0.0, AgreementCode::Bullish),
        axis(0.5, 0.0, AgreementCode::Bullish),
        1,
        AgreementCode::Bullish,
        0,
    );
    assert_eq!(
        decide(&strategy, &input, &conflict, StrategyPosition::flat()),
        Decision::Hold
    );

    let late = context(
        true,
        axis(1.0, 0.0, AgreementCode::Bullish),
        axis(0.5, 0.0, AgreementCode::Bullish),
        0,
        AgreementCode::Bullish,
        1,
    );
    assert_eq!(
        decide(&strategy, &input, &late, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_non_contiguous_execution_bar_blocks_an_entry() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(0).with_contiguous(false);
    let context = bullish_context();

    assert_eq!(
        decide(&strategy, &input, &context, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn an_exit_on_context_lost_precedes_structure_lost() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1).with_structure(Some(-1), Some(1));
    let context = context(
        true,
        axis(0.0, 0.0, AgreementCode::Mixed),
        axis(0.5, 0.0, AgreementCode::Bullish),
        0,
        AgreementCode::Bullish,
        0,
    );

    assert_eq!(
        decide(
            &strategy,
            &input,
            &context,
            StrategyPosition::opened(Side::Long, 0)
        ),
        Decision::Exit(Exit {
            side: Side::Long,
            bar: input,
            reason: ExitReason::ContextLost,
        })
    );
}

#[test]
fn a_missing_context_holds_an_open_position() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1).with_structure(Some(1), Some(0));
    let context = context(
        false,
        axis(1.0, 0.0, AgreementCode::Bullish),
        axis(0.5, 0.0, AgreementCode::Bullish),
        0,
        AgreementCode::Bullish,
        0,
    );

    assert_eq!(
        decide(
            &strategy,
            &input,
            &context,
            StrategyPosition::opened(Side::Long, 0)
        ),
        Decision::Hold
    );
}

#[test]
fn an_unavailable_context_share_holds_an_open_position() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1).with_structure(Some(1), Some(0));
    let mut context = bullish_context();
    context.direction.bullish_share = None;
    context.direction.bearish_share = None;

    assert_eq!(
        decide(
            &strategy,
            &input,
            &context,
            StrategyPosition::opened(Side::Long, 0)
        ),
        Decision::Hold
    );
}

#[test]
fn an_unavailable_progression_holds_an_open_position() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1);
    let mut context = bullish_context();
    context.progression.direction = AxisContext::unavailable();

    assert_eq!(
        decide(
            &strategy,
            &input,
            &context,
            StrategyPosition::opened(Side::Long, 0)
        ),
        Decision::Hold
    );
}

#[test]
fn a_measured_mixed_progression_still_exits_on_context_lost() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1);
    let mut context = bullish_context();
    context.progression.direction = axis(0.4, 0.4, AgreementCode::Mixed);

    assert_eq!(
        decide(
            &strategy,
            &input,
            &context,
            StrategyPosition::opened(Side::Long, 0)
        ),
        Decision::Exit(Exit {
            side: Side::Long,
            bar: input,
            reason: ExitReason::ContextLost,
        })
    );
}

#[test]
fn a_context_from_another_bar_is_not_used() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1);
    let context = bullish_context();

    assert_eq!(
        strategy.decide(&input, &context, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn the_structure_score_does_not_change_the_decision() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(0);
    let mut first = bullish_context();
    let mut second = bullish_context();
    first.structure.score = axis(0.0, 0.0, AgreementCode::Mixed);
    second.structure.score = axis(1.0, 0.0, AgreementCode::Bullish);

    assert_eq!(
        decide(&strategy, &input, &first, StrategyPosition::flat()),
        decide(&strategy, &input, &second, StrategyPosition::flat())
    );
}

#[test]
fn an_exit_on_structure_lost_leaves_a_valid_context_alone() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1).with_structure(Some(-1), Some(1));
    let context = bullish_context();

    assert_eq!(
        decide(
            &strategy,
            &input,
            &context,
            StrategyPosition::opened(Side::Long, 0)
        ),
        Decision::Exit(Exit {
            side: Side::Long,
            bar: input,
            reason: ExitReason::StructureLost,
        })
    );
}

#[test]
fn an_unavailable_structure_is_not_a_structure_loss() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = long_input(1).with_structure(None, Some(1));
    let context = bullish_context();

    assert_eq!(
        decide(
            &strategy,
            &input,
            &context,
            StrategyPosition::opened(Side::Long, 0)
        ),
        Decision::Hold
    );
}

#[test]
fn the_rule_never_reverses_on_the_exit_bar() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = short_input(1).with_structure(Some(-1), Some(-1));
    let context = bullish_context();

    let decision = decide(
        &strategy,
        &input,
        &context,
        StrategyPosition::opened(Side::Short, 0),
    );
    assert!(matches!(decision, Decision::Exit(_)));
}

#[test]
fn a_bar_from_another_timeframe_is_ignored() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let input = StrategyInput::new(900, 0, 0, Some(0.2), Some(0.2), Some(0.0))
        .with_structure(Some(1), Some(0));
    let context = bullish_context();

    assert_eq!(
        decide(&strategy, &input, &context, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn decisions_are_deterministic_across_repeated_runs() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let inputs = [long_input(0), short_input(1), long_input(2)];
    let contexts = [bullish_context(), bearish_context(), bullish_context()];
    let run = || {
        let mut position = StrategyPosition::flat();
        let mut decisions = Vec::new();
        for (input, context) in inputs.iter().zip(&contexts) {
            if position.is_open() {
                position.bars_held += 1;
            }
            let decision = strategy.decide(input, context, position);
            match decision {
                Decision::Enter(entry) => {
                    position = StrategyPosition::opened(entry.intent.side, input.bar_index);
                }
                Decision::Exit(_) => position = StrategyPosition::flat(),
                Decision::Hold => {}
            }
            decisions.push(decision);
        }
        decisions
    };
    let expected = run();
    for _ in 0..10 {
        assert_eq!(run(), expected);
    }
}

#[test]
fn later_bars_do_not_change_earlier_decisions() {
    let strategy = LadderContextStrategy::new(StrategyConfig::default());
    let inputs = [long_input(0), short_input(1), long_input(2)];
    let contexts = [bullish_context(), bearish_context(), bullish_context()];
    let run = |inputs: &[StrategyInput], contexts: &[LadderContext]| {
        let mut position = StrategyPosition::flat();
        inputs
            .iter()
            .zip(contexts)
            .map(|(input, context)| {
                let decision = strategy.decide(input, context, position);
                match decision {
                    Decision::Enter(entry) => {
                        position = StrategyPosition::opened(entry.intent.side, input.bar_index);
                    }
                    Decision::Exit(_) => position = StrategyPosition::flat(),
                    Decision::Hold => {}
                }
                decision
            })
            .collect::<Vec<_>>()
    };

    let prefix = run(&inputs[..1], &contexts[..1]);
    let full = run(&inputs, &contexts);
    assert_eq!(full[..1], prefix);

    let mut changed = inputs;
    changed[2] = short_input(2);
    assert_eq!(run(&changed[..1], &contexts[..1]), prefix);
}

#[test]
fn entry_configuration_rejects_out_of_range_values() {
    assert_eq!(
        ExecutionEntryConfig::new(-0.1, 0.1, 0.0, 0, 1),
        Err(StrategyConfigError::DirectionMin)
    );
    assert_eq!(
        ExecutionEntryConfig::new(0.1, 1.1, 0.0, 0, 1),
        Err(StrategyConfigError::MomentumMin)
    );
    assert_eq!(
        ExecutionEntryConfig::new(0.1, 0.1, 1.1, 0, 1),
        Err(StrategyConfigError::StructureMin)
    );
    assert_eq!(
        ExecutionEntryConfig::new(0.1, 0.1, 0.0, 3, 4),
        Err(StrategyConfigError::StructureShiftCode)
    );
    assert_eq!(
        ExecutionEntryConfig::new(0.1, 0.1, 0.0, 1, 1),
        Err(StrategyConfigError::StructureShiftOrder)
    );
    assert_eq!(
        ContextEntryConfig::new(1.1, 0.5, 0, 0),
        Err(StrategyConfigError::DirectionShareMin)
    );
    assert_eq!(
        ContextEntryConfig::new(0.5, -0.1, 0, 0),
        Err(StrategyConfigError::MomentumBearishShareMax)
    );
    assert_eq!(
        ExitConfig::new(false, false),
        Err(StrategyConfigError::EmptyExitReasons)
    );
}

#[test]
fn a_weight_count_mismatch_is_rejected() {
    let ladder = LadderConfig::new(300, vec![900, 3_600]).expect("the ladder is valid");
    let result = StrategyConfig::new(
        ladder,
        LadderWeights::equal(1),
        EntryConfig::default(),
        ExitConfig::default(),
        RiskPolicy::default(),
        true,
    );
    assert_eq!(
        result,
        Err(StrategyConfigError::WeightsLength {
            expected: 2,
            actual: 1,
        })
    );
}

#[test]
fn the_configuration_carries_an_explicit_risk_policy() {
    let risk = RiskPolicy::new(3.0, 2.0).expect("the risk policy is valid");
    let config = StrategyConfig::new(
        LadderConfig::default(),
        LadderWeights::equal(1),
        EntryConfig::default(),
        ExitConfig::default(),
        risk,
        true,
    )
    .expect("the config is valid");
    let strategy = LadderContextStrategy::new(config);

    assert_eq!(
        strategy
            .config()
            .risk()
            .expect("the custom config has a risk policy")
            .stop_atr_multiple(),
        3.0
    );
    assert_eq!(
        strategy
            .config()
            .risk()
            .expect("the custom config has a risk policy")
            .reward_to_risk(),
        2.0
    );
}

fn level_context() -> LadderContext {
    let mut context = bullish_context();
    context.levels = LevelContext {
        reference_price: Some(100.0),
        support: Some(LevelView {
            timeframe_secs: 900,
            price: 99.0,
            occurred_at: 0,
            available_at: 0,
            scale_code: 2,
            kind_code: -1,
        }),
        resistance: Some(LevelView {
            timeframe_secs: 900,
            price: 104.0,
            occurred_at: 0,
            available_at: 0,
            scale_code: 2,
            kind_code: 1,
        }),
        support_distance_fraction: Some(0.01),
        resistance_distance_fraction: Some(0.04),
        retest_support: true,
        retest_resistance: false,
        support_broken: false,
        resistance_broken: false,
        ..LevelContext::unavailable()
    };
    context
}

fn level_config() -> StrategyConfig {
    StrategyConfig::new_level_retest(
        LadderConfig::default(),
        LadderWeights::equal(1),
        EntryConfig::default(),
        ExitConfig::new_level(true, false, true).expect("the exit config is valid"),
        LevelRetestConfig::new(0.02, 0.002, 2.0, None).expect("the level config is valid"),
        true,
    )
    .expect("the strategy config is valid")
}

#[test]
fn a_level_retest_enters_with_a_structural_bracket() {
    let strategy = LadderContextStrategy::new(level_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0));
    let decision = strategy.decide(&bar, &level_context(), StrategyPosition::flat());
    let Decision::Enter(entry) = decision else {
        panic!("the retest must enter");
    };
    assert_eq!(entry.intent.side, Side::Long);
    match entry.intent.stop {
        StopSpec::Price { price } => assert!((price - 98.802).abs() < 1e-12),
        other => panic!("expected a price stop, got {other:?}"),
    }
    match entry.intent.target {
        TargetSpec::Price { price } => assert!((price - 104.0).abs() < 1e-12),
        other => panic!("expected a price target, got {other:?}"),
    }
}

fn breakout_context() -> LadderContext {
    let mut context = level_context();
    context.levels.reference_price = Some(102.0);
    context.levels.resistance = Some(LevelView {
        timeframe_secs: 900,
        price: 106.0,
        occurred_at: 0,
        available_at: 0,
        scale_code: 2,
        kind_code: 1,
    });
    context.levels.broken_resistance = Some(LevelView {
        timeframe_secs: 900,
        price: 101.0,
        occurred_at: 0,
        available_at: 0,
        scale_code: 2,
        kind_code: 1,
    });
    context.levels.resistance_distance_fraction = Some(4.0 / 102.0);
    context
}

fn breakout_config() -> StrategyConfig {
    StrategyConfig::new_breakout(
        LadderConfig::default(),
        LadderWeights::equal(1),
        EntryConfig::default(),
        ExitConfig::new_level(true, false, true).expect("the exit config is valid"),
        LevelRetestConfig::new(0.02, 0.002, 1.5, None).expect("the breakout config is valid"),
        true,
    )
    .expect("the strategy config is valid")
}

#[test]
fn a_confirmed_level_breakout_enters_with_a_structural_bracket() {
    let strategy = LadderContextStrategy::new(breakout_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0));
    let Decision::Enter(entry) =
        strategy.decide(&bar, &breakout_context(), StrategyPosition::flat())
    else {
        panic!("the breakout must enter");
    };
    assert_eq!(entry.intent.side, Side::Long);
    match entry.intent.stop {
        StopSpec::Price { price } => assert!((price - 100.798).abs() < 1e-12),
        other => panic!("expected a price stop, got {other:?}"),
    }
    match entry.intent.target {
        TargetSpec::Price { price } => assert!((price - 106.0).abs() < 1e-12),
        other => panic!("expected a price target, got {other:?}"),
    }
}

#[test]
fn a_breakout_without_a_crossed_level_holds() {
    let strategy = LadderContextStrategy::new(breakout_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0));
    let mut context = breakout_context();
    context.levels.broken_resistance = None;
    assert_eq!(
        strategy.decide(&bar, &context, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_confirmed_level_break_closes_a_level_retest_position() {
    let strategy = LadderContextStrategy::new(level_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0));
    let mut context = level_context();
    context.levels.broken_support = context.levels.support;
    let Decision::Exit(exit) =
        strategy.decide(&bar, &context, StrategyPosition::with_side(Side::Long))
    else {
        panic!("the broken support must exit");
    };
    assert_eq!(exit.reason, ExitReason::LevelLost);
}

#[test]
fn a_level_retest_does_not_require_a_structure_shift() {
    let strategy = LadderContextStrategy::new(level_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0))
        .with_structure(Some(1), Some(1));
    assert!(matches!(
        strategy.decide(&bar, &level_context(), StrategyPosition::flat()),
        Decision::Enter(_)
    ));
}

fn bearish_level_context() -> LadderContext {
    let mut context = bearish_context();
    context.levels = LevelContext {
        reference_price: Some(100.0),
        support: Some(LevelView {
            timeframe_secs: 900,
            price: 96.0,
            occurred_at: 0,
            available_at: 0,
            scale_code: 2,
            kind_code: -1,
        }),
        resistance: Some(LevelView {
            timeframe_secs: 900,
            price: 101.0,
            occurred_at: 0,
            available_at: 0,
            scale_code: 2,
            kind_code: 1,
        }),
        support_distance_fraction: Some(0.04),
        resistance_distance_fraction: Some(0.01),
        retest_support: false,
        retest_resistance: true,
        support_broken: false,
        resistance_broken: false,
        ..LevelContext::unavailable()
    };
    context
}

#[test]
fn a_short_level_retest_mirrors_the_long_bracket() {
    let strategy = LadderContextStrategy::new(level_config());
    let bar = StrategyInput::new(300, 0, 0, Some(-0.4), Some(-0.2), Some(-1.0));
    let Decision::Enter(entry) =
        strategy.decide(&bar, &bearish_level_context(), StrategyPosition::flat())
    else {
        panic!("the short retest must enter");
    };
    assert_eq!(entry.intent.side, Side::Short);
    match entry.intent.stop {
        StopSpec::Price { price } => assert!((price - 101.202).abs() < 1e-12),
        other => panic!("expected a price stop, got {other:?}"),
    }
    match entry.intent.target {
        TargetSpec::Price { price } => assert!((price - 96.0).abs() < 1e-12),
        other => panic!("expected a price target, got {other:?}"),
    }
}

#[test]
fn an_exact_touch_is_a_valid_retest_but_a_missing_level_is_not() {
    let strategy = LadderContextStrategy::new(level_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0));

    let mut exact = level_context();
    exact.levels.support = Some(LevelView {
        timeframe_secs: 900,
        price: 100.0,
        occurred_at: 0,
        available_at: 0,
        scale_code: 2,
        kind_code: -1,
    });
    exact.levels.support_distance_fraction = Some(0.0);
    exact.levels.retest_support = true;
    assert!(matches!(
        strategy.decide(&bar, &exact, StrategyPosition::flat()),
        Decision::Enter(_)
    ));

    let mut missing = level_context();
    missing.levels = LevelContext::unavailable();
    assert_eq!(
        strategy.decide(&bar, &missing, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_failed_retest_or_a_thin_reward_does_not_enter() {
    let strategy = LadderContextStrategy::new(level_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0));

    let mut no_retest = level_context();
    no_retest.levels.retest_support = false;
    assert_eq!(
        strategy.decide(&bar, &no_retest, StrategyPosition::flat()),
        Decision::Hold
    );

    let mut thin = level_context();
    thin.levels.resistance = Some(LevelView {
        timeframe_secs: 900,
        price: 100.5,
        occurred_at: 0,
        available_at: 0,
        scale_code: 2,
        kind_code: 1,
    });
    thin.levels.resistance_distance_fraction = Some(0.005);
    assert_eq!(
        strategy.decide(&bar, &thin, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_broken_level_exits_a_long_position() {
    let strategy = LadderContextStrategy::new(level_config());
    let bar = StrategyInput::new(300, 0, 0, Some(0.2), Some(0.1), Some(0.0));
    let mut context = level_context();
    context.levels.support_broken = true;
    let decision = strategy.decide(&bar, &context, StrategyPosition::opened(Side::Long, 0));
    assert_eq!(
        decision,
        Decision::Exit(Exit {
            side: Side::Long,
            bar,
            reason: ExitReason::LevelLost,
        })
    );
}

fn three_timeframe_config() -> StrategyConfig {
    StrategyConfig::new_three_timeframe(
        LadderConfig::new(60, vec![300, 900]).expect("the ladder is valid"),
        LadderWeights::equal(2),
        EntryConfig::default(),
        ExitConfig::new_level(true, false, true).expect("the exit config is valid"),
        ThreeTimeframeConfig::new(
            LevelRetestConfig::new(
                0.01,
                0.005,
                2.0,
                Some(quantrule_model::readings::PivotScale::Major),
            )
            .expect("the level config is valid"),
            15,
        )
        .expect("the agreement config is valid"),
        true,
    )
    .expect("the strategy config is valid")
}

fn three_timeframe_context(side: Side, age: u64) -> LadderContext {
    let mut context = if side == Side::Long {
        level_context()
    } else {
        bearish_level_context()
    };
    context.execution_timeframe_secs = 60;
    context.execution_close_time = 60;
    context.agreement = AgreementContext {
        long: side == Side::Long,
        short: side == Side::Short,
        long_bars: if side == Side::Long { age } else { 0 },
        short_bars: if side == Side::Short { age } else { 0 },
    };
    context
}

#[test]
fn a_three_timeframe_long_enters_after_the_agreement_age() {
    let strategy = LadderContextStrategy::new(three_timeframe_config());
    let bar = StrategyInput::new(60, 0, 0, Some(0.4), Some(0.2), Some(1.0))
        .with_structure(Some(1), Some(0));
    let Decision::Enter(entry) = strategy.decide(
        &bar,
        &three_timeframe_context(Side::Long, 15),
        StrategyPosition::flat(),
    ) else {
        panic!("the agreement and 1m shift must enter");
    };
    assert_eq!(entry.intent.side, Side::Long);
    match entry.intent.stop {
        StopSpec::Price { price } => assert!((price - 98.505).abs() < 1e-12),
        other => panic!("expected a price stop, got {other:?}"),
    }
    match entry.intent.target {
        TargetSpec::Price { price } => assert!((price - 104.0).abs() < 1e-12),
        other => panic!("expected a price target, got {other:?}"),
    }
}

#[test]
fn a_three_timeframe_entry_waits_for_the_age_and_the_retest() {
    let strategy = LadderContextStrategy::new(three_timeframe_config());
    let bar = StrategyInput::new(60, 0, 0, Some(0.4), Some(0.2), Some(1.0))
        .with_structure(Some(1), Some(0));
    assert_eq!(
        strategy.decide(
            &bar,
            &three_timeframe_context(Side::Long, 14),
            StrategyPosition::flat(),
        ),
        Decision::Hold
    );
    let no_retest_input = StrategyInput::new(60, 0, 0, Some(0.4), Some(0.2), Some(1.0));
    let mut no_retest = three_timeframe_context(Side::Long, 15);
    no_retest.levels.retest_support = false;
    assert_eq!(
        strategy.decide(&no_retest_input, &no_retest, StrategyPosition::flat(),),
        Decision::Hold
    );
}

#[test]
fn a_three_timeframe_short_mirrors_the_long() {
    let strategy = LadderContextStrategy::new(three_timeframe_config());
    let bar = StrategyInput::new(60, 0, 0, Some(-0.4), Some(-0.2), Some(-1.0))
        .with_structure(Some(-1), Some(0));
    let Decision::Enter(entry) = strategy.decide(
        &bar,
        &three_timeframe_context(Side::Short, 15),
        StrategyPosition::flat(),
    ) else {
        panic!("the short agreement and 1m shift must enter");
    };
    assert_eq!(entry.intent.side, Side::Short);
    match entry.intent.stop {
        StopSpec::Price { price } => assert!((price - 101.505).abs() < 1e-12),
        other => panic!("expected a price stop, got {other:?}"),
    }
    match entry.intent.target {
        TargetSpec::Price { price } => assert!((price - 96.0).abs() < 1e-12),
        other => panic!("expected a price target, got {other:?}"),
    }
}
