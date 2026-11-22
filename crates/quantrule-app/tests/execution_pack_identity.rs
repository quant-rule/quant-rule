//! The public execution-simulation-pack seam.
//!
//! This test uses only the exported `quantrule_app::pack` API, so a service or
//! later registry issue can depend on the same behaviour without reaching into
//! the parser internals. The fixture is canonical JSON; its hash is a golden
//! value so a canonicalisation change cannot pass silently.

use quantrule_app::pack::{
    EndOfDataPolicy, ExecutionPack, OrderType, PackError, PackIndex, PackState, TieBreak,
    ZeroVolumePolicy,
};
use quantrule_data::Timeframe;

/// The canonical fixture bytes, including the trailing newline.
const FIXTURE: &str = "{\"kind\":\"execution\",\"name\":\"btcusd-conservative-fills\",\"owner\":\"operator\",\"payload\":{\"commission\":{\"base\":\"equity\",\"per\":\"operation\",\"rate\":0.0002},\"end_of_data\":\"leave_open\",\"fee_rate\":0.0,\"fill_timeframe\":\"5m\",\"funding\":{\"interval_hours\":8,\"rate\":0.0},\"order_type\":\"market_next_open\",\"slippage_bps\":2.0,\"tie_break\":\"stop_first\",\"zero_volume\":\"reject\"},\"schema\":\"quantrule.pack/v1\",\"version\":\"1.0.0\"}\n";

const FIXTURE_ID: &str = "sha256:b526f41af8ba7c8d1699911c2f214a6a97fd611e14173d6cc3f52d4c4d7469be";

fn changed_slippage() -> String {
    FIXTURE.replace("\"slippage_bps\":2.0", "\"slippage_bps\":3.0")
}

#[test]
fn the_canonical_fixture_has_the_expected_content_hash() {
    let pack = ExecutionPack::from_stored_bytes(FIXTURE.as_bytes(), "pack.json")
        .expect("the canonical fixture is a pack");
    assert_eq!(pack.canonical_text(), FIXTURE);
    assert_eq!(pack.id().to_wire(), FIXTURE_ID);
    assert_eq!(pack.state(), PackState::Draft);
    assert_eq!(pack.payload().fill_timeframe(), Timeframe::M5);
    assert_eq!(pack.payload().order_type(), OrderType::MarketNextOpen);
    assert_eq!(pack.payload().tie_break(), TieBreak::StopFirst);
    assert_eq!(pack.payload().fee_rate(), 0.0);
    assert_eq!(pack.payload().slippage_bps(), 2.0);
    assert_eq!(pack.payload().commission().rate(), 0.0002);
    assert_eq!(pack.payload().funding().interval().hours(), 8);
    assert_eq!(pack.payload().zero_volume(), ZeroVolumePolicy::Reject);
    assert_eq!(pack.payload().end_of_data(), EndOfDataPolicy::LeaveOpen);
}

#[test]
fn equivalent_json_reaches_the_same_identity() {
    let reordered = FIXTURE.replace(
        "\"kind\":\"execution\",\"name\":\"btcusd-conservative-fills\",\"owner\":\"operator\"",
        "\"owner\":\"operator\",\"name\":\"btcusd-conservative-fills\",\"kind\":\"execution\"",
    );
    assert_ne!(reordered, FIXTURE, "the fixture must actually be reordered");
    let pack = ExecutionPack::parse_json(&reordered, "pack.json").expect("the pack is valid");
    assert_eq!(pack.canonical_text(), FIXTURE);
    assert_eq!(pack.id().to_wire(), FIXTURE_ID);
}

#[test]
fn repeated_serialization_is_byte_identical() {
    let first = ExecutionPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    let second = ExecutionPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    assert_eq!(first.id(), second.id());
}

#[test]
fn a_label_cannot_be_reused_with_different_bytes() {
    let first = ExecutionPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
    let second =
        ExecutionPack::parse_json(&changed_slippage(), "pack.json").expect("the pack is valid");
    assert_ne!(first.id(), second.id());

    let mut index = PackIndex::new();
    index.register(&first).expect("the first registration");
    index
        .register(&first)
        .expect("the same bytes are idempotent");
    let conflict = index
        .register(&second)
        .expect_err("the label already names different bytes");
    assert_eq!(conflict.name(), "btcusd-conservative-fills");
    assert_eq!(conflict.version(), "1.0.0");
    assert_eq!(conflict.existing(), first.id());
    assert_eq!(conflict.incoming(), second.id());
}

#[test]
fn a_frozen_pack_keeps_its_bytes_and_leaves_retirement_readable() {
    let mut pack = ExecutionPack::parse_json(FIXTURE, "pack.json").expect("the pack is valid");
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
fn a_wrong_kind_is_not_parsed_as_an_execution_pack() {
    let portfolio = FIXTURE.replace("\"kind\":\"execution\"", "\"kind\":\"portfolio\"");
    assert!(matches!(
        ExecutionPack::parse_json(&portfolio, "pack.json"),
        Err(PackError::WrongKind { .. })
    ));
}
