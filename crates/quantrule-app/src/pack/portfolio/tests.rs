use quantrule_backtest::portfolio::{DrawdownAction, SizingMode};
use serde_json::{Value, json};

use super::{PortfolioConstraint, PortfolioPack, RebalanceRule};
use crate::config::{ConfigError, ConfigErrorKind};
use crate::pack::{PackError, PackIndex, PackKind, PackState};

fn portfolio() -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "portfolio",
        "version": "1.0.0",
        "name": "single-position-conservative",
        "owner": "operator",
        "payload": {
            "sizing": {
                "mode": "equity_fraction",
                "fraction": 0.05,
                "quantity_step": 0.0001
            },
            "risk": {
                "per_trade_risk": 0.01,
                "max_exposure": 1.0,
                "max_open_positions": 1
            },
            "drawdown": {
                "limit": 0.15,
                "action": "halt"
            },
            "rebalance": "none",
            "constraints": [
                {"kind": "max_positions_per_instrument", "value": 1},
                {"kind": "min_holding_bars", "value": 1}
            ]
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
fn a_portfolio_pack_parses_and_carries_its_identity() {
    let pack = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    assert_eq!(pack.metadata().kind(), PackKind::Portfolio);
    assert_eq!(pack.metadata().version(), "1.0.0");
    assert_eq!(pack.metadata().name(), "single-position-conservative");
    assert_eq!(pack.metadata().owner(), "operator");
    assert_eq!(
        pack.payload().sizing(),
        SizingMode::EquityFraction {
            fraction: 0.05,
            quantity_step: 0.0001,
        }
    );
    assert_eq!(pack.payload().per_trade_risk(), 0.01);
    assert_eq!(pack.payload().max_exposure(), 1.0);
    assert_eq!(pack.payload().max_open_positions(), 1);
    assert_eq!(pack.payload().drawdown().limit(), Some(0.15));
    assert_eq!(pack.payload().drawdown().action(), DrawdownAction::Halt);
    assert_eq!(pack.payload().rebalance(), RebalanceRule::None);
    assert_eq!(
        pack.payload().constraints(),
        [
            PortfolioConstraint::MaxPositionsPerInstrument(1),
            PortfolioConstraint::MinHoldingBars(1),
        ]
    );
    assert_eq!(pack.state(), PackState::Draft);
    assert!(pack.id().to_wire().starts_with("sha256:"));
    assert!(pack.canonical_text().ends_with('\n'));
    assert_eq!(pack.canonical_bytes(), pack.canonical_text().as_bytes());
}

#[test]
fn a_fixed_notional_pack_parses() {
    let mut value = portfolio();
    value["payload"]["sizing"] = json!({
        "mode": "fixed_notional",
        "notional": 2500.0,
        "quantity_step": 0.001
    });
    let pack = PortfolioPack::parse(&value, "pack.json").expect("the pack is valid");
    assert_eq!(
        pack.payload().sizing(),
        SizingMode::FixedNotional {
            notional: 2500.0,
            quantity_step: 0.001,
        }
    );
}

#[test]
fn a_bar_interval_rebalance_parses() {
    let mut value = portfolio();
    value["payload"]["rebalance"] = json!({"mode": "bar_interval", "bars": 4});
    let pack = PortfolioPack::parse(&value, "pack.json").expect("the pack is valid");
    assert_eq!(
        pack.payload().rebalance(),
        RebalanceRule::BarInterval { bars: 4 }
    );
    assert_eq!(pack.payload().rebalance().bars(), Some(4));
}

#[test]
fn a_rebalance_rule_is_deterministic_and_validated() {
    let mut value = portfolio();
    value["payload"]["rebalance"] = json!({"mode": "bar_interval", "bars": 0});
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("zero bars is invalid"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.rebalance.bars");

    let mut value = portfolio();
    value["payload"]["rebalance"] = json!({"mode": "calendar_month", "bars": 1});
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the mode is unsupported"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.rebalance.mode");

    let mut value = portfolio();
    value["payload"]["rebalance"] = json!({"mode": "bar_interval", "bars": 1, "phase": 0.5});
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the field is unknown"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.rebalance.phase");

    let mut value = portfolio();
    value["payload"]["rebalance"] = json!(["none"]);
    let error =
        document_error(PortfolioPack::parse(&value, "pack.json").expect_err("the type is wrong"));
    assert_eq!(error.kind(), ConfigErrorKind::Type);
    assert_eq!(error.path(), "$.payload.rebalance");
}

#[test]
fn an_absent_rebalance_and_constraints_are_the_defaults() {
    let mut value = portfolio();
    let payload = value["payload"].as_object_mut().expect("an object");
    payload.remove("rebalance");
    payload.remove("constraints");
    let pack = PortfolioPack::parse(&value, "pack.json").expect("the defaults are valid");
    assert_eq!(pack.payload().rebalance(), RebalanceRule::None);
    assert!(pack.payload().constraints().is_empty());
}

#[test]
fn an_unknown_sizing_mode_or_field_is_rejected() {
    let mut value = portfolio();
    value["payload"]["sizing"]["mode"] = json!("volatility_target");
    let error =
        document_error(PortfolioPack::parse(&value, "pack.json").expect_err("the mode is unknown"));
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.sizing.mode");

    let mut value = portfolio();
    value["payload"]["sizing"] = json!({
        "mode": "fixed_notional",
        "fraction": 0.05,
        "quantity_step": 0.0001
    });
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the field is unknown"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.sizing.fraction");
}

#[test]
fn sizing_values_must_be_positive_and_bounded() {
    let cases = [
        (
            json!({"mode": "fixed_notional", "notional": 0.0, "quantity_step": 0.0001}),
            "$.payload.sizing.notional",
        ),
        (
            json!({"mode": "fixed_notional", "notional": 2500.0, "quantity_step": 0.0}),
            "$.payload.sizing.quantity_step",
        ),
        (
            json!({"mode": "equity_fraction", "fraction": 0.0, "quantity_step": 0.0001}),
            "$.payload.sizing.fraction",
        ),
        (
            json!({"mode": "equity_fraction", "fraction": 1.5, "quantity_step": 0.0001}),
            "$.payload.sizing.fraction",
        ),
        (
            json!({"mode": "equity_fraction", "fraction": 0.05, "quantity_step": 0.0}),
            "$.payload.sizing.quantity_step",
        ),
    ];
    for (sizing, path) in cases {
        let mut value = portfolio();
        value["payload"]["sizing"] = sizing;
        let error = document_error(
            PortfolioPack::parse(&value, "pack.json").expect_err("the sizing is invalid"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::Range);
        assert_eq!(error.path(), path);
    }
}

#[test]
fn risk_values_are_bounded() {
    for (field, invalid) in [
        ("per_trade_risk", json!(0.0)),
        ("per_trade_risk", json!(1.5)),
        ("max_exposure", json!(0.0)),
        ("max_exposure", json!(1.5)),
    ] {
        let mut value = portfolio();
        value["payload"]["risk"][field] = invalid;
        let error = document_error(
            PortfolioPack::parse(&value, "pack.json").expect_err("the risk is out of range"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::Range);
        assert_eq!(error.path(), format!("$.payload.risk.{field}"));
    }

    let mut value = portfolio();
    value["payload"]["risk"]["max_open_positions"] = json!(0);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("zero positions is invalid"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.risk.max_open_positions");

    let mut value = portfolio();
    value["payload"]["risk"]["max_open_positions"] = json!(1.5);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("a fraction is not a count"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Type);
    assert_eq!(error.path(), "$.payload.risk.max_open_positions");

    let mut value = portfolio();
    value["payload"]["risk"]["max_open_positions"] = json!(4_294_967_296u64);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the count exceeds u32"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.risk.max_open_positions");

    let mut value = portfolio();
    value["payload"]["risk"]["per_trade_risk"] = json!(1.0);
    value["payload"]["risk"]["max_exposure"] = json!(1.0);
    value["payload"]["drawdown"]["limit"] = json!(1.0);
    value["payload"]["sizing"] = json!({
        "mode": "equity_fraction",
        "fraction": 1.0,
        "quantity_step": 0.0001
    });
    PortfolioPack::parse(&value, "pack.json").expect("the inclusive upper bound is valid");
}

#[test]
fn drawdown_limit_and_action_are_validated() {
    let mut value = portfolio();
    value["payload"]["drawdown"]["limit"] = json!(0.0);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the limit is out of range"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.drawdown.limit");

    let mut value = portfolio();
    value["payload"]["drawdown"]["action"] = json!("none");
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the action is unsupported"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Unsupported);
    assert_eq!(error.path(), "$.payload.drawdown.action");
}

#[test]
fn a_constraint_must_be_unique_and_ascending() {
    let mut value = portfolio();
    value["payload"]["constraints"] = json!([
        {"kind": "max_positions_per_instrument", "value": 1},
        {"kind": "max_positions_per_instrument", "value": 1}
    ]);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the constraint repeats"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.constraints[1].kind");

    let mut value = portfolio();
    value["payload"]["constraints"] = json!([
        {"kind": "min_holding_bars", "value": 1},
        {"kind": "max_positions_per_instrument", "value": 1}
    ]);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the order has no canonical form"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.constraints[1].kind");
}

#[test]
fn an_unknown_or_invalid_constraint_is_rejected() {
    let mut value = portfolio();
    value["payload"]["constraints"] = json!([{"kind": "max_gross_exposure", "value": 1.0}]);
    let error =
        document_error(PortfolioPack::parse(&value, "pack.json").expect_err("the kind is unknown"));
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.constraints[0].kind");

    let mut value = portfolio();
    value["payload"]["constraints"] = json!([
        {"kind": "max_positions_per_instrument", "value": 1, "weight": 0.5}
    ]);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the field is unknown"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.constraints[0].weight");

    let mut value = portfolio();
    value["payload"]["constraints"] = json!([
        {"kind": "max_positions_per_instrument", "value": 0}
    ]);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the value is not positive"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Range);
    assert_eq!(error.path(), "$.payload.constraints[0].value");
}

#[test]
fn a_per_instrument_limit_cannot_exceed_the_position_cap() {
    let mut value = portfolio();
    value["payload"]["constraints"] = json!([
        {"kind": "max_positions_per_instrument", "value": 2}
    ]);
    let error = document_error(
        PortfolioPack::parse(&value, "pack.json").expect_err("the constraints disagree"),
    );
    assert_eq!(error.kind(), ConfigErrorKind::Inconsistent);
    assert_eq!(error.path(), "$.payload.constraints[0].value");
    assert!(error.to_string().contains("max_open_positions"));
}

#[test]
fn account_and_statistical_fields_are_rejected() {
    for field in [
        "initial_cash",
        "currency",
        "leverage",
        "correlation",
        "covariance",
        "optimizer",
        "confidence",
        "sharpe",
    ] {
        let mut value = portfolio();
        value["payload"][field] = json!(1.0);
        let error = document_error(
            PortfolioPack::parse(&value, "pack.json").expect_err("the field is not in the schema"),
        );
        assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
        assert_eq!(error.path(), format!("$.payload.{field}"));
    }

    let mut value = portfolio();
    value["payload"]["risk"]["volatility_target"] = json!(0.1);
    let error =
        document_error(PortfolioPack::parse(&value, "pack.json").expect_err("the risk is fitted"));
    assert_eq!(error.kind(), ConfigErrorKind::UnknownField);
    assert_eq!(error.path(), "$.payload.risk.volatility_target");
}

#[test]
fn the_identity_is_stable_across_key_order_and_whitespace() {
    let left = portfolio();
    let text = serde_json::to_string(&left).expect("the document serialises");
    let pretty = serde_json::to_string_pretty(&left).expect("the document serialises");
    let first = PortfolioPack::parse_json(&text, "pack.json").expect("the pack is valid");
    let second = PortfolioPack::parse_json(&pretty, "pack.json").expect("the pack is valid");
    assert_eq!(first.id(), second.id());
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn parsing_the_same_document_twice_is_byte_identical() {
    let first = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    let second = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    assert_eq!(first.canonical_text(), second.canonical_text());
    assert_eq!(first.id(), second.id());
}

#[test]
fn the_identity_changes_with_the_content() {
    let first = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    let mut changed = portfolio();
    changed["payload"]["risk"]["per_trade_risk"] = json!(0.02);
    let second = PortfolioPack::parse(&changed, "pack.json").expect("the pack is valid");
    assert_ne!(first.id(), second.id());
    assert_ne!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn the_envelope_fields_are_part_of_the_identity() {
    let base = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    for (field, replacement) in [
        ("name", json!("another-name")),
        ("version", json!("1.0.1")),
        ("owner", json!("another-owner")),
    ] {
        let mut changed = portfolio();
        changed[field] = replacement;
        let pack = PortfolioPack::parse(&changed, "pack.json").expect("the pack is valid");
        assert_ne!(base.id(), pack.id(), "{field} must change the identity");
    }
}

#[test]
fn a_stored_canonical_pack_round_trips() {
    let pack = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    let stored = PortfolioPack::from_stored_bytes(pack.canonical_bytes(), "pack.json")
        .expect("the bytes are canonical");
    assert_eq!(stored.id(), pack.id());
    assert_eq!(stored.canonical_bytes(), pack.canonical_bytes());
    assert_eq!(stored.metadata(), pack.metadata());
}

#[test]
fn a_non_canonical_store_is_rejected() {
    let document = portfolio();
    let pretty = serde_json::to_string_pretty(&document).expect("the document serialises");
    let error = PortfolioPack::from_stored_bytes(pretty.as_bytes(), "pack.json")
        .expect_err("pretty JSON is not canonical");
    assert!(matches!(error, PackError::NonCanonical { .. }));
}

#[test]
fn non_utf8_and_invalid_json_are_rejected() {
    let error = PortfolioPack::from_stored_bytes(&[0xff, 0xfe], "pack.json")
        .expect_err("invalid UTF-8 is not a pack");
    assert_eq!(document_error(error).kind(), ConfigErrorKind::InvalidJson);

    let error = PortfolioPack::parse_json("{", "pack.json").expect_err("invalid JSON");
    assert_eq!(document_error(error).kind(), ConfigErrorKind::InvalidJson);
}

#[test]
fn a_wrong_pack_kind_is_rejected() {
    let mut value = portfolio();
    value["kind"] = json!("strategy");
    let error =
        PortfolioPack::parse(&value, "pack.json").expect_err("the parser is for portfolio packs");
    assert!(matches!(
        error,
        PackError::WrongKind {
            expected: PackKind::Portfolio,
            found: PackKind::Strategy
        }
    ));
}

#[test]
fn a_missing_required_payload_field_is_rejected() {
    let mut value = portfolio();
    value["payload"]
        .as_object_mut()
        .expect("an object")
        .remove("risk");
    let error =
        document_error(PortfolioPack::parse(&value, "pack.json").expect_err("the risk is missing"));
    assert_eq!(error.kind(), ConfigErrorKind::MissingField);
    assert_eq!(error.path(), "$.payload.risk");
}

#[test]
fn a_draft_freezes_without_changing_its_bytes() {
    let mut pack = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
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
    let mut pack = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    assert!(pack.freeze().is_err(), "a draft cannot freeze");
    pack.validate().expect("draft validates");
    assert!(pack.validate().is_err(), "validated cannot revalidate");
    assert!(pack.retire().is_err(), "validated cannot retire");
    pack.freeze().expect("validated freezes");
    assert!(pack.validate().is_err(), "frozen cannot revalidate");
    assert!(pack.reject().is_err(), "frozen cannot return to draft");
}

#[test]
fn the_identity_index_accepts_a_portfolio_pack() {
    let pack = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    let mut index = PackIndex::new();
    index.register(&pack).expect("the first registration");
    index
        .register(&pack)
        .expect("the same bytes are idempotent");
    assert_eq!(index.len(), 1);
    assert_eq!(
        index.resolve("single-position-conservative", "1.0.0"),
        Some(pack.id())
    );
}

#[test]
fn a_portfolio_label_with_different_bytes_is_rejected() {
    let first = PortfolioPack::parse(&portfolio(), "pack.json").expect("the pack is valid");
    let mut changed = portfolio();
    changed["payload"]["risk"]["max_exposure"] = json!(0.5);
    let second = PortfolioPack::parse(&changed, "pack.json").expect("the pack is valid");
    let mut index = PackIndex::new();
    index.register(&first).expect("the first registration");
    let conflict = index
        .register(&second)
        .expect_err("the same label must not change bytes");
    assert_eq!(conflict.name(), "single-position-conservative");
    assert_eq!(conflict.version(), "1.0.0");
    assert_eq!(conflict.existing(), first.id());
    assert_eq!(conflict.incoming(), second.id());
}
