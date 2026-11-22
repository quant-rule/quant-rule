//! The public strategy-pack seam.
//!
//! This test uses only the exported `quantrule_app::pack` API, so a service or
//! later registry issue can depend on the same behaviour without reaching into
//! the parser internals. The fixture is canonical JSON; its hash is a golden
//! value so a canonicalisation change cannot pass silently.

use quantrule_app::pack::{PackError, PackIndex, PackState, StrategyPack};

/// The canonical fixture bytes, including the trailing newline.
const FIXTURE: &str = "{\"kind\":\"strategy\",\"name\":\"compression-fade-5m\",\"owner\":\"operator\",\"payload\":{\"context_timeframes\":[\"15m\"],\"engine\":\"event_composite\",\"entry\":[{\"direction\":1,\"event_type\":3,\"kind\":\"event\",\"source\":4,\"subtype\":1,\"timeframe\":\"5m\"}],\"event_window_bars\":1,\"execution_timeframe\":\"5m\",\"exit\":[],\"giveback_arm_atr\":0.0,\"giveback_atr\":null,\"mode\":\"signal\",\"require_contiguous\":true,\"risk\":{\"reward_to_risk\":1.5,\"stop_atr_multiple\":2.0},\"time_stop_bars\":0,\"universe\":[\"BTCUSD\"]},\"schema\":\"quantrule.pack/v1\",\"version\":\"1.0.0\"}\n";

const FIXTURE_ID: &str = "sha256:dcbe8d930f11cdde71ca0c62be61f36abe44d822a3ae5dc5f35b7e8780f20ad6";

fn changed_reward() -> String {
    FIXTURE.replace("\"reward_to_risk\":1.5", "\"reward_to_risk\":2.0")
}

#[test]
fn the_canonical_fixture_has_the_expected_content_hash() {
    let pack = StrategyPack::from_stored_bytes(FIXTURE.as_bytes(), "pack.json")
        .expect("the canonical fixture is a pack");
    assert_eq!(pack.canonical_text(), FIXTURE);
    assert_eq!(pack.id().to_wire(), FIXTURE_ID);
    assert_eq!(pack.state(), PackState::Draft);
}

#[test]
fn equivalent_json_reaches_the_same_identity() {
    let reordered = FIXTURE.replace(
        "\"kind\":\"strategy\",\"name\":\"compression-fade-5m\",\"owner\":\"operator\"",
        "\"owner\":\"operator\",\"name\":\"compression-fade-5m\",\"kind\":\"strategy\"",
    );
    assert_ne!(reordered, FIXTURE, "the fixture must actually be reordered");
    let pack = StrategyPack::parse_json(&reordered, "pack.json").expect("the pack is valid");
    assert_eq!(pack.canonical_text(), FIXTURE);
    assert_eq!(pack.id().to_wire(), FIXTURE_ID);
}

#[test]
fn a_label_cannot_be_reused_with_different_bytes() {
    let first = StrategyPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    let second =
        StrategyPack::parse_json(&changed_reward(), "pack.json").expect("the pack is valid");
    assert_ne!(first.id(), second.id());

    let mut index = PackIndex::new();
    index.register(&first).expect("the first registration");
    index
        .register(&first)
        .expect("the same bytes are idempotent");
    let conflict = index
        .register(&second)
        .expect_err("the label already names different bytes");
    assert_eq!(conflict.name(), "compression-fade-5m");
    assert_eq!(conflict.version(), "1.0.0");
    assert_eq!(conflict.existing(), first.id());
    assert_eq!(conflict.incoming(), second.id());
}

#[test]
fn a_frozen_pack_keeps_its_bytes_and_leaves_retirement_readable() {
    let mut pack = StrategyPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
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
fn a_wrong_kind_is_not_parsed_as_a_strategy_pack() {
    let portfolio = FIXTURE.replace("\"kind\":\"strategy\"", "\"kind\":\"portfolio\"");
    assert!(matches!(
        StrategyPack::parse_json(&portfolio, "pack.json"),
        Err(PackError::WrongKind { .. })
    ));
}
