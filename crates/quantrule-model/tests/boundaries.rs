//! Boundary guards for the model module graph.
//!
//! The guards are source-level because Rust has no stable way to assert that a
//! module does not import another. They protect the one-way edge from the model
//! series to the report projection: the run and timeframe modules must not
//! depend on the report.

use std::fs;
use std::path::PathBuf;

fn source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn the_model_series_does_not_import_the_report() {
    for file in ["run.rs", "timeframe.rs"] {
        let source = source(file);
        assert!(
            !source.contains("use crate::report"),
            "{file} must not import the report; the report projects the series"
        );
    }
}

#[test]
fn the_report_does_not_define_the_input_or_run_contract() {
    let report = source("report/flat.rs");
    for definition in [
        "struct MarketStateConfig",
        "struct TimeframeInput",
        "struct ModelRun",
        "struct TimeframeModel",
    ] {
        assert!(
            !report.contains(definition),
            "the report must project the run only; found {definition}"
        );
    }
}
