//! Architecture guards for the shell crate.
//!
//! The shell is a gateway client. These tests protect the one-way transport
//! edge, the three-dependency budget, and the absence of the retired local
//! pipeline.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn crates_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the shell crate lives under crates/")
        .to_path_buf()
}

fn shell_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn source(directory: &str) -> Vec<(PathBuf, String)> {
    let root = shell_root().join("src").join(directory);
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

/// Every workspace crate and the workspace crates it may depend on.
///
/// The order is the dependency order, so a crate may only name crates above
/// it. The shell reaches the product only through the transport contract.
const ALLOWED_EDGES: [(&str, &[&str]); 6] = [
    ("quantrule-api", &[]),
    ("quantrule-data", &[]),
    ("quantrule-model", &["quantrule-data"]),
    ("quantrule-backtest", &["quantrule-data", "quantrule-model"]),
    (
        "quantrule-app",
        &["quantrule-backtest", "quantrule-data", "quantrule-model"],
    ),
    ("quantrule-shell", &["quantrule-api"]),
];

fn manifest(crate_name: &str) -> String {
    let path = crates_root().join(crate_name).join("Cargo.toml");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// The keys of one manifest table.
fn table_keys(manifest: &str, table: &str) -> Vec<String> {
    let mut in_table = false;
    let mut keys = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_table = line == table;
            continue;
        }
        if !in_table || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            keys.push(key.trim().to_owned());
        }
    }
    keys
}

fn workspace_graph() -> BTreeMap<String, Vec<String>> {
    ALLOWED_EDGES
        .iter()
        .map(|(name, _)| {
            let text = manifest(name);
            let dependencies = table_keys(&text, "[dependencies]")
                .into_iter()
                .filter(|key| key.starts_with("quantrule-"))
                .collect();
            ((*name).to_owned(), dependencies)
        })
        .collect()
}

#[test]
fn the_workspace_graph_is_acyclic_and_directional() {
    let graph = workspace_graph();
    for (crate_name, allowed) in ALLOWED_EDGES {
        let actual = graph.get(crate_name).expect("the crate is in the graph");
        for dependency in actual {
            assert!(
                allowed.contains(&dependency.as_str()),
                "{crate_name} must not depend on {dependency}"
            );
        }
    }
}

#[test]
fn the_shell_depends_only_on_the_contract_and_two_support_crates() {
    let text = manifest("quantrule-shell");
    let dependencies: BTreeSet<String> = table_keys(&text, "[dependencies]").into_iter().collect();
    let expected: BTreeSet<String> = ["quantrule-api", "serde_json", "ureq"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        dependencies, expected,
        "the shell product dependency set changed"
    );
    assert!(
        table_keys(&text, "[dev-dependencies]").is_empty(),
        "the shell must not add a dev-dependency; tests use std"
    );
}

#[test]
fn the_shell_never_names_an_engine_or_application_crate() {
    for (path, text) in source("") {
        for forbidden in [
            "quantrule_app",
            "quantrule_data",
            "quantrule_model",
            "quantrule_backtest",
            "quantrule-research",
            "quantrule-simulation",
        ] {
            assert!(
                !text.contains(forbidden),
                "{} must not name {forbidden}",
                path.display()
            );
        }
    }
}

#[test]
fn the_shell_has_no_local_source_or_database_module() {
    let crate_root = shell_root();
    for retired in [
        "src/source",
        "src/fetch.rs",
        "src/export.rs",
        "src/render.rs",
        "src/render_flat.rs",
        "src/request.rs",
        "src/command/memory.rs",
        "src/command/backtest.rs",
        "src/command/analytics.rs",
    ] {
        assert!(
            !crate_root.join(retired).exists(),
            "{retired} was retired with the local surface"
        );
    }
}

#[test]
fn only_the_transport_module_uses_ureq() {
    for (path, text) in source("command") {
        assert!(
            !text.contains("ureq::"),
            "{} must call the client, not ureq",
            path.display()
        );
    }
    let transport = fs::read_to_string(shell_root().join("src/client.rs"))
        .expect("the transport module exists");
    assert!(transport.contains("ureq::Agent"));
}

#[test]
fn the_binary_forbids_unsafe_code() {
    let main = fs::read_to_string(shell_root().join("src/main.rs")).expect("main.rs exists");
    assert!(main.contains("#![forbid(unsafe_code)]"));
}

#[test]
fn renderers_do_not_depend_on_the_command_module() {
    let text = fs::read_to_string(shell_root().join("src/output.rs")).expect("output.rs exists");
    assert!(
        !text.contains("use crate::command"),
        "output.rs must not import the command layer"
    );
}

#[test]
fn the_shell_does_not_orchestrate_pipeline_stages() {
    for (path, text) in source("") {
        for forbidden in [
            "pipeline::run_model",
            "ExecutionEngine::new",
            "PortfolioLedger::new",
            "SpecRun::compute",
            "rusqlite",
            "std::net::TcpListener",
        ] {
            assert!(
                !text.contains(forbidden),
                "{} must not carry a local engine: {forbidden}",
                path.display()
            );
        }
    }
}
