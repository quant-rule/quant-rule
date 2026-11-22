//! The evidence a strategy decision carries.
//!
//! A decision's evidence is the published event and state snapshot that
//! satisfied the rule. The strategy decision record and the signal artifact both
//! publish this evidence, so the rule semantics are built once here. The module
//! reads no file, clock, or random source.
//!
//! Evidence is numeric and registry-based: an event is named by its type,
//! subtype, source, direction, and availability; a state is named by its field,
//! value, timeframe, and bar. The shell maps the numbers to words.

use quantrule_backtest::strategy::{
    Condition, Decision, EventStrategyInput, EventTimeframeView, EventView, ExitReason, StateField,
};
use serde_json::{Value, json};

use crate::config::StrategyConfig;

/// Returns the evidence for one decision.
///
/// An entry carries its entry-condition evidence, an event-condition exit its
/// exit-condition evidence, and a bracket entry the compression-range event that
/// placed it. A time-stop or giveback exit has no condition evidence.
#[must_use]
pub(crate) fn decision_evidence(
    config: &StrategyConfig,
    input: &EventStrategyInput,
    decision: Decision,
) -> Vec<Value> {
    match decision {
        Decision::Enter(_) => config
            .entry_conditions()
            .iter()
            .flat_map(|condition| condition_evidence(*condition, input))
            .collect(),
        Decision::Exit(exit)
            if matches!(exit.reason, ExitReason::TimeStop | ExitReason::Giveback) =>
        {
            Vec::new()
        }
        Decision::Exit(_) => config
            .exit_conditions()
            .iter()
            .flat_map(|condition| condition_evidence(*condition, input))
            .collect(),
        Decision::EnterBracket(_) => config
            .compression_event()
            .map(|(timeframe, event_type, subtype, source)| {
                input
                    .timeframe(timeframe)
                    .map(|view| {
                        view.events
                            .iter()
                            .filter(|event| {
                                event.event_type == event_type
                                    && event.subtype == subtype
                                    && event.source == source
                                    && event.direction == 0
                                    && event.available_bar_index == input.execution_bar_index
                            })
                            .map(event_evidence)
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default(),
        Decision::Hold => Vec::new(),
    }
}

fn condition_matches(condition: Condition, input: &EventStrategyInput) -> bool {
    match condition {
        Condition::Event {
            timeframe_secs,
            event_type,
            subtype,
            source,
            direction,
            contrarian: _,
        } => input
            .timeframe(u64::from(timeframe_secs))
            .is_some_and(|view| {
                view.events.iter().any(|event| {
                    event.event_type == event_type
                        && event.subtype == subtype
                        && event.source == u16::from(source)
                        && (direction == 0 || event.direction == direction)
                })
            }),
        Condition::State {
            timeframe_secs,
            field,
            value,
        } => input
            .timeframe(u64::from(timeframe_secs))
            .is_some_and(|view| observed_state(view, field) == Some(value)),
    }
}

fn condition_evidence(condition: Condition, input: &EventStrategyInput) -> Vec<Value> {
    if !condition_matches(condition, input) {
        return Vec::new();
    }
    match condition {
        Condition::Event {
            timeframe_secs,
            event_type,
            subtype,
            source,
            direction,
            contrarian: _,
        } => input
            .timeframe(u64::from(timeframe_secs))
            .map(|view| {
                view.events
                    .iter()
                    .filter(|event| {
                        event.event_type == event_type
                            && event.subtype == subtype
                            && event.source == u16::from(source)
                            && (direction == 0 || event.direction == direction)
                    })
                    .map(event_evidence)
                    .collect()
            })
            .unwrap_or_default(),
        Condition::State {
            timeframe_secs,
            field,
            ..
        } => input
            .timeframe(u64::from(timeframe_secs))
            .and_then(|view| {
                observed_state(view, field).map(|observed| {
                    json!({
                        "kind": "state",
                        "timeframe_secs": timeframe_secs,
                        "field": state_field_label(field),
                        "value": observed,
                        "bar_index": view.bar_index,
                    })
                })
            })
            .into_iter()
            .collect(),
    }
}

fn event_evidence(event: &EventView) -> Value {
    json!({
        "kind": "event",
        "event_code": event.event_code,
        "event_type": event.event_type,
        "subtype": event.subtype,
        "source": event.source,
        "occurred_bar": event.occurred_bar_index,
        "available_bar": event.available_bar_index,
        "direction": event.direction,
    })
}

fn observed_state(view: &EventTimeframeView, field: StateField) -> Option<i8> {
    match field {
        StateField::ProgressionStage => view.progression_stage,
        StateField::ProgressionLegDirection => view.progression_leg_direction,
        _ => state_value(view.states, field),
    }
}

fn state_value(states: quantrule_model::timeframe::ModelStates, field: StateField) -> Option<i8> {
    match field {
        StateField::EmaStack => states.ema_stack,
        StateField::RsiZone => states.rsi_zone,
        StateField::MacdState => states.macd_state,
        StateField::MacdHistogramState => states.macd_histogram_state,
        StateField::AdxRegime => states.adx_regime,
        StateField::AdxSlope => states.adx_slope,
        StateField::DiState => states.di_state,
        StateField::RvolRegime => states.rvol_regime,
        StateField::VolRegime => states.vol_regime,
        StateField::StructureState => states.structure_state,
        StateField::ProgressionStage | StateField::ProgressionLegDirection => None,
    }
}

fn state_field_label(field: StateField) -> &'static str {
    match field {
        StateField::EmaStack => "ema_stack",
        StateField::RsiZone => "rsi_zone",
        StateField::MacdState => "macd_state",
        StateField::MacdHistogramState => "macd_histogram_state",
        StateField::AdxRegime => "adx_regime",
        StateField::AdxSlope => "adx_slope",
        StateField::DiState => "di_state",
        StateField::RvolRegime => "rvol_regime",
        StateField::VolRegime => "vol_regime",
        StateField::StructureState => "structure_state",
        StateField::ProgressionStage => "progression_stage",
        StateField::ProgressionLegDirection => "progression_leg_direction",
    }
}

#[cfg(test)]
mod tests {
    use super::{decision_evidence, state_field_label};
    use crate::config::StrategyConfig;
    use quantrule_backtest::intent::StrategyPosition;
    use quantrule_backtest::strategy::{
        Decision, EventStrategyInput, EventTimeframeView, EventView, StateField,
    };
    use quantrule_model::timeframe::ModelStates;
    use serde_json::json;

    fn states() -> ModelStates {
        ModelStates {
            ema_stack: Some(1),
            rsi_zone: None,
            macd_state: None,
            macd_histogram_state: None,
            adx_regime: None,
            adx_slope: None,
            di_state: None,
            rvol_regime: None,
            vol_regime: None,
            structure_state: None,
        }
    }

    fn config() -> StrategyConfig {
        crate::config::parse_strategy(
            &json!({
                "schema": "quantrule.strategy/v1",
                "engine": "event_composite",
                "execution_timeframe": "5m",
                "event_window_bars": 1,
                "time_stop_bars": 0,
                "risk": { "stop_atr_multiple": 2.0, "reward_to_risk": 1.5 },
                "entry": [
                    { "kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1, "source": 4, "direction": 1 }
                ],
                "exit": [
                    { "kind": "state", "timeframe": "5m", "field": "ema_stack", "value": 1 }
                ]
            }),
            "strategy.json",
        )
        .expect("the strategy is valid")
    }

    fn event() -> EventView {
        EventView {
            event_code: 1,
            event_type: 3,
            subtype: 1,
            source: 4,
            direction: 1,
            occurred_bar_index: 2,
            available_bar_index: 2,
            reference_code: None,
            primary_value: Some(100.0),
            reference_value: None,
        }
    }

    fn input() -> EventStrategyInput {
        EventStrategyInput {
            execution_timeframe_secs: 300,
            execution_bar_index: 2,
            execution_open_time: 600,
            execution_contiguous: true,
            timeframes: vec![EventTimeframeView {
                timeframe_secs: 300,
                bar_index: 2,
                open_time: 600,
                states: states(),
                progression_stage: None,
                progression_leg_direction: None,
                events: vec![event()],
            }],
        }
    }

    #[test]
    fn an_entry_carries_its_matching_event() {
        let config = config();
        let input = input();
        let decision = config.decide(&input, StrategyPosition::flat());
        let evidence = decision_evidence(&config, &input, decision);
        assert_eq!(
            evidence,
            vec![json!({
                "kind": "event",
                "event_code": 1,
                "event_type": 3,
                "subtype": 1,
                "source": 4,
                "occurred_bar": 2,
                "available_bar": 2,
                "direction": 1,
            })]
        );
    }

    #[test]
    fn an_exit_carries_the_state_that_matched() {
        let config = config();
        let input = input();
        let decision = config.decide(
            &input,
            StrategyPosition::with_side(quantrule_backtest::side::Side::Long),
        );
        let evidence = decision_evidence(&config, &input, decision);
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0]["kind"], json!("state"));
        assert_eq!(evidence[0]["field"], json!("ema_stack"));
        assert_eq!(evidence[0]["value"], json!(1));
    }

    #[test]
    fn a_hold_has_no_evidence_and_state_labels_are_stable() {
        assert!(
            decision_evidence(&config(), &input(), Decision::Hold).is_empty(),
            "a hold has no evidence"
        );
        assert_eq!(state_field_label(StateField::RsiZone), "rsi_zone");
    }
}
