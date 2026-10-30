//! Boundary guards for the backtest module graph.
//!
//! Execution and portfolio are below strategy in the decision stack. They must
//! not import it, or a strategy change becomes an execution/accounting change.
//! The guards are source-level because Rust has no stable negative-import test.

use std::fs;
use std::path::{Path, PathBuf};

fn sources(directory: &str) -> Vec<(PathBuf, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(directory);
    let mut files = Vec::new();
    collect(&root, &mut files);
    files
}

fn collect(path: &Path, files: &mut Vec<(PathBuf, String)>) {
    let entries =
        fs::read_dir(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    for entry in entries {
        let entry =
            entry.unwrap_or_else(|error| panic!("read entry in {}: {error}", path.display()));
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            files.push((path, text));
        }
    }
}

#[test]
fn execution_and_portfolio_do_not_import_strategy() {
    for directory in ["execution", "portfolio"] {
        for (path, source) in sources(directory) {
            assert!(
                !source.contains("use crate::strategy"),
                "{} must not import the strategy module",
                path.display()
            );
        }
    }
}

#[test]
fn the_strategy_does_not_import_execution_or_portfolio() {
    for (path, source) in sources("strategy") {
        for forbidden in ["use crate::execution", "use crate::portfolio"] {
            assert!(
                !source.contains(forbidden),
                "{} must not import the execution or portfolio module",
                path.display()
            );
        }
    }
}

#[test]
fn the_execution_engine_does_not_accept_a_strategy_decision() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("execution")
        .join("engine.rs");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    assert!(
        !source.contains("Decision::") && !source.contains(": Decision"),
        "{} must use the execution-owned OrderIntent, not a strategy Decision",
        path.display()
    );
}
