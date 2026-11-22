use serde_json::{Value, json};

use super::{StrategyMode, StrategyPack};
use crate::config::{ConfigError, ConfigErrorKind};
use crate::pack::{PackError, PackKind, PackState};

fn signal() -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": "1.0.0",
        "name": "compression-fade-5m",
        "owner": "operator",
        "payload": {
            "mode": "signal",
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": ["15m"],
            "universe": ["BTCUSD"],
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "giveback_atr": null,
            "giveback_arm_atr": 0.0,
            "require_contiguous": true,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1, "source": 4, "direction": 1}
            ],
            "exit": []
        }
    })
}

fn fold() -> Value {
    let mut value = signal();
    value["payload"]["mode"] = json!("fold");
    value["payload"]["time_stop_bars"] = json!(12);
    value["payload"]["giveback_atr"] = json!(1.0);
    value["payload"]["giveback_arm_atr"] = json!(0.5);
    value
}

fn bracket() -> Value {
    let mut value = signal();
    value["payload"] = json!({
        "mode": "fold",
        "engine": "compression_bracket",
        "execution_timeframe": "5m",
        "context_timeframes": [],
        "time_stop_bars": 12,
        "require_contiguous": true,
        "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
        "event": {"timeframe": "5m", "event_type": 3, "subtype": 8, "source": 5},
        "entry_valid_bars": 1
    });
    value
}

fn document_error(error: PackError) -> ConfigError {
    match error {
        PackError::Document(error) => error,
        other => panic!("expected a document error, got {other}"),
    }
}

#[test]
fn a_signal_pack_parses_and_carries_its_identity() {
    let pack = StrategyPack::parse(&signal(), "pack.json").expect("the pack is valid");
    assert_eq!(pack.metadata().kind(), PackKind::Strategy);
    assert_eq!(pack.metadata().version(), "1.0.0");
    assert_eq!(pack.metadata().name(), "compression-fade-5m");
    assert_eq!(pack.metadata().owner(), "operator");
    assert_eq!(pack.payload().mode(), StrategyMode::Signal);
    assert_eq!(pack.payload().universe(), ["BTCUSD"]);
    assert_eq!(pack.payload().strategy().engine_name(), "event_composite");
    assert_eq!(pack.payload().strategy().execution_timeframe.code(), "5m");
    assert_eq!(pack.payload().strategy().context_timeframes.len(), 1);
    assert_eq!(pack.payload().strategy().risk.stop_atr_multiple(), 2.0);
    assert_eq!(pack.payload().strategy().risk.reward_to_risk(), 1.5);
    assert_eq!(pack.state(), PackState::Draft);
    assert!(pack.id().to_wire().starts_with("sha256:"));
    assert!(pack.canonical_text().ends_with('\n'));
    assert_eq!(pack.canonical_bytes(), pack.canonical_text().as_bytes());
}

#[test]
fn a_fold_pack_with_giveback_parses() {
    let pack = StrategyPack::parse(&fold(), "pack.json").expect("the fold pack is valid");
    assert_eq!(pack.payload().mode(), StrategyMode::Fold);
}

#[test]
fn a_compression_bracket_pack_parses() {
    let pack = StrategyPack::parse(&bracket(), "pack.json").expect("the bracket pack is valid");
    assert_eq!(
        pack.payload().strategy().engine_name(),
        "compression_bracket"
    );
    assert_eq!(
        pack.payload().strategy().compression_event(),
        Some((300, 3, 8, 5))
    );
}

#[test]
fn the_identity_is_stable_across_key_order_and_whitespace() {
    let left = signal();
    let text = serde_json::to_string(&left).expect("the document serialises");
    let pretty = serde_json::to_string_pretty(&left).expect("the document serialises");
    let first = StrategyPack::parse_json(&text, "pack.json").expect("the pack is valid");
    let second = StrategyPack::parse_json(&pretty, "pack.json").expect("the pack is valid");
    assert_eq!(first.id(), second.id());
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn the_identity_changes_with_the_content() {
    let first = StrategyPack::parse(&signal(), "pack.json").expect("the pack is valid");
    let mut changed = signal();
    changed["payload"]["risk"]["reward_to_risk"] = json!(2.0);
    let second = StrategyPack::parse(&changed, "pack.json").expect("the pack is valid");
    assert_ne!(first.id(), second.id());
    assert_ne!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn the_envelope_fields_are_part_of_the_identity() {
    let base = StrategyPack::parse(&signal(), "pack.json").expect("the pack is valid");
    for (field, replacement) in [
        ("name", json!("another-name")),
        ("version", json!("1.0.1")),
        ("owner", json!("another-owner")),
    ] {
        let mut changed = signal();
        changed[field] = replacement;
        let pack = StrategyPack::parse(&changed, "pack.json").expect("the pack is valid");
        assert_ne!(base.id(), pack.id(), "{field} must change the identity");
    }
}

#[test]
fn a_stored_canonical_pack_round_trips() {
    let pack = StrategyPack::parse(&signal(), "pack.json").expect("the pack is valid");
    let stored =
        StrategyPack::from_stored_bytes(pack.canonical_bytes(), "pack.json").expect("canonical");
    assert_eq!(stored.id(), pack.id());
    assert_eq!(stored.canonical_bytes(), pack.canonical_bytes());
    assert_eq!(stored.metadata(), pack.metadata());
}

#[test]
fn a_non_canonical_store_is_rejected() {
    let document = signal();
    let pretty = serde_json::to_string_pretty(&document).expect("the document serialises");
    let error = StrategyPack::from_stored_bytes(pretty.as_bytes(), "pack.json")
        .expect_err("pretty JSON is not canonical");
    assert!(matches!(error, PackError::NonCanonical { .. }));
}

#[test]
fn non_utf8_stored_bytes_are_rejected() {
    let error = StrategyPack::from_stored_bytes(&[0xff, 0xfe], "pack.json")
        .expect_err("invalid UTF-8 is not a pack");
    assert_eq!(document_error(error).kind(), ConfigErrorKind::InvalidJson);
}

#[test]
fn invalid_json_is_rejected() {
    let error = StrategyPack::parse_json("{", "pack.json").expect_err("invalid JSON");
    assert_eq!(document_error(error).kind(), ConfigErrorKind::InvalidJson);
}

#[test]
fn an_unknown_mode_or_engine_is_rejected() {
    let mut value = signal();
    value["payload"]["mode"] = json!("live");
    let error =
        document_error(StrategyPack::parse(&value, "pack.json").expect_err("the mode is unknown"));
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.mode");

    let mut value = signal();
    value["payload"]["engine"] = json!("weighted_score");
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the engine is unknown"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.engine");
}

#[test]
fn an_unknown_payload_field_is_rejected() {
    let mut value = signal();
    value["payload"]["confidence"] = json!(0.9);
    let error =
        document_error(StrategyPack::parse(&value, "pack.json").expect_err("the field is unknown"));
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.confidence");
}

#[test]
fn a_statistical_condition_field_is_rejected() {
    let mut value = signal();
    value["payload"]["entry"][0]["weight"] = json!(0.4);
    let error =
        document_error(StrategyPack::parse(&value, "pack.json").expect_err("the field is unknown"));
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.entry[0].weight");
}

#[test]
fn a_statistical_risk_field_is_rejected() {
    let mut value = signal();
    value["payload"]["risk"]["confidence"] = json!(0.99);
    let error =
        document_error(StrategyPack::parse(&value, "pack.json").expect_err("the field is unknown"));
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.risk.confidence");
}

#[test]
fn a_signal_pack_rejects_a_non_null_giveback() {
    let mut value = signal();
    value["payload"]["giveback_atr"] = json!(1.0);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("giveback is fold-only"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.giveback_atr");
}

#[test]
fn a_signal_pack_rejects_a_nonzero_giveback_arm() {
    let mut value = signal();
    value["payload"]["giveback_arm_atr"] = json!(0.5);
    let error =
        document_error(StrategyPack::parse(&value, "pack.json").expect_err("the arm is fold-only"));
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.giveback_arm_atr");
}

#[test]
fn a_fold_pack_rejects_an_arm_without_a_threshold() {
    let mut value = fold();
    value["payload"]["giveback_atr"] = Value::Null;
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the arm has no threshold"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.payload.giveback_arm_atr");
}

#[test]
fn context_timeframes_must_be_ascending() {
    let mut value = signal();
    value["payload"]["context_timeframes"] = json!(["15m", "5m"]);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the order is descending"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.context_timeframes[1]");
}

#[test]
fn a_context_timeframe_equal_to_execution_is_rejected() {
    let mut value = signal();
    value["payload"]["context_timeframes"] = json!(["5m"]);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the context repeats execution"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.context_timeframes[0]");
}

#[test]
fn a_duplicate_context_timeframe_is_rejected() {
    let mut value = signal();
    value["payload"]["context_timeframes"] = json!(["15m", "15m"]);
    let error =
        document_error(StrategyPack::parse(&value, "pack.json").expect_err("the context repeats"));
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.context_timeframes[1]");
}

#[test]
fn an_unregistered_event_is_rejected() {
    let mut value = signal();
    value["payload"]["entry"][0]["subtype"] = json!(2);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the event is not registered"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload");
    assert!(error.to_string().contains("not registered"));
}

#[test]
fn a_diagnostic_event_is_rejected() {
    let mut value = signal();
    value["payload"]["entry"][0]["subtype"] = json!(21);
    value["payload"]["entry"][0]["source"] = json!(41);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the event is diagnostic"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert!(error.to_string().contains("diagnostic"));
}

#[test]
fn duplicate_conditions_are_rejected() {
    let mut value = signal();
    let condition = value["payload"]["entry"][0].clone();
    value["payload"]["entry"] = json!([condition.clone(), condition]);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the condition repeats"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert!(error.to_string().contains("duplicated"));
}

#[test]
fn conflicting_entry_directions_are_rejected() {
    let mut value = signal();
    let first = value["payload"]["entry"][0].clone();
    let mut opposite = first.clone();
    opposite["subtype"] = json!(24);
    opposite["source"] = json!(5);
    opposite["direction"] = json!(-1);
    value["payload"]["entry"] = json!([first, opposite]);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the directions conflict"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert!(error.to_string().contains("disagree"));
}

#[test]
fn a_state_value_outside_its_range_is_rejected() {
    let mut value = signal();
    value["payload"]["exit"] = json!([
        {"kind": "state", "timeframe": "5m", "field": "vol_regime", "value": 3}
    ]);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the state value is outside"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert!(error.to_string().contains("outside its field's range"));
}

#[test]
fn a_bracket_must_use_the_registered_compression_event() {
    let mut value = bracket();
    value["payload"]["event"]["subtype"] = json!(7);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the event is not the range event"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert!(error.to_string().contains("compression-range"));
}

#[test]
fn a_bracket_pack_rejects_event_composite_fields() {
    let mut value = bracket();
    value["payload"]["event_window_bars"] = json!(3);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json")
            .expect_err("the field belongs to the other engine"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.event_window_bars");
}

#[test]
fn a_wrong_pack_kind_is_rejected() {
    let mut value = signal();
    value["kind"] = json!("portfolio");
    let error = StrategyPack::parse(&value, "pack.json").expect_err("the parser is for strategy");
    assert!(matches!(
        error,
        PackError::WrongKind {
            expected: PackKind::Strategy,
            found: PackKind::Portfolio
        }
    ));
}

#[test]
fn an_empty_or_duplicate_universe_instrument_is_rejected() {
    let mut value = signal();
    value["payload"]["universe"] = json!([""]);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the instrument is empty"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.universe[0]");

    let mut value = signal();
    value["payload"]["universe"] = json!(["BTCUSD", "BTCUSD"]);
    let error = document_error(
        StrategyPack::parse(&value, "pack.json").expect_err("the instrument repeats"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.universe[1]");
}

#[test]
fn a_missing_required_payload_field_is_rejected() {
    let mut value = signal();
    value["payload"]
        .as_object_mut()
        .expect("an object")
        .remove("risk");
    let error =
        document_error(StrategyPack::parse(&value, "pack.json").expect_err("the risk is missing"));
    assert_eq!(error.kind(), ConfigErrorKind::MissingField);
    assert_eq!(error.path(), "$.payload.risk");
}

#[test]
fn a_draft_freezes_without_changing_its_bytes() {
    let mut pack = StrategyPack::parse(&signal(), "pack.json").expect("the pack is valid");
    let id = pack.id().clone();
    let canonical = pack.canonical_bytes().to_vec();
    pack.validate().expect("draft validates");
    assert_eq!(pack.state(), PackState::Validated);
    pack.freeze().expect("validated freezes");
    assert_eq!(pack.state(), PackState::Frozen);
    assert_eq!(pack.id(), &id);
    assert_eq!(pack.canonical_bytes(), canonical);
    pack.retire().expect("frozen retires");
    assert_eq!(pack.state(), PackState::Retired);
    assert_eq!(pack.id(), &id);
    assert_eq!(pack.canonical_bytes(), canonical);
}

#[test]
fn invalid_lifecycle_transitions_are_rejected() {
    let mut pack = StrategyPack::parse(&signal(), "pack.json").expect("the pack is valid");
    assert!(pack.freeze().is_err(), "a draft cannot freeze");
    pack.validate().expect("draft validates");
    assert!(pack.validate().is_err(), "validated cannot revalidate");
    assert!(pack.retire().is_err(), "validated cannot retire");
    pack.freeze().expect("validated freezes");
    assert!(pack.validate().is_err(), "frozen cannot revalidate");
    assert!(pack.reject().is_err(), "frozen cannot return to draft");
}

#[test]
fn rejecting_a_validated_pack_returns_it_to_draft() {
    let mut pack = StrategyPack::parse(&signal(), "pack.json").expect("the pack is valid");
    pack.validate().expect("draft validates");
    pack.reject().expect("validated rejects");
    assert_eq!(pack.state(), PackState::Draft);
    pack.validate().expect("the corrected draft validates");
    pack.freeze().expect("the corrected pack freezes");
    assert_eq!(pack.state(), PackState::Frozen);
}
