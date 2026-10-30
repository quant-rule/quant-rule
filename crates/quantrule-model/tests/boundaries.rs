//! Boundary guards for the model module graph.
//!
//! The guards are source-level because Rust has no stable way to assert that a
//! module does not import another. They protect the one-way edge that broke the
//! `run`/`state` cycle: the run orchestration must not depend on the view.

use std::fs;
use std::path::PathBuf;

fn source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn run_does_not_import_the_state_view() {
    let run = source("run.rs");
    assert!(
        !run.contains("use crate::state"),
        "run.rs must not import the state view; the state module depends on run, not the reverse"
    );
}

#[test]
fn state_does_not_define_the_input_or_consensus_contract() {
    let state = source("state.rs");
    for definition in [
        "struct MarketStateConfig",
        "struct TimeframeInput",
        "struct TimeframeConsensus",
    ] {
        assert!(
            !state.contains(definition),
            "state.rs must keep the view only; found {definition}"
        );
    }
}
