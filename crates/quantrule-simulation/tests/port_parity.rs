//! Port-parity comparison for PORT-001.
//!
//! The fixture is captured from the Python simulation harness at commit
//! `0814a129`. Every case names the stage and the SIM run it exercises; this
//! test runs the Rust port on the case input and compares every expected value,
//! including every number, to the fixture tolerance.
//!
//! The shared stage inputs live in `PORT-001.datasets.json` beside the fixture:
//! the real 2018-2019 label series, the 2019-12-31 bridge and 2020-2022 window,
//! and the event catalog. The fixture comments the projection of each case.

// port-parity test: PORT-001
const FIXTURE: &str = "docs/port-parity/PORT-001.golden.json";
const DATASETS: &str = "docs/port-parity/PORT-001.datasets.json";

#[path = "port_parity/support.rs"]
mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use support::{Case, Datasets, Fixture, assert_matches, reorder, run_stage};

fn repo_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn load_fixture() -> Fixture {
    let text = std::fs::read_to_string(repo_path(FIXTURE)).expect("read fixture");
    serde_json::from_str(&text).expect("parse fixture")
}

fn load_datasets(fixture: &Fixture) -> Datasets {
    let text = std::fs::read_to_string(repo_path(&fixture.datasets_file))
        .expect("read the shared datasets");
    serde_json::from_str(&text).expect("parse the shared datasets")
}

fn run(case: &Case, datasets: &Datasets) -> Value {
    let catalog = datasets.catalog();
    run_stage(&case.stage, &case.input, datasets, &catalog)
}

#[test]
fn the_fixture_envelope_is_valid() {
    let fixture = load_fixture();
    assert_eq!(fixture.schema, "quantrule.port-parity-fixture/v1");
    assert_eq!(fixture.id, "PORT-001");
    assert!(fixture.source.contains("0814a129"), "{}", fixture.source);
    assert_eq!(fixture.captured, "2026-11-14");
    assert!(fixture.tolerance > 0.0 && fixture.tolerance <= 1e-9);
    assert!(!fixture.notes.is_empty());
    assert_eq!(fixture.datasets_file, DATASETS);
    assert!(!fixture.cases.is_empty());
    let datasets = load_datasets(&fixture);
    assert_eq!(datasets.schema, "quantrule.port-parity-datasets/v1");
    assert!(!datasets.catalog.is_empty());
    assert!(!datasets.label_series.rows.is_empty());
    assert!(!datasets.tail_bars.rows.is_empty());
    let mut names = BTreeSet::new();
    for case in &fixture.cases {
        assert!(!case.name.is_empty());
        assert!(
            names.insert(case.name.clone()),
            "duplicate case {}",
            case.name
        );
        assert!(!case.notes.is_empty(), "{} needs a note", case.name);
        assert!(
            matches!(
                case.stage.as_str(),
                "window" | "marks" | "oracle" | "review" | "check" | "meta" | "exit"
            ),
            "{} has an unknown stage {}",
            case.name,
            case.stage
        );
        assert!(!case.sim.is_empty(), "{} names no SIM run", case.name);
    }
}

#[test]
fn the_fixture_covers_every_sim_run() {
    let fixture = load_fixture();
    let covered: BTreeSet<&str> = fixture
        .cases
        .iter()
        .flat_map(|case| case.sim.iter().map(String::as_str))
        .collect();
    for id in [
        "SIM-01", "SIM-02", "SIM-03", "SIM-04", "SIM-05", "SIM-06", "SIM-07", "SIM-08", "SIM-09",
        "SIM-10",
    ] {
        assert!(covered.contains(id), "the fixture does not cover {id}");
    }
}

#[test]
fn the_fixture_covers_the_named_edge_cases() {
    let fixture = load_fixture();
    let names: BTreeSet<&str> = fixture
        .cases
        .iter()
        .map(|case| case.name.as_str())
        .collect();
    for name in [
        "window-single-bar",
        "oracle-single-bar",
        "oracle-equal-excursion",
        "oracle-target-and-stop-same-bar",
        "check-unmarked",
        "marks-empty",
        "review-empty",
        "sim-10-blocked-windows",
        "window-real-scan",
        "check-real-scan",
    ] {
        assert!(names.contains(name), "the fixture is missing {name}");
    }
}

#[test]
fn every_case_matches_the_python_reference() {
    let fixture = load_fixture();
    let datasets = load_datasets(&fixture);
    for case in &fixture.cases {
        let actual = run(case, &datasets);
        assert_matches(
            &case.expected,
            &actual,
            fixture.tolerance,
            &format!("{} ({})", case.name, case.stage),
        );
    }
}

#[test]
fn a_reordered_input_produces_the_same_output() {
    let fixture = load_fixture();
    let datasets = load_datasets(&fixture);
    let catalog = datasets.catalog();
    for case in &fixture.cases {
        match case.stage.as_str() {
            "window" | "oracle" | "review" | "marks" => {
                let first = run_stage(&case.stage, &case.input, &datasets, &catalog);
                let reordered = reorder(&case.stage, &case.input);
                let second = run_stage(&case.stage, &reordered, &datasets, &catalog);
                assert_eq!(first, second, "{} is not order-independent", case.name);
            }
            "meta" => {
                let first = run_stage(&case.stage, &case.input, &datasets, &catalog);
                let reordered = reorder(&case.stage, &case.input);
                let second = run_stage(&case.stage, &reordered, &datasets, &catalog);
                for key in ["unmatched", "cells", "candidate", "check"] {
                    assert_eq!(
                        &first[key], &second[key],
                        "{} changed {key} when the marks were reordered",
                        case.name
                    );
                }
                let mut first_rows = first["rows"].as_array().expect("rows").clone();
                let mut second_rows = second["rows"].as_array().expect("rows").clone();
                let sort = |rows: &mut Vec<Value>| {
                    rows.sort_by_key(|row| {
                        (
                            row["bar"].as_u64().unwrap_or(0),
                            row["side"].as_str().unwrap_or("").to_owned(),
                            row["chart"].as_str().unwrap_or("").to_owned(),
                        )
                    });
                };
                sort(&mut first_rows);
                sort(&mut second_rows);
                assert_eq!(
                    first_rows, second_rows,
                    "{} changed the labelled row set when the marks were reordered",
                    case.name
                );
            }
            // The check and exit rows are a time sequence; reversing them is a
            // different input, so their determinism is the repeat test below.
            _ => {}
        }
    }
}

#[test]
fn the_same_case_is_deterministic() {
    let fixture = load_fixture();
    let datasets = load_datasets(&fixture);
    for case in &fixture.cases {
        let first = run(case, &datasets);
        let second = run(case, &datasets);
        assert_eq!(first, second, "{} changed between runs", case.name);
    }
}
