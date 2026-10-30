//! Tests for the unified ladder-context strategy and its configuration.

use crate::context::{
    AgreementCode, AxisContext, LadderContext, ProgressionContext, StructureContext,
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
    strategy.decide(input, context, position)
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
    assert_eq!(config.risk().stop_atr_multiple(), 2.0);
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

    assert_eq!(strategy.config().risk().stop_atr_multiple(), 3.0);
    assert_eq!(strategy.config().risk().reward_to_risk(), 2.0);
}
