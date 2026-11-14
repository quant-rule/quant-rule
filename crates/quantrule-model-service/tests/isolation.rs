//! The model service boundary.
//!
//! The service may read the data service and the object store. It may not link
//! the strategy, execution, or portfolio engine, open a database, or read a run,
//! catalog, or research schema. The manifest materializes the run config to
//! extract its model inputs, but it never evaluates a strategy and never exposes
//! a strategy, execution, or portfolio value.

use std::path::{Path, PathBuf};

fn sources(root: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn the_service_links_no_strategy_engine_and_no_database() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::fs::read_to_string(manifest.join("Cargo.toml")).expect("manifest");
    for forbidden in ["quantrule-backtest", "sqlx", "quantrule-server"] {
        assert!(!cargo.contains(forbidden), "Cargo.toml names {forbidden}");
    }

    let mut files = Vec::new();
    sources(&manifest.join("src"), &mut files);
    assert!(!files.is_empty(), "no sources found");
    for file in files {
        let relative = file
            .strip_prefix(&manifest)
            .expect("relative")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&file).expect("source");
        for forbidden in [
            "quantrule_backtest",
            "ExecutionEngine",
            "PortfolioLedger",
            "RunStore",
            "sqlx",
            "run_backtest",
            "strategy::",
            "execution::",
            "portfolio::",
            "catalog.",
            "runs.",
            "research.",
        ] {
            assert!(
                !text.contains(forbidden),
                "{relative} still names {forbidden}"
            );
        }
    }
}
