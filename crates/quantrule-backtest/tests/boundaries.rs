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

/// The `use` paths in a source file, one per line.
///
/// The parser looks at the declaration, not at every occurrence of the text:
/// a comment or a string that names the module is not an import.
fn imports(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix("use ")?;
            Some(rest.trim_start_matches("::").to_owned())
        })
        .collect()
}

fn imports_prefix(source: &str, prefix: &str) -> bool {
    imports(source)
        .iter()
        .any(|path| path == prefix || path.starts_with(&format!("{prefix}::")))
}

#[test]
fn a_comment_is_not_an_import() {
    assert!(!imports_prefix(
        "// use crate::strategy
",
        "crate::strategy"
    ));
    assert!(imports_prefix(
        "use crate::strategy::{Decision};
",
        "crate::strategy"
    ));
}

#[test]
fn execution_and_portfolio_do_not_import_strategy() {
    for directory in ["execution", "portfolio"] {
        for (path, source) in sources(directory) {
            assert!(
                !imports_prefix(&source, "crate::strategy"),
                "{} must not import the strategy module",
                path.display()
            );
        }
    }
}

#[test]
fn the_strategy_does_not_import_execution_or_portfolio() {
    for (path, source) in sources("strategy") {
        for forbidden in ["crate::execution", "crate::portfolio"] {
            assert!(
                !imports_prefix(&source, forbidden),
                "{} must not import the execution or portfolio module",
                path.display()
            );
        }
    }
}

#[test]
fn the_execution_engine_does_not_accept_a_strategy_decision() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("execution")
        .join("engine");
    let mut paths: Vec<PathBuf> = fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "the engine sources must exist");
    for path in paths {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert!(
            !imports_prefix(&source, "crate::strategy"),
            "{} must use the execution-owned OrderIntent, not a strategy Decision",
            path.display()
        );
    }
}
