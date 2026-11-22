use quantrule_backtest::execution::ExecutionConfig;
use quantrule_backtest::portfolio::{
    CommissionBase, CommissionConfig, CommissionPer, FundingConfig, FundingInterval,
};
use quantrule_data::Timeframe;
use serde_json::{Value, json};

use super::{EndOfDataPolicy, ExecutionPack, OrderType, TieBreak, ZeroVolumePolicy};
use crate::config::{ConfigError, ConfigErrorKind};
use crate::pack::{PackError, PackIndex, PackKind, PackState};

fn execution() -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "execution",
        "version": "1.0.0",
        "name": "btcusd-conservative-fills",
        "owner": "operator",
        "payload": {
            "fill_timeframe": "5m",
            "order_type": "market_next_open",
            "tie_break": "stop_first",
            "fee_rate": 0.0,
            "slippage_bps": 2.0,
            "commission": {
                "rate": 0.0002,
                "base": "equity",
                "per": "operation"
            },
            "funding": {
                "rate": 0.0,
                "interval_hours": 8
            },
            "zero_volume": "reject",
            "end_of_data": "leave_open"
        }
    })
}

fn document_error(error: PackError) -> ConfigError {
    match error {
        PackError::Document(error) => error,
        other => panic!("expected a document error, got {other}"),
    }
}

#[test]
fn an_execution_pack_parses_and_carries_its_identity() {
    let pack = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    assert_eq!(pack.metadata().kind(), PackKind::Execution);
    assert_eq!(pack.metadata().version(), "1.0.0");
    assert_eq!(pack.metadata().name(), "btcusd-conservative-fills");
    assert_eq!(pack.metadata().owner(), "operator");
    assert_eq!(pack.payload().fill_timeframe(), Timeframe::M5);
    assert_eq!(pack.payload().order_type(), OrderType::MarketNextOpen);
    assert_eq!(pack.payload().tie_break(), TieBreak::StopFirst);
    assert_eq!(
        pack.payload().costs(),
        ExecutionConfig::new(0.0, 2.0).expect("the costs are valid")
    );
    assert_eq!(pack.payload().fee_rate(), 0.0);
    assert_eq!(pack.payload().slippage_bps(), 2.0);
    assert_eq!(
        pack.payload().commission(),
        CommissionConfig::new(0.0002, CommissionBase::Equity, CommissionPer::Operation)
            .expect("the commission is valid")
    );
    assert_eq!(
        pack.payload().funding(),
        FundingConfig::new(0.0, FundingInterval::H8).expect("the funding is valid")
    );
    assert_eq!(pack.payload().zero_volume(), ZeroVolumePolicy::Reject);
    assert_eq!(pack.payload().end_of_data(), EndOfDataPolicy::LeaveOpen);
    assert_eq!(pack.state(), PackState::Draft);
    assert!(pack.id().to_wire().starts_with("sha256:"));
    assert!(pack.canonical_text().ends_with('\n'));
    assert_eq!(pack.canonical_bytes(), pack.canonical_text().as_bytes());
}

#[test]
fn the_closed_policy_codes_round_trip() {
    assert_eq!(
        OrderType::from_code(OrderType::MarketNextOpen.code()),
        Some(OrderType::MarketNextOpen)
    );
    assert_eq!(
        TieBreak::from_code(TieBreak::StopFirst.code()),
        Some(TieBreak::StopFirst)
    );
    assert_eq!(
        ZeroVolumePolicy::from_code(ZeroVolumePolicy::Reject.code()),
        Some(ZeroVolumePolicy::Reject)
    );
    assert_eq!(
        EndOfDataPolicy::from_code(EndOfDataPolicy::LeaveOpen.code()),
        Some(EndOfDataPolicy::LeaveOpen)
    );
}

#[test]
fn an_unsupported_order_or_tie_policy_is_rejected() {
    let mut value = execution();
    value["payload"]["order_type"] = json!("limit");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the order style is unsupported"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.order_type");

    let mut value = execution();
    value["payload"]["tie_break"] = json!("target_first");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the tie rule is unsupported"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.tie_break");

    let mut value = execution();
    value["payload"]["order_type"] = json!(1);
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the order style is not a string"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Type);
    assert_eq!(error.path(), "$.payload.order_type");
}

#[test]
fn an_unsupported_zero_volume_or_end_of_data_policy_is_rejected() {
    let mut value = execution();
    value["payload"]["zero_volume"] = json!("skip");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the zero-volume rule is unsupported"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.zero_volume");

    let mut value = execution();
    value["payload"]["end_of_data"] = json!("close_last");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the end-of-data rule is unsupported"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.end_of_data");
}

#[test]
fn an_unsupported_fill_timeframe_is_rejected() {
    let mut value = execution();
    value["payload"]["fill_timeframe"] = json!("4h");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the timeframe is above the ceiling"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.fill_timeframe");

    let mut value = execution();
    value["payload"]["fill_timeframe"] = json!("2m");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the timeframe is unknown"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.fill_timeframe");
}

#[test]
fn fee_and_slippage_are_bounded() {
    let mut value = execution();
    value["payload"]["fee_rate"] = json!(-0.0001);
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the fee rate is negative"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.fee_rate");

    let mut value = execution();
    value["payload"]["slippage_bps"] = json!(-1.0);
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the slippage is negative"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.slippage_bps");

    let mut value = execution();
    value["payload"]["slippage_bps"] = json!(10_000.0);
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json")
            .expect_err("the slippage exceeds the contract bound"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.slippage_bps");

    let mut value = execution();
    value["payload"]["fee_rate"] = json!(0.001);
    value["payload"]["slippage_bps"] = json!(9_999.999);
    ExecutionPack::parse(&value, "pack.json").expect("the upper bound is exclusive and valid");
}

#[test]
fn commission_is_closed_and_validated() {
    for (field, replacement, kind, path) in [
        (
            "rate",
            json!(-0.0001),
            ConfigErrorKind::Range,
            "$.payload.commission.rate",
        ),
        (
            "base",
            json!("notional"),
            ConfigErrorKind::Unsupported,
            "$.payload.commission.base",
        ),
        (
            "per",
            json!("bar"),
            ConfigErrorKind::Unsupported,
            "$.payload.commission.per",
        ),
    ] {
        let mut value = execution();
        value["payload"]["commission"][field] = replacement;
        let error = document_error(
            ExecutionPack::parse(&value, "pack.json").expect_err("the commission is invalid"),
        );
        assert_eq!(error.kind(), kind);
        assert_eq!(error.path(), path);
    }

    let mut value = execution();
    value["payload"]["commission"]["tier"] = json!(1);
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the commission field is unknown"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.commission.tier");

    let mut value = execution();
    value["payload"]["commission"]
        .as_object_mut()
        .expect("an object")
        .remove("rate");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the commission rate is missing"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::MissingField);
    assert_eq!(error.path(), "$.payload.commission.rate");
}

#[test]
fn funding_is_closed_and_validated() {
    for hours in [0, 3, 25] {
        let mut value = execution();
        value["payload"]["funding"]["interval_hours"] = json!(hours);
        let error = document_error(
            ExecutionPack::parse(&value, "pack.json").expect_err("the interval is unsupported"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::Range);
        assert_eq!(error.path(), "$.payload.funding.interval_hours");
    }

    let mut value = execution();
    value["payload"]["funding"]["rate"] = json!(-0.0001);
    let pack = ExecutionPack::parse(&value, "pack.json")
        .expect("a negative rate reverses the funding payer");
    assert_eq!(pack.payload().funding().rate(), -0.0001);

    let mut value = execution();
    value["payload"]["funding"]["mode"] = json!("fixed");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the funding field is unknown"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.funding.mode");

    let mut value = execution();
    value["payload"]["funding"]
        .as_object_mut()
        .expect("an object")
        .remove("rate");
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the funding rate is missing"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::MissingField);
    assert_eq!(error.path(), "$.payload.funding.rate");
}

#[test]
fn broker_live_market_and_statistical_fields_are_rejected() {
    for field in [
        "broker",
        "venue",
        "order_routing",
        "latency_ms",
        "participation_rate",
        "order_book",
        "volatility",
        "confidence",
        "slippage_model",
        "fill_probability",
    ] {
        let mut value = execution();
        value["payload"][field] = json!(1.0);
        let error = document_error(
            ExecutionPack::parse(&value, "pack.json").expect_err("the field is not in the schema"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::UnknownField, "{field}");
        assert_eq!(error.path(), format!("$.payload.{field}"));
    }
}

#[test]
fn a_missing_required_payload_field_is_rejected() {
    for field in [
        "fill_timeframe",
        "order_type",
        "tie_break",
        "fee_rate",
        "slippage_bps",
        "commission",
        "funding",
        "zero_volume",
        "end_of_data",
    ] {
        let mut value = execution();
        value["payload"]
            .as_object_mut()
            .expect("an object")
            .remove(field);
        let error = document_error(
            ExecutionPack::parse(&value, "pack.json").expect_err("the field is missing"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::MissingField, "{field}");
        assert_eq!(error.path(), format!("$.payload.{field}"));
    }
}

#[test]
fn a_non_object_payload_is_rejected() {
    let mut value = execution();
    value["payload"] = json!([]);
    let error = document_error(
        ExecutionPack::parse(&value, "pack.json").expect_err("the payload is not an object"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Type);
    assert_eq!(error.path(), "$.payload");
}

#[test]
fn the_identity_is_stable_across_key_order_and_whitespace() {
    let left = execution();
    let text = serde_json::to_string(&left).expect("the document serialises");
    let pretty = serde_json::to_string_pretty(&left).expect("the document serialises");
    let first = ExecutionPack::parse_json(&text, "pack.json").expect("the pack is valid");
    let second = ExecutionPack::parse_json(&pretty, "pack.json").expect("the pack is valid");
    assert_eq!(first.id(), second.id());
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn parsing_the_same_document_twice_is_byte_identical() {
    let first = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    let second = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    assert_eq!(first.canonical_text(), second.canonical_text());
    assert_eq!(first.id(), second.id());
}

#[test]
fn the_identity_changes_with_the_content() {
    let first = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    let mut changed = execution();
    changed["payload"]["slippage_bps"] = json!(3.0);
    let second = ExecutionPack::parse(&changed, "pack.json").expect("the pack is valid");
    assert_ne!(first.id(), second.id());
    assert_ne!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn the_envelope_fields_are_part_of_the_identity() {
    let base = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    for (field, replacement) in [
        ("name", json!("another-name")),
        ("version", json!("1.0.1")),
        ("owner", json!("another-owner")),
    ] {
        let mut changed = execution();
        changed[field] = replacement;
        let pack = ExecutionPack::parse(&changed, "pack.json").expect("the pack is valid");
        assert_ne!(base.id(), pack.id(), "{field} must change the identity");
    }
}

#[test]
fn a_stored_canonical_pack_round_trips() {
    let pack = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    let stored = ExecutionPack::from_stored_bytes(pack.canonical_bytes(), "pack.json")
        .expect("the bytes are canonical");
    assert_eq!(stored.id(), pack.id());
    assert_eq!(stored.canonical_bytes(), pack.canonical_bytes());
    assert_eq!(stored.metadata(), pack.metadata());
}

#[test]
fn a_non_canonical_store_is_rejected() {
    let document = execution();
    let pretty = serde_json::to_string_pretty(&document).expect("the document serialises");
    let error = ExecutionPack::from_stored_bytes(pretty.as_bytes(), "pack.json")
        .expect_err("pretty JSON is not canonical");
    assert!(matches!(error, PackError::NonCanonical { .. }));
}

#[test]
fn non_utf8_and_invalid_json_are_rejected() {
    let error = ExecutionPack::from_stored_bytes(&[0xff, 0xfe], "pack.json")
        .expect_err("invalid UTF-8 is not a pack");
    assert_eq!(document_error(error).kind(), ConfigErrorKind::InvalidJson);

    let error = ExecutionPack::parse_json("{", "pack.json").expect_err("invalid JSON");
    assert_eq!(document_error(error).kind(), ConfigErrorKind::InvalidJson);
}

#[test]
fn a_wrong_pack_kind_is_rejected() {
    let mut value = execution();
    value["kind"] = json!("portfolio");
    let error =
        ExecutionPack::parse(&value, "pack.json").expect_err("the parser is for execution packs");
    assert!(matches!(
        error,
        PackError::WrongKind {
            expected: PackKind::Execution,
            found: PackKind::Portfolio
        }
    ));
}

#[test]
fn a_draft_freezes_without_changing_its_bytes() {
    let mut pack = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
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
    let mut pack = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    assert!(pack.freeze().is_err(), "a draft cannot freeze");
    pack.validate().expect("draft validates");
    assert!(pack.validate().is_err(), "validated cannot revalidate");
    assert!(pack.retire().is_err(), "validated cannot retire");
    pack.freeze().expect("validated freezes");
    assert!(pack.validate().is_err(), "frozen cannot revalidate");
    assert!(pack.reject().is_err(), "frozen cannot return to draft");
}

#[test]
fn the_identity_index_accepts_an_execution_pack() {
    let pack = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    let mut index = PackIndex::new();
    index.register(&pack).expect("the first registration");
    index
        .register(&pack)
        .expect("the same bytes are idempotent");
    assert_eq!(index.len(), 1);
    assert_eq!(
        index.resolve("btcusd-conservative-fills", "1.0.0"),
        Some(pack.id())
    );
}

#[test]
fn an_execution_label_with_different_bytes_is_rejected() {
    let first = ExecutionPack::parse(&execution(), "pack.json").expect("the pack is valid");
    let mut changed = execution();
    changed["payload"]["slippage_bps"] = json!(5.0);
    let second = ExecutionPack::parse(&changed, "pack.json").expect("the pack is valid");
    let mut index = PackIndex::new();
    index.register(&first).expect("the first registration");
    let conflict = index
        .register(&second)
        .expect_err("the same label must not change bytes");
    assert_eq!(conflict.name(), "btcusd-conservative-fills");
    assert_eq!(conflict.version(), "1.0.0");
    assert_eq!(conflict.existing(), first.id());
    assert_eq!(conflict.incoming(), second.id());
}
