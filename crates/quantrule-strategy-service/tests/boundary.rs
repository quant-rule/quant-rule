//! The strategy service boundary.
//!
//! The service may read packs and register artifacts through the data service,
//! ask the model service for a frozen bundle, and read and write the object
//! store. It may not open a database, consume a queue, import run, research,
//! execution or portfolio code, or expose a public route.

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

fn violations(text: &str) -> Vec<&'static str> {
    [
        "sqlx",
        "quantrule_backtest",
        "quantrule_research",
        "quantrule_evidence_service",
        "quantrule_simulation_service",
        "ExecutionEngine",
        "PortfolioLedger",
        "RunStore",
        "RunManifest",
        "canonical_digest",
        "run_backtest",
        "Queue",
        "runs.",
        "research.",
        "catalog.",
    ]
    .into_iter()
    .filter(|name| text.contains(name))
    .collect()
}

#[test]
fn the_service_links_no_database_queue_or_run_code() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cargo = std::fs::read_to_string(manifest.join("Cargo.toml")).expect("manifest");
    for forbidden in [
        "sqlx",
        "quantrule-backtest-service",
        "quantrule-evidence-service",
        "quantrule-simulation-service",
        "quantrule-gateway",
        "quantrule-server",
    ] {
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
        assert!(
            violations(&text).is_empty(),
            "{relative} still names {:?}",
            violations(&text)
        );
    }
}

#[test]
fn the_service_has_no_public_alias_and_no_migration_path() {
    let source =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/routes.rs"))
            .expect("routes");
    assert!(!source.contains(".nest(\"/api/v1\""));
    assert!(source.contains(".route(\"/internal/v1/signals\", post(compute_signal))"));
    let main =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
            .expect("main");
    for forbidden in ["migrate_only_requested", "migrate_owner_database", "Queue"] {
        assert!(!main.contains(forbidden), "main.rs names {forbidden}");
    }
}

#[test]
fn a_seeded_database_import_fails_the_guard() {
    let seeded = "pub async fn read() { sqlx::query(\"SELECT 1\"); }";
    assert_eq!(violations(seeded), vec!["sqlx"]);
}
