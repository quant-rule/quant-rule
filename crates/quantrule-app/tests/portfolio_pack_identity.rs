//! The public portfolio-policy-pack seam.
//!
//! This test uses only the exported `quantrule_app::pack` API, so a service or
//! later registry issue can depend on the same behaviour without reaching into
//! the parser internals. The fixture is canonical JSON; its hash is a golden
//! value so a canonicalisation change cannot pass silently.

use quantrule_app::pack::{
    PackError, PackIndex, PackState, PortfolioConstraint, PortfolioPack, RebalanceRule,
};

/// The canonical fixture bytes, including the trailing newline.
const FIXTURE: &str = "{\"kind\":\"portfolio\",\"name\":\"single-position-conservative\",\"owner\":\"operator\",\"payload\":{\"constraints\":[{\"kind\":\"max_positions_per_instrument\",\"value\":1},{\"kind\":\"min_holding_bars\",\"value\":1}],\"drawdown\":{\"action\":\"halt\",\"limit\":0.15},\"rebalance\":\"none\",\"risk\":{\"max_exposure\":1.0,\"max_open_positions\":1,\"per_trade_risk\":0.01},\"sizing\":{\"fraction\":0.05,\"mode\":\"equity_fraction\",\"quantity_step\":0.0001}},\"schema\":\"quantrule.pack/v1\",\"version\":\"1.0.0\"}\n";

const FIXTURE_ID: &str = "sha256:4cae9bd74fed0267c331342defe8f159bb3ac9ec0e888b85e4d3d1486d49c0c9";

fn changed_per_trade_risk() -> String {
    FIXTURE.replace("\"per_trade_risk\":0.01", "\"per_trade_risk\":0.02")
}

#[test]
fn the_canonical_fixture_has_the_expected_content_hash() {
    let pack = PortfolioPack::from_stored_bytes(FIXTURE.as_bytes(), "pack.json")
        .expect("the canonical fixture is a pack");
    assert_eq!(pack.canonical_text(), FIXTURE);
    assert_eq!(pack.id().to_wire(), FIXTURE_ID);
    assert_eq!(pack.state(), PackState::Draft);
    assert_eq!(pack.payload().max_open_positions(), 1);
    assert_eq!(pack.payload().rebalance(), RebalanceRule::None);
    assert_eq!(
        pack.payload().constraints(),
        [
            PortfolioConstraint::MaxPositionsPerInstrument(1),
            PortfolioConstraint::MinHoldingBars(1),
        ]
    );
}

#[test]
fn equivalent_json_reaches_the_same_identity() {
    let reordered = FIXTURE.replace(
        "\"kind\":\"portfolio\",\"name\":\"single-position-conservative\",\"owner\":\"operator\"",
        "\"owner\":\"operator\",\"name\":\"single-position-conservative\",\"kind\":\"portfolio\"",
    );
    assert_ne!(reordered, FIXTURE, "the fixture must actually be reordered");
    let pack = PortfolioPack::parse_json(&reordered, "pack.json").expect("the pack is valid");
    assert_eq!(pack.canonical_text(), FIXTURE);
    assert_eq!(pack.id().to_wire(), FIXTURE_ID);
}

#[test]
fn repeated_serialization_is_byte_identical() {
    let first = PortfolioPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    let second = PortfolioPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    assert_eq!(first.id(), second.id());
}

#[test]
fn a_label_cannot_be_reused_with_different_bytes() {
    let first = PortfolioPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    let second = PortfolioPack::parse_json(&changed_per_trade_risk(), "pack.json")
        .expect("the pack is valid");
    assert_ne!(first.id(), second.id());

    let mut index = PackIndex::new();
    index.register(&first).expect("the first registration");
    index
        .register(&first)
        .expect("the same bytes are idempotent");
    let conflict = index
        .register(&second)
        .expect_err("the label already names different bytes");
    assert_eq!(conflict.name(), "single-position-conservative");
    assert_eq!(conflict.version(), "1.0.0");
    assert_eq!(conflict.existing(), first.id());
    assert_eq!(conflict.incoming(), second.id());
}

#[test]
fn a_frozen_pack_keeps_its_bytes_and_leaves_retirement_readable() {
    let mut pack = PortfolioPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    let id = pack.id().clone();
    pack.validate().expect("the draft validates");
    pack.freeze().expect("the validated pack freezes");
    assert_eq!(pack.state(), PackState::Frozen);
    assert_eq!(pack.id(), &id);
    assert_eq!(pack.canonical_text(), FIXTURE);
    pack.retire().expect("the frozen pack retires");
    assert_eq!(pack.state(), PackState::Retired);
    assert_eq!(pack.id(), &id);
    assert_eq!(pack.canonical_text(), FIXTURE);
}

#[test]
fn a_wrong_kind_is_not_parsed_as_a_portfolio_pack() {
    let strategy = FIXTURE.replace("\"kind\":\"portfolio\"", "\"kind\":\"strategy\"");
    assert!(matches!(
        PortfolioPack::parse_json(&strategy, "pack.json"),
        Err(PackError::WrongKind { .. })
    ));
}
