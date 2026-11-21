//! Primitive tests for the prediction evaluator.

use super::*;

fn row(close: f64, high: f64, low: f64, atr: f64) -> Row {
    Row {
        timeframe: 300,
        bar: 0,
        timestamp: Some(1_700_000_000),
        close,
        high,
        low,
        atr: Some(atr),
        states: BTreeMap::new(),
    }
}

fn args(outcome: &str) -> Args {
    Args {
        bundles: Vec::new(),
        instrument: "unknown".to_owned(),
        spec_id: None,
        family: None,
        partition: "model_dev".to_owned(),
        timeframe: None,
        split_start: None,
        split_end: None,
        horizon: 2,
        outcome: outcome.to_owned(),
        target_atr: 1.0,
        stop_atr: 1.0,
        cost_bps: 0.0,
        random_draws: 0,
        shuffle_draws: 0,
        back_lag: None,
        seed: 0,
        primary_cells: 1,
        alpha: 0.05,
        min_events: 0,
        min_clusters: 0,
        min_lift: 0.0,
        min_instrument_events: 0,
        min_instruments: 1,
        state_file: None,
        min_state_increment: 0.0,
    }
}

#[test]
fn stop_wins_the_same_bar_tie() {
    let rows = vec![
        row(100.0, 100.5, 99.5, 1.0),
        row(99.0, 101.5, 98.5, 1.0),
        row(100.0, 100.5, 99.5, 1.0),
    ];
    let outcome = barrier_outcome(&rows, 0, 1, 2, 1.0, 1.0, 0.0).expect("outcome");
    assert!(!outcome.hit);
}

#[test]
fn a_target_before_the_stop_is_a_hit() {
    let rows = vec![
        row(100.0, 100.5, 99.5, 1.0),
        row(101.0, 101.5, 99.9, 1.0),
        row(100.0, 100.5, 99.5, 1.0),
    ];
    let outcome = barrier_outcome(&rows, 0, 1, 2, 1.0, 1.0, 0.0).expect("outcome");
    assert!(outcome.hit);
}

#[test]
fn a_forward_cost_can_turn_a_gain_negative() {
    let rows = vec![
        row(100.0, 100.5, 99.5, 1.0),
        row(100.0, 100.5, 99.5, 1.0),
        row(100.5, 101.0, 100.0, 1.0),
    ];
    let outcome = forward_outcome(&rows, 0, 1, 2, 100.0).expect("outcome");
    assert!(!outcome.hit);
}

#[test]
fn a_state_clause_requires_a_published_value() {
    let gate = StateGate {
        clauses: vec![StateClause {
            column: "adx_regime".to_owned(),
            values: Some(vec![1]),
            min_value: None,
            max_value: None,
        }],
        raw: serde_json::json!({}),
    };
    let mut value = row(100.0, 100.5, 99.5, 1.0);
    value.states.insert("adx_regime".to_owned(), "1".to_owned());
    assert!(state_matches(&value, &gate));
    value.states.remove("adx_regime");
    assert!(!state_matches(&value, &gate));
}

#[test]
fn the_args_helper_is_usable() {
    assert_eq!(args("barrier").horizon, 2);
}
