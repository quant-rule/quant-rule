//! Architecture guards for the shell crate.
//!
//! The shell parses, calls the pipeline, and renders. These tests protect the
//! one-way renderer edges and the workspace crate graph.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn source(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

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
fn renderers_do_not_depend_on_the_command_module() {
    let files: Vec<(String, String)> = ["render.rs", "render_flat.rs"]
        .into_iter()
        .map(|file| (file.to_owned(), source(file)))
        .collect();
    for (name, text) in files {
        assert!(
            !text.contains("use crate::command"),
            "{name} must not import command; the command module calls the renderer"
        );
    }
}

#[test]
fn commands_do_not_rebuild_the_ladder_or_model_inputs() {
    for (path, text) in sources("command") {
        assert!(
            !text.contains("TimeframeInput {"),
            "{} must use the pipeline input builder",
            path.display()
        );
        assert!(
            !text.contains("const LADDER"),
            "{} must use the data engine ladder",
            path.display()
        );
    }
}

#[test]
fn the_legacy_analytics_crate_is_gone() {
    let manifest_path = crates_root().join("../Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display()));
    assert!(!manifest.contains("quantrule-analytics"));
    assert!(!manifest.contains("analytics"));
}

/// Every workspace crate and the workspace crates it may depend on.
///
/// The order is the dependency order, so a crate may only name crates above
/// it. Adding an edge is an architecture decision; this table is its guard.
const ALLOWED_EDGES: [(&str, &[&str]); 5] = [
    ("quantrule-data", &[]),
    ("quantrule-model", &["quantrule-data"]),
    ("quantrule-backtest", &["quantrule-data", "quantrule-model"]),
    (
        "quantrule-app",
        &["quantrule-backtest", "quantrule-data", "quantrule-model"],
    ),
    (
        "quantrule-shell",
        &[
            "quantrule-app",
            "quantrule-backtest",
            "quantrule-data",
            "quantrule-model",
        ],
    ),
];

fn crates_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the shell crate lives under crates/")
        .to_path_buf()
}

/// Reads every workspace manifest and returns its workspace dependencies.
fn workspace_graph() -> BTreeMap<String, Vec<String>> {
    let crates = crates_root();
    ALLOWED_EDGES
        .iter()
        .map(|(name, _)| {
            let manifest = crates.join(name).join("Cargo.toml");
            let text = fs::read_to_string(&manifest)
                .unwrap_or_else(|error| panic!("read {}: {error}", manifest.display()));
            ((*name).to_owned(), parse_workspace_dependencies(&text))
        })
        .collect()
}

/// The `quantrule-*` keys of the manifest's `[dependencies]` table.
fn parse_workspace_dependencies(manifest: &str) -> Vec<String> {
    let mut in_dependencies = false;
    let mut dependencies = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            continue;
        }
        if !in_dependencies || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.starts_with("quantrule-") {
            dependencies.push(key.to_owned());
        }
    }
    dependencies
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
