//! Unit tests for the signal artifact and reference position.
//!
//! The fixtures publish small model bundles directly, so the reference position
//! is exercised without a filesystem. The public integration test loads the same
//! shapes from a bundle directory.

use std::collections::BTreeMap;

use quantrule_api::Digest;
use quantrule_backtest::strategy::EventView;
use quantrule_data::Candle;
use quantrule_model::timeframe::ModelStates;
use serde_json::json;

use crate::pack::StrategyPack;
use crate::stages::strategy::{BarRow, ModelBundle};

use super::artifact::{SignalAction, SignalArtifact, SignalIntent, SignalReason, SignalSide};
use super::compute::{SignalComputeRequest, compute};
use super::error::SignalError;

const EXECUTION: u64 = 300;
const EXECUTION_TIMEFRAME: i64 = 300;

fn states(ema_stack: Option<i8>) -> ModelStates {
    ModelStates {
        ema_stack,
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

#[derive(Clone)]
struct Bar {
    index: usize,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
    atr: Option<f64>,
    ema_stack: Option<i8>,
}

impl Bar {
    fn new(index: usize, open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            index,
            open,
            high,
            low,
            close,
            volume: Some(10.0),
            atr: Some(1.0),
            ema_stack: None,
        }
    }

    fn volume(mut self, volume: Option<f64>) -> Self {
        self.volume = volume;
        self
    }

    fn atr(mut self, atr: Option<f64>) -> Self {
        self.atr = atr;
        self
    }

    fn ema(mut self, ema_stack: Option<i8>) -> Self {
        self.ema_stack = ema_stack;
        self
    }

    fn row(&self) -> BarRow {
        let timestamp = self.index as i64 * EXECUTION_TIMEFRAME;
        let candle = Candle::new(
            self.index as u64,
            timestamp,
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume,
        )
        .expect("the test candle is valid");
        BarRow {
            timeframe_secs: EXECUTION,
            bar: self.index,
            timestamp,
            candle,
            states: states(self.ema_stack),
            progression_stage: None,
            progression_leg_direction: None,
            atr: self.atr,
        }
    }
}

fn event(
    bar: usize,
    event_type: u8,
    subtype: u8,
    source: u16,
    direction: i8,
    primary: Option<f64>,
    reference: Option<f64>,
) -> EventView {
    EventView {
        event_code: 1,
        event_type,
        subtype,
        source,
        direction,
        occurred_bar_index: bar,
        available_bar_index: bar,
        reference_code: None,
        primary_value: primary,
        reference_value: reference,
    }
}

fn bundle(bars: Vec<Bar>, events: Vec<EventView>) -> ModelBundle {
    let mut by_bar = BTreeMap::new();
    by_bar.insert(EXECUTION, bars.iter().map(Bar::row).collect());
    let mut by_timeframe = BTreeMap::new();
    by_timeframe.insert(EXECUTION, events);
    ModelBundle::from_parts(
        "data-v1".to_owned(),
        "model-config-v1".to_owned(),
        Some("2024-01-01".to_owned()),
        Some("2024-12-31".to_owned()),
        by_bar,
        by_timeframe,
    )
}

fn event_pack(exit_state: bool, time_stop: u64) -> StrategyPack {
    event_pack_with_direction(exit_state, time_stop, 1)
}

fn event_pack_with_direction(exit_state: bool, time_stop: u64, direction: i8) -> StrategyPack {
    let exit = if exit_state {
        json!([
            {"kind": "state", "timeframe": "5m", "field": "ema_stack", "value": -1}
        ])
    } else {
        json!([])
    };
    StrategyPack::parse(
        &json!({
            "schema": "quantrule.pack/v1",
            "kind": "strategy",
            "version": "1.0.0",
            "name": "signal-event-5m",
            "owner": "operator",
            "payload": {
                "mode": "signal",
                "engine": "event_composite",
                "execution_timeframe": "5m",
                "context_timeframes": [],
                "universe": ["BTCUSD"],
                "event_window_bars": 1,
                "time_stop_bars": time_stop,
                "require_contiguous": true,
                "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
                "entry": [
                    {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                     "source": 4, "direction": direction}
                ],
                "exit": exit
            }
        }),
        "pack.json",
    )
    .expect("the signal event pack is valid")
}

fn bracket_pack() -> StrategyPack {
    StrategyPack::parse(
        &json!({
            "schema": "quantrule.pack/v1",
            "kind": "strategy",
            "version": "1.0.0",
            "name": "signal-bracket-5m",
            "owner": "operator",
            "payload": {
                "mode": "signal",
                "engine": "compression_bracket",
                "execution_timeframe": "5m",
                "context_timeframes": [],
                "time_stop_bars": 12,
                "require_contiguous": true,
                "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
                "event": {"timeframe": "5m", "event_type": 3, "subtype": 8, "source": 5},
                "entry_valid_bars": 1
            }
        }),
        "pack.json",
    )
    .expect("the signal bracket pack is valid")
}

fn fold_pack() -> StrategyPack {
    StrategyPack::parse(
        &json!({
            "schema": "quantrule.pack/v1",
            "kind": "strategy",
            "version": "1.0.0",
            "name": "fold-event-5m",
            "owner": "operator",
            "payload": {
                "mode": "fold",
                "engine": "event_composite",
                "execution_timeframe": "5m",
                "context_timeframes": [],
                "event_window_bars": 1,
                "time_stop_bars": 12,
                "giveback_atr": 1.0,
                "giveback_arm_atr": 0.5,
                "require_contiguous": true,
                "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
                "entry": [
                    {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                     "source": 4, "direction": 1}
                ],
                "exit": []
            }
        }),
        "pack.json",
    )
    .expect("the fold pack is valid")
}

fn model_id() -> Digest {
    Digest::sha256(b"model-artifact")
}

fn compute_artifact(bundle: &ModelBundle, pack: &StrategyPack) -> SignalArtifact {
    let id = model_id();
    compute(&SignalComputeRequest {
        bundle,
        pack,
        model_artifact_id: &id,
    })
    .expect("the signal artifact computes")
}

fn rising_bars() -> Vec<Bar> {
    vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 102.0, 99.0, 101.0),
        Bar::new(2, 101.0, 102.0, 100.0, 101.0),
        Bar::new(3, 102.0, 105.0, 101.0, 104.0),
        Bar::new(4, 104.0, 105.0, 103.0, 104.0),
        Bar::new(5, 104.0, 105.0, 103.0, 104.0),
    ]
}

fn entry_event() -> EventView {
    event(1, 3, 1, 4, 1, Some(101.0), None)
}

#[test]
fn the_header_names_the_pack_model_data_and_period() {
    let pack = event_pack(false, 0);
    let artifact = compute_artifact(&bundle(rising_bars(), vec![entry_event()]), &pack);
    assert_eq!(artifact.strategy_pack_id(), pack.id());
    assert_eq!(artifact.model_artifact_id(), &model_id());
    assert_eq!(artifact.data_version(), "data-v1");
    assert_eq!(artifact.timeframe().code(), "5m");
    assert_eq!(artifact.period_from(), "2024-01-01");
    assert_eq!(artifact.period_to(), "2024-12-31");
    assert_eq!(artifact.rows().len(), 6);
    assert!(artifact.id().to_wire().starts_with("sha256:"));
    assert!(artifact.canonical_text().ends_with('\n'));
    assert_eq!(
        artifact.canonical_bytes(),
        artifact.canonical_text().as_bytes()
    );
}

#[test]
fn the_reference_position_enters_on_the_event_and_exits_on_the_target() {
    let artifact = compute_artifact(
        &bundle(rising_bars(), vec![entry_event()]),
        &event_pack(false, 0),
    );
    let rows = artifact.rows();
    assert_eq!(rows[0].action, SignalAction::Hold);
    assert_eq!(rows[1].action, SignalAction::Enter);
    assert_eq!(rows[1].side, Some(SignalSide::Long));
    assert_eq!(rows[1].reason, Some(SignalReason::Entry));
    assert!(matches!(rows[1].intent, Some(SignalIntent::Trade(_))));
    assert_eq!(rows[2].action, SignalAction::Hold);
    assert_eq!(rows[3].action, SignalAction::Exit);
    assert_eq!(rows[3].side, Some(SignalSide::Long));
    assert_eq!(rows[3].reason, Some(SignalReason::TakeProfit));
    assert_eq!(rows[4].action, SignalAction::Hold);
    assert_eq!(rows[5].action, SignalAction::Hold);
}

#[test]
fn the_artifact_is_byte_deterministic() {
    let bundle = bundle(rising_bars(), vec![entry_event()]);
    let pack = event_pack(false, 0);
    let first = compute_artifact(&bundle, &pack);
    let second = compute_artifact(&bundle, &pack);
    assert_eq!(first.id(), second.id());
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    assert_eq!(first, second);
}

#[test]
fn a_later_bar_cannot_change_an_earlier_row() {
    let full = compute_artifact(
        &bundle(rising_bars(), vec![entry_event()]),
        &event_pack(false, 0),
    );
    let mut shorter_bars = rising_bars();
    shorter_bars.truncate(4);
    let shorter = compute_artifact(
        &bundle(shorter_bars, vec![entry_event()]),
        &event_pack(false, 0),
    );
    assert_eq!(shorter.rows(), &full.rows()[..4]);

    let mut changed_bars = rising_bars();
    changed_bars[5] = Bar::new(5, 200.0, 220.0, 190.0, 210.0);
    let changed = compute_artifact(
        &bundle(changed_bars, vec![entry_event()]),
        &event_pack(false, 0),
    );
    assert_eq!(&changed.rows()[..5], &full.rows()[..5]);
}

#[test]
fn a_fold_pack_is_rejected() {
    let error = compute(&SignalComputeRequest {
        bundle: &bundle(rising_bars(), vec![entry_event()]),
        pack: &fold_pack(),
        model_artifact_id: &model_id(),
    })
    .expect_err("a fold pack has no signal artifact");
    assert!(matches!(error, SignalError::FoldMode));
}

#[test]
fn a_mismatched_model_or_pack_is_refused() {
    let pack = event_pack(false, 0);
    let other = event_pack(true, 0);
    let artifact = compute_artifact(&bundle(rising_bars(), vec![entry_event()]), &pack);
    artifact
        .verify_sources(&pack, &model_id())
        .expect("the frozen inputs match");
    let wrong_model = Digest::sha256(b"another-model");
    let error = artifact
        .verify_sources(&pack, &wrong_model)
        .expect_err("the model id differs");
    assert!(matches!(
        error,
        SignalError::SourceMismatch {
            field: "model_artifact_id",
            ..
        }
    ));
    let error = artifact
        .verify_sources(&other, &model_id())
        .expect_err("the pack id differs");
    assert!(matches!(
        error,
        SignalError::SourceMismatch {
            field: "strategy_pack_id",
            ..
        }
    ));
}

#[test]
fn a_short_series_keeps_the_last_decision_without_inventing_a_fill() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 102.0, 99.0, 101.0),
    ];
    let artifact = compute_artifact(&bundle(bars, vec![entry_event()]), &event_pack(false, 0));
    assert_eq!(artifact.rows().len(), 2);
    assert_eq!(artifact.rows()[1].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[1].side, Some(SignalSide::Long));
}

#[test]
fn an_empty_requested_timeframe_is_an_explicit_error() {
    let empty = bundle(Vec::new(), Vec::new());
    let error = compute(&SignalComputeRequest {
        bundle: &empty,
        pack: &event_pack(false, 0),
        model_artifact_id: &model_id(),
    })
    .expect_err("no execution bar");
    assert!(matches!(error, SignalError::EmptyExecution { .. }));

    let mut other = bundle(rising_bars(), Vec::new());
    other = ModelBundle::from_parts(
        other.data_version.clone(),
        other.config_id.clone(),
        other.period_from.clone(),
        other.period_to.clone(),
        BTreeMap::from([(900, rising_bars().iter().map(Bar::row).collect())]),
        BTreeMap::new(),
    );
    let error = compute(&SignalComputeRequest {
        bundle: &other,
        pack: &event_pack(false, 0),
        model_artifact_id: &model_id(),
    })
    .expect_err("the execution timeframe is absent");
    assert!(matches!(
        error,
        SignalError::Context(
            crate::stages::strategy::StrategyContextError::MissingTimeframe {
                timeframe_secs: 300
            }
        )
    ));
}

#[test]
fn a_single_bar_holds() {
    let artifact = compute_artifact(
        &bundle(
            vec![Bar::new(0, 100.0, 101.0, 99.0, 100.0)],
            vec![event(0, 3, 1, 4, 1, None, None)],
        ),
        &event_pack(false, 0),
    );
    assert_eq!(artifact.rows().len(), 1);
    assert_eq!(artifact.rows()[0].action, SignalAction::Hold);
}

#[test]
fn a_gap_rejects_the_pending_entry() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 101.0, 99.0, 100.0),
        Bar::new(3, 100.0, 101.0, 99.0, 100.0),
    ];
    let artifact = compute_artifact(&bundle(bars, vec![entry_event()]), &event_pack(false, 0));
    assert_eq!(artifact.rows()[1].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[2].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[2].reason, Some(SignalReason::EntryRejected));
    assert_eq!(artifact.rows()[2].side, None);
}

#[test]
fn a_zero_volume_bar_rejects_the_pending_entry() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 101.0, 99.0, 100.0),
        Bar::new(2, 100.0, 101.0, 99.0, 100.0).volume(Some(0.0)),
    ];
    let artifact = compute_artifact(&bundle(bars, vec![entry_event()]), &event_pack(false, 0));
    assert_eq!(artifact.rows()[2].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[2].reason, Some(SignalReason::EntryRejected));
}

#[test]
fn a_missing_atr_rejects_the_entry() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 101.0, 99.0, 100.0).atr(None),
        Bar::new(2, 100.0, 101.0, 99.0, 100.0).atr(None),
    ];
    let artifact = compute_artifact(&bundle(bars, vec![entry_event()]), &event_pack(false, 0));
    assert_eq!(artifact.rows()[1].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[2].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[2].reason, Some(SignalReason::EntryRejected));
}

#[test]
fn the_time_stop_closes_the_reference_position() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0).atr(Some(10.0)),
        Bar::new(1, 100.0, 101.0, 99.0, 100.0).atr(Some(10.0)),
        Bar::new(2, 100.0, 101.0, 99.0, 100.0).atr(Some(10.0)),
        Bar::new(3, 100.0, 101.0, 99.0, 100.0).atr(Some(10.0)),
        Bar::new(4, 100.0, 101.0, 99.0, 100.0).atr(Some(10.0)),
    ];
    let artifact = compute_artifact(&bundle(bars, vec![entry_event()]), &event_pack(false, 2));
    assert_eq!(artifact.rows()[2].action, SignalAction::Hold);
    assert_eq!(artifact.rows()[3].action, SignalAction::Hold);
    assert_eq!(artifact.rows()[4].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[4].reason, Some(SignalReason::TimeStop));
    assert_eq!(artifact.rows()[4].side, Some(SignalSide::Long));
}

#[test]
fn an_event_or_state_exit_closes_the_reference_position() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 101.0, 99.0, 100.0),
        Bar::new(2, 100.0, 101.0, 99.0, 100.0),
        Bar::new(3, 100.0, 101.0, 99.0, 100.0).ema(Some(-1)),
    ];
    let artifact = compute_artifact(&bundle(bars, vec![entry_event()]), &event_pack(true, 0));
    assert_eq!(artifact.rows()[1].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[3].action, SignalAction::Exit);
    assert_eq!(
        artifact.rows()[3].reason,
        Some(SignalReason::EventCondition)
    );
    assert_eq!(artifact.rows()[3].side, Some(SignalSide::Long));
    assert_eq!(artifact.rows()[3].evidence.len(), 1);
}

#[test]
fn a_two_sided_bracket_chooses_the_side_at_execution() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 101.0, 99.0, 100.0),
        Bar::new(2, 104.0, 106.0, 103.5, 105.0),
        Bar::new(3, 105.0, 109.0, 104.0, 108.0),
    ];
    let events = vec![event(1, 3, 8, 5, 0, Some(105.0), Some(95.0))];
    let artifact = compute_artifact(&bundle(bars, events), &bracket_pack());
    assert_eq!(artifact.rows()[1].action, SignalAction::Bracket);
    assert_eq!(artifact.rows()[1].side, None);
    assert!(matches!(
        artifact.rows()[1].intent,
        Some(SignalIntent::Bracket(_))
    ));
    assert_eq!(artifact.rows()[2].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[2].side, Some(SignalSide::Long));
    assert_eq!(artifact.rows()[2].reason, Some(SignalReason::BracketFill));
    assert_eq!(artifact.rows()[3].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[3].reason, Some(SignalReason::TakeProfit));
}

#[test]
fn an_ambiguous_bracket_is_rejected() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 101.0, 99.0, 100.0),
        Bar::new(2, 100.0, 106.0, 94.0, 100.0),
    ];
    let events = vec![event(1, 3, 8, 5, 0, Some(105.0), Some(95.0))];
    let artifact = compute_artifact(&bundle(bars, events), &bracket_pack());
    assert_eq!(artifact.rows()[1].action, SignalAction::Bracket);
    assert_eq!(artifact.rows()[2].action, SignalAction::Exit);
    assert_eq!(
        artifact.rows()[2].reason,
        Some(SignalReason::BracketRejected)
    );
    assert_eq!(artifact.rows()[2].side, None);
}

#[test]
fn an_atr_stop_closes_the_reference_position() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 102.0, 99.0, 101.0),
        Bar::new(2, 101.0, 102.0, 100.0, 101.0),
        Bar::new(3, 102.0, 103.0, 98.0, 99.0),
    ];
    let artifact = compute_artifact(&bundle(bars, vec![entry_event()]), &event_pack(false, 0));
    assert_eq!(artifact.rows()[1].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[2].action, SignalAction::Hold);
    assert_eq!(artifact.rows()[3].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[3].reason, Some(SignalReason::StopLoss));
    assert_eq!(artifact.rows()[3].side, Some(SignalSide::Long));
}

#[test]
fn a_short_entry_takes_the_short_target() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 102.0, 99.0, 101.0),
        Bar::new(2, 101.0, 102.0, 100.0, 101.0),
        Bar::new(3, 99.0, 100.0, 97.0, 98.0),
    ];
    let events = vec![event(1, 3, 1, 4, -1, Some(101.0), None)];
    let artifact = compute_artifact(
        &bundle(bars, events),
        &event_pack_with_direction(false, 0, 0),
    );
    assert_eq!(artifact.rows()[1].action, SignalAction::Enter);
    assert_eq!(artifact.rows()[1].side, Some(SignalSide::Short));
    assert_eq!(artifact.rows()[3].action, SignalAction::Exit);
    assert_eq!(artifact.rows()[3].reason, Some(SignalReason::TakeProfit));
    assert_eq!(artifact.rows()[3].side, Some(SignalSide::Short));
}

#[test]
fn reordered_events_produce_the_same_bytes() {
    let bars = vec![
        Bar::new(0, 100.0, 101.0, 99.0, 100.0),
        Bar::new(1, 100.0, 102.0, 99.0, 101.0),
    ];
    let mut later = event(1, 3, 1, 4, 1, Some(101.0), None);
    later.event_code = 2;
    let mut earlier = event(0, 3, 1, 4, 1, Some(100.0), None);
    earlier.event_code = 1;
    let left = vec![later, earlier];
    let right = vec![earlier, later];
    let first = compute_artifact(&bundle(bars.clone(), left), &event_pack(false, 0));
    let second = compute_artifact(&bundle(bars, right), &event_pack(false, 0));
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn a_canonical_artifact_round_trips_and_a_non_canonical_one_is_refused() {
    let artifact = compute_artifact(
        &bundle(rising_bars(), vec![entry_event()]),
        &event_pack(false, 0),
    );
    let stored = SignalArtifact::from_stored_bytes(artifact.canonical_bytes(), "signal.json")
        .expect("the canonical bytes round trip");
    assert_eq!(stored.id(), artifact.id());
    assert_eq!(stored.canonical_bytes(), artifact.canonical_bytes());
    assert_eq!(stored, artifact);

    let pretty = serde_json::to_string_pretty(&artifact.to_value()).expect("the value is JSON");
    let error = SignalArtifact::from_stored_bytes(pretty.as_bytes(), "signal.json")
        .expect_err("pretty bytes are not canonical");
    assert!(matches!(error, SignalError::NonCanonical { .. }));
}

#[test]
fn the_closed_parser_refuses_a_fill_or_a_quantity_field() {
    let artifact = compute_artifact(
        &bundle(rising_bars(), vec![entry_event()]),
        &event_pack(false, 0),
    );
    let mut value = artifact.to_value();
    value["rows"][0]["quantity"] = json!(1.0);
    let error = SignalArtifact::parse_json(
        &serde_json::to_string(&value).expect("the edited value is JSON"),
        "signal.json",
    )
    .expect_err("a quantity is not a signal field");
    assert!(error.to_string().contains("quantity"));
}

#[test]
fn the_parser_rejects_an_inconsistent_row_shape() {
    let artifact = compute_artifact(
        &bundle(rising_bars(), vec![entry_event()]),
        &event_pack(false, 0),
    );
    let mut value = artifact.to_value();
    value["rows"][0]["side"] = json!("long");
    let error = SignalArtifact::parse_json(
        &serde_json::to_string(&value).expect("the edited value is JSON"),
        "signal.json",
    )
    .expect_err("a hold cannot carry a side");
    assert!(error.to_string().contains("hold"));

    let mut value = artifact.to_value();
    value["rows"][1]["bar_index"] = json!(0);
    let error = SignalArtifact::parse_json(
        &serde_json::to_string(&value).expect("the edited value is JSON"),
        "signal.json",
    )
    .expect_err("rows are strictly ascending");
    assert!(error.to_string().contains("ascending"));
}

#[test]
fn the_parser_refuses_free_text_evidence() {
    let artifact = compute_artifact(
        &bundle(rising_bars(), vec![entry_event()]),
        &event_pack(false, 0),
    );
    let mut value = artifact.to_value();
    value["rows"][1]["evidence"][0]["label"] = json!("buy");
    let error = SignalArtifact::parse_json(
        &serde_json::to_string(&value).expect("the edited value is JSON"),
        "signal.json",
    )
    .expect_err("evidence is a closed event or state reference, not free text");
    assert!(error.to_string().contains("label"));
}

#[test]
fn a_signal_pack_cannot_enable_giveback() {
    let mut value = json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": "1.0.0",
        "name": "signal-giveback",
        "owner": "operator",
        "payload": {
            "mode": "signal",
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "giveback_atr": 1.0,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                 "source": 4, "direction": 1}
            ],
            "exit": []
        }
    });
    let error = StrategyPack::parse(&value, "pack.json").expect_err("giveback is fold-only");
    assert!(error.to_string().contains("signal mode"));

    value["payload"]["giveback_atr"] = json!(null);
    assert!(
        StrategyPack::parse(&value, "pack.json").is_ok(),
        "a null giveback is the signal default"
    );
}
