//! Shell error, determinism, JSON, and help output tests.

use crate::cases::assertions::{assert_failure, assert_success, stderr, stdout};
use crate::support::{Fixture, canonical_digest, digest, run_id};

#[test]
fn the_error_envelope_keeps_its_stable_code() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "status", &digest('9')]);
    assert_failure(&output);
    assert_eq!(stderr(&output), "error: run.not_found: run is unknown\n");
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_parse_error_names_the_usage_without_a_request() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "list", "--limit", "0"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("--limit must be between 1 and 200"));
    assert!(fixture.requests().is_empty());
}

#[test]
fn a_missing_required_option_is_refused_without_a_request() {
    let fixture = Fixture::start();
    let output = fixture.run(&["simulation", "create", "--premise", "p"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("--bundle is required"));
    assert!(fixture.requests().is_empty());
}

#[test]
fn the_same_command_prints_the_same_bytes() {
    let fixture = Fixture::start();
    let first = fixture.run(&["run", "status", &run_id()]);
    let second = fixture.run(&["run", "status", &run_id()]);
    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(fixture.requests().len(), 2);
}

#[test]
fn interactive_and_non_interactive_output_are_identical() {
    let fixture = Fixture::start();
    let non_interactive = fixture.run(&["dataset", "list"]);
    let interactive = fixture.run_interactive("dataset list\nquit\n");
    assert_success(&non_interactive);
    assert_success(&interactive);
    assert_eq!(interactive.stdout, non_interactive.stdout);
}

#[test]
fn json_output_matches_the_gateway_value() {
    let fixture = Fixture::start();
    let output = fixture.run(&["--json", "run", "status", &run_id()]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the output is one JSON value");
    assert_eq!(value["run_id"], run_id());
    assert_eq!(value["provenance"]["model_version"], "M1.0");
    assert_eq!(value["digest"], canonical_digest());
}

#[test]
fn help_does_not_need_a_gateway() {
    let fixture = Fixture::start();
    let output = fixture.run(&["help", "run", "submit"]);
    assert_success(&output);
    assert!(stdout(&output).starts_with("run submit\n"));
    assert!(
        stdout(&output)
            .contains("usage:  run submit <data_version> <config_id> [--model-artifact <id>]")
    );
    assert!(fixture.requests().is_empty());
}

#[test]
fn help_survives_a_malformed_token() {
    let fixture = Fixture::start();
    let output = fixture.run_with_env(&["help", "run", "submit"], "QUANTRULE_TOKEN", "not a token");
    assert_success(&output);
    assert!(stdout(&output).starts_with("run submit\n"));
    assert!(fixture.requests().is_empty());
}

#[test]
fn a_retired_stage_command_is_unknown() {
    let fixture = Fixture::start();
    let output = fixture.run(&["backtest", "run", "--config", "x"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("unknown command 'backtest'"));
    assert!(fixture.requests().is_empty());
}
