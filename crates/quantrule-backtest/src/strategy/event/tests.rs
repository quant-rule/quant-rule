use super::{Condition, EventCompositeConfig, EventCompositeStrategy, StateField};
use crate::intent::{RiskPolicy, StrategyPosition};
use crate::side::Side;
use crate::strategy::decision::{Decision, ExitReason};
use crate::strategy::event_input::{EventStrategyInput, EventTimeframeView};
use quantrule_model::timeframe::ModelStates;

#[test]
fn the_context_state_fields_parse_and_validate() {
    assert_eq!(
        StateField::from_code("vol_regime"),
        Some(StateField::VolRegime)
    );
    assert_eq!(
        StateField::from_code("structure_state"),
        Some(StateField::StructureState)
    );
    assert!(StateField::VolRegime.accepts(0));
    assert!(StateField::VolRegime.accepts(2));
    assert!(!StateField::VolRegime.accepts(-1));
    assert!(!StateField::VolRegime.accepts(3));
    assert!(StateField::StructureState.accepts(-1));
    assert!(StateField::StructureState.accepts(1));
    assert!(!StateField::StructureState.accepts(2));
}

#[test]
fn the_context_states_match_exact_codes() {
    let input = input();
    let vol = Condition::State {
        timeframe_secs: 900,
        field: StateField::VolRegime,
        value: 1,
    };
    let structure = Condition::State {
        timeframe_secs: 900,
        field: StateField::StructureState,
        value: 1,
    };
    assert!(vol.matches(&input));
    assert!(structure.matches(&input));
    assert!(
        !Condition::State {
            timeframe_secs: 900,
            field: StateField::VolRegime,
            value: 2,
        }
        .matches(&input)
    );
}

fn states() -> ModelStates {
    ModelStates {
        ema_stack: Some(1),
        rsi_zone: Some(0),
        macd_state: Some(1),
        macd_histogram_state: Some(1),
        adx_regime: Some(1),
        adx_slope: Some(1),
        di_state: Some(1),
        rvol_regime: Some(1),
        vol_regime: Some(1),
        structure_state: Some(1),
    }
}

fn input() -> EventStrategyInput {
    EventStrategyInput {
        execution_timeframe_secs: 900,
        execution_bar_index: 10,
        execution_open_time: 9_000,
        execution_contiguous: true,
        timeframes: vec![EventTimeframeView {
            timeframe_secs: 900,
            bar_index: 10,
            open_time: 9_000,
            states: states(),
            progression_stage: None,
            progression_leg_direction: None,
            events: Vec::new(),
            context: None,
        }],
    }
}

#[test]
fn a_state_only_entry_is_rejected() {
    let result = EventCompositeConfig::new(
        3,
        0,
        &[Condition::State {
            timeframe_secs: 900,
            field: StateField::EmaStack,
            value: 1,
        }],
        &[],
    );
    assert!(result.is_err());
}

#[test]
fn a_registered_signal_subtype_above_seven_is_accepted() {
    let result = EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 14,
            source: 19,
            direction: 1,
            contrarian: false,
        }],
        &[],
    );
    assert!(result.is_ok());
}

#[test]
fn the_new_structure_events_are_registered_strategy_events() {
    for (event_type, subtype, source) in [(1, 1, 56), (1, 1, 57), (3, 19, 55)] {
        let result = EventCompositeConfig::new(
            3,
            0,
            &[Condition::Event {
                timeframe_secs: 900,
                event_type,
                subtype,
                source,
                direction: 1,
                contrarian: false,
            }],
            &[],
        );
        assert!(
            result.is_ok(),
            "event ({event_type}, {subtype}, {source}) should be accepted"
        );
    }
}

#[test]
fn a_registered_diagnostic_subtype_is_rejected() {
    let result = EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 21,
            source: 41,
            direction: 1,
            contrarian: false,
        }],
        &[],
    );
    assert_eq!(
        result,
        Err(crate::strategy::StrategyConfigError::EventDiagnostic)
    );
}

#[test]
fn an_unregistered_event_condition_is_rejected() {
    let result = EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 1,
            source: 2,
            direction: 1,
            contrarian: false,
        }],
        &[],
    );
    assert_eq!(
        result,
        Err(crate::strategy::StrategyConfigError::EventNotRegistered)
    );
}

#[test]
fn a_diagnostic_event_in_the_input_cannot_satisfy_the_rule() {
    let config = EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 1,
            source: 4,
            direction: 1,
            contrarian: false,
        }],
        &[],
    )
    .expect("the config is valid");
    let mut input = input();
    input.timeframes[0].events = vec![crate::strategy::event_input::EventView {
        event_code: 120,
        event_type: 3,
        subtype: 21,
        source: 41,
        direction: 1,
        occurred_bar_index: 10,
        available_bar_index: 10,
        reference_code: None,
        primary_value: Some(0.55),
        reference_value: Some(0.5),
    }];
    let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
    assert_eq!(
        strategy.decide(&input, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_directional_event_entry_opens_the_side() {
    let config = EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 6,
            source: 16,
            direction: 1,
            contrarian: false,
        }],
        &[],
    )
    .expect("the config is valid");
    let mut input = input();
    input.timeframes[0].events = vec![crate::strategy::event_input::EventView {
        event_code: 0,
        event_type: 3,
        subtype: 6,
        source: 16,
        direction: 1,
        occurred_bar_index: 10,
        available_bar_index: 10,
        reference_code: None,
        primary_value: None,
        reference_value: None,
    }];
    let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
    let Decision::Enter(entry) = strategy.decide(&input, StrategyPosition::flat()) else {
        panic!("the entry must fire");
    };
    assert_eq!(entry.side(), Side::Long);
}

#[test]
fn a_contrarian_event_entry_opens_the_opposite_side() {
    for (event_direction, expected) in [(1, Side::Short), (-1, Side::Long)] {
        let config = EventCompositeConfig::new(
            3,
            0,
            &[Condition::Event {
                timeframe_secs: 900,
                event_type: 3,
                subtype: 6,
                source: 16,
                direction: 0,
                contrarian: true,
            }],
            &[],
        )
        .expect("the config is valid");
        let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
        let mut input = input();
        input.timeframes[0].events = vec![event(event_direction)];
        let Decision::Enter(entry) = strategy.decide(&input, StrategyPosition::flat()) else {
            panic!("the contrarian entry must fire");
        };
        assert_eq!(entry.side(), expected);
    }
}

#[test]
fn a_conflicting_entry_is_rejected() {
    let result = EventCompositeConfig::new(
        3,
        0,
        &[
            Condition::Event {
                timeframe_secs: 900,
                event_type: 3,
                subtype: 6,
                source: 16,
                direction: 1,
                contrarian: false,
            },
            Condition::Event {
                timeframe_secs: 900,
                event_type: 3,
                subtype: 2,
                source: 17,
                direction: -1,
                contrarian: false,
            },
        ],
        &[],
    );
    assert!(result.is_err());
}

#[test]
fn an_exit_condition_closes_the_position() {
    let config = EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 6,
            source: 16,
            direction: 1,
            contrarian: false,
        }],
        &[Condition::State {
            timeframe_secs: 900,
            field: StateField::EmaStack,
            value: -1,
        }],
    )
    .expect("the config is valid");
    let mut input = input();
    input.timeframes[0].states.ema_stack = Some(-1);
    let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
    let Decision::Exit(exit) = strategy.decide(&input, StrategyPosition::with_side(Side::Long))
    else {
        panic!("the exit must fire");
    };
    assert_eq!(
        exit.reason,
        crate::strategy::decision::ExitReason::EventCondition
    );
}

fn event(direction: i8) -> crate::strategy::event_input::EventView {
    crate::strategy::event_input::EventView {
        event_code: 0,
        event_type: 3,
        subtype: 6,
        source: 16,
        direction,
        occurred_bar_index: 10,
        available_bar_index: 10,
        reference_code: None,
        primary_value: None,
        reference_value: None,
    }
}

fn zero_direction_config() -> EventCompositeConfig {
    EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 6,
            source: 16,
            direction: 0,
            contrarian: false,
        }],
        &[],
    )
    .expect("the config is valid")
}

#[test]
fn a_zero_direction_event_opens_the_matched_side() {
    let strategy =
        EventCompositeStrategy::new(zero_direction_config(), RiskPolicy::default(), true);
    for (direction, side) in [(-1, Side::Short), (1, Side::Long)] {
        let mut input = input();
        input.timeframes[0].events = vec![event(direction)];
        let Decision::Enter(entry) = strategy.decide(&input, StrategyPosition::flat()) else {
            panic!("the entry must fire for direction {direction}");
        };
        assert_eq!(entry.side(), side);
    }
}

#[test]
fn a_zero_direction_event_with_opposite_matches_is_ambiguous() {
    let strategy =
        EventCompositeStrategy::new(zero_direction_config(), RiskPolicy::default(), true);
    let mut input = input();
    input.timeframes[0].events = vec![event(1), event(-1)];
    assert_eq!(
        strategy.decide(&input, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn an_exact_direction_ignores_the_other_side() {
    let config = EventCompositeConfig::new(
        3,
        0,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 6,
            source: 16,
            direction: 1,
            contrarian: false,
        }],
        &[],
    )
    .expect("the config is valid");
    let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
    let mut input = input();
    input.timeframes[0].events = vec![event(-1)];
    assert_eq!(
        strategy.decide(&input, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_neutral_event_cannot_choose_a_side() {
    let strategy =
        EventCompositeStrategy::new(zero_direction_config(), RiskPolicy::default(), true);
    let mut input = input();
    input.timeframes[0].events = vec![event(0)];
    assert_eq!(
        strategy.decide(&input, StrategyPosition::flat()),
        Decision::Hold
    );
}

#[test]
fn a_time_stop_closes_the_position() {
    let config = EventCompositeConfig::new(
        3,
        3,
        &[Condition::Event {
            timeframe_secs: 900,
            event_type: 3,
            subtype: 6,
            source: 16,
            direction: 0,
            contrarian: false,
        }],
        &[],
    )
    .expect("the config is valid");
    let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
    let mut position = StrategyPosition::with_side(Side::Long);
    position.bars_held = 3;
    let Decision::Exit(exit) = strategy.decide(&input(), position) else {
        panic!("the time stop must fire");
    };
    assert_eq!(exit.reason, crate::strategy::decision::ExitReason::TimeStop);
}

#[test]
fn a_zero_time_stop_never_times_out() {
    let strategy =
        EventCompositeStrategy::new(zero_direction_config(), RiskPolicy::default(), true);
    let mut position = StrategyPosition::with_side(Side::Long);
    position.bars_held = 10_000;
    assert_eq!(strategy.decide(&input(), position), Decision::Hold);
}

#[test]
fn a_giveback_exit_fires_after_the_arm_and_threshold() {
    let config = zero_direction_config()
        .with_giveback_atr(1.0, 0.5)
        .expect("the giveback config is valid");
    let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
    let position =
        StrategyPosition::opened(Side::Long, 0).with_excursions(100.0, 1.0, 0.5, 2.0, 0.2);
    let Decision::Exit(exit) = strategy.decide(&input(), position) else {
        panic!("the giveback exit must fire");
    };
    assert_eq!(exit.reason, ExitReason::Giveback);
}

#[test]
fn a_giveback_exit_waits_for_the_arm() {
    let config = zero_direction_config()
        .with_giveback_atr(1.0, 0.5)
        .expect("the giveback config is valid");
    let strategy = EventCompositeStrategy::new(config, RiskPolicy::default(), true);
    let position =
        StrategyPosition::opened(Side::Long, 0).with_excursions(100.0, 1.0, 0.0, 0.4, 0.2);
    assert_eq!(strategy.decide(&input(), position), Decision::Hold);
}

#[test]
fn an_invalid_giveback_config_is_rejected() {
    assert!(zero_direction_config().with_giveback_atr(0.0, 0.0).is_err());
    assert!(
        zero_direction_config()
            .with_giveback_atr(1.0, -1.0)
            .is_err()
    );
}
