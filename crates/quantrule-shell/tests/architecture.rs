//! Architecture guards for the shell crate.
//!
//! The shell parses, calls the pipeline, and renders. These tests protect the
//! one-way renderer edges, the single strategy walk, and the workspace crate
//! graph: the architecture documents are only truthful while a test reads the
//! manifests themselves.

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
    for file in ["lab_summary.rs", "render_state.rs"] {
        let text = source(file);
        assert!(
            !text.contains("use crate::command"),
            "{file} must not import command; the command module calls the renderer"
        );
    }
}

#[test]
fn the_shell_does_not_walk_the_strategy() {
    for (path, text) in sources("command") {
        assert!(
            !text.contains(".on_bar("),
            "{} must not call the strategy walk; the runner owns it",
            path.display()
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

/// Every workspace crate and the workspace crates it may depend on.
///
/// The order is the dependency order, so a crate may only name crates above
/// it. Adding an edge is an architecture decision; this table is its guard.
const ALLOWED_EDGES: [(&str, &[&str]); 5] = [
    ("quantrule-data", &[]),
    ("quantrule-model", &["quantrule-data"]),
    ("quantrule-backtest", &["quantrule-data", "quantrule-model"]),
    (
        "quantrule-analytics",
        &["quantrule-data", "quantrule-model"],
    ),
    (
        "quantrule-shell",
        &[
            "quantrule-analytics",
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
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

/// Checks a graph against [`ALLOWED_EDGES`], naming a forbidden pair.
fn check_graph(graph: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    for (name, allowed) in ALLOWED_EDGES {
        let Some(actual) = graph.get(name) else {
            return Err(format!("the graph omits the crate {name}"));
        };
        for dependency in actual {
            if !ALLOWED_EDGES
                .iter()
                .any(|(candidate, _)| *candidate == dependency.as_str())
            {
                return Err(format!(
                    "{name} depends on {dependency}, which is not a workspace crate"
                ));
            }
            if !allowed.contains(&dependency.as_str()) {
                return Err(format!("forbidden dependency: {name} -> {dependency}"));
            }
        }
        for dependency in allowed {
            if !actual.iter().any(|edge| edge.as_str() == *dependency) {
                return Err(format!(
                    "the graph omits the allowed dependency: {name} -> {dependency}"
                ));
            }
        }
    }
    for name in graph.keys() {
        if !ALLOWED_EDGES
            .iter()
            .any(|(candidate, _)| *candidate == name)
        {
            return Err(format!("the graph names an unknown crate {name}"));
        }
    }
    Ok(())
}

#[test]
fn the_workspace_crate_graph_is_one_way() {
    let graph = workspace_graph();
    if let Err(message) = check_graph(&graph) {
        panic!("the workspace crate graph violates the baseline: {message}");
    }
}

#[test]
fn a_synthetic_forbidden_edge_names_the_crate_pair() {
    let mut graph = workspace_graph();
    graph
        .get_mut("quantrule-model")
        .expect("the model crate")
        .push("quantrule-backtest".to_owned());
    let error = check_graph(&graph).expect_err("a model -> backtest edge is forbidden");
    assert!(
        error.contains("quantrule-model -> quantrule-backtest"),
        "the error must name the pair: {error}"
    );
}

#[test]
fn external_manifest_dependencies_are_not_workspace_edges() {
    let manifest = "[dependencies]\nserde = \"1\"\nquantrule-data = { path = \"../quantrule-data\" }\n\n[dev-dependencies]\nquantrule-model = { path = \"../quantrule-model\" }\n";
    assert_eq!(
        parse_workspace_dependencies(manifest),
        vec!["quantrule-data".to_owned()]
    );
}
