//! Check that every ported computation carries parity evidence.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::report::Diagnostic;

const REGISTRY: &str = "docs/port-parity/registry.json";
const SCHEMA: &str = "quantrule.port-parity/v1";
const FIXTURE_SCHEMA: &str = "quantrule.port-parity-fixture/v1";
const TOLERANCE: f64 = 1e-9;

/// Check the port-parity registry and every file it names.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let path = root.join(REGISTRY);
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{REGISTRY} is missing"),
        )];
    };
    let Ok(registry) = serde_json::from_str::<Value>(&text) else {
        return vec![Diagnostic::path(
            "port-parity",
            &path,
            root,
            "registry is not valid JSON",
        )];
    };
    let mut diagnostics = Vec::new();
    if registry.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("registry schema must be {SCHEMA}"),
        ));
    }
    let keys: BTreeSet<&str> = registry
        .as_object()
        .map(|object| object.keys().map(String::as_str).collect())
        .unwrap_or_default();
    if keys != BTreeSet::from(["schema", "ports"]) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            "registry must contain exactly schema and ports",
        ));
    }
    let ports = registry
        .get("ports")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !registry.get("ports").is_some_and(Value::is_array) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            "registry ports must be a list",
        ));
        return diagnostics;
    }
    let mut seen = BTreeSet::new();
    for entry in &ports {
        check_entry(root, &path, entry, &mut seen, &mut diagnostics);
    }
    diagnostics
}

fn check_entry(
    root: &Path,
    registry: &Path,
    entry: &Value,
    seen: &mut BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(entry) = entry.as_object() else {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            "registry entries must be objects",
        ));
        return;
    };
    let required = ["id", "source", "targets", "test", "fixture", "status"];
    let missing: Vec<_> = required
        .iter()
        .filter(|field| !entry.contains_key(**field))
        .collect();
    if !missing.is_empty() {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!(
                "registry entry is missing {}",
                missing
                    .iter()
                    .map(|field| **field)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
        return;
    }
    let unknown: Vec<_> = entry
        .keys()
        .filter(|key| !required.contains(&key.as_str()) && key.as_str() != "notes")
        .collect();
    if !unknown.is_empty() {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!(
                "registry entry has unknown fields: {}",
                unknown
                    .iter()
                    .map(|key| key.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
        return;
    }
    let Some(id) = entry.get("id").and_then(Value::as_str) else {
        return;
    };
    if !is_port_id(id) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("registry entry id must match PORT-NNN: {id:?}"),
        ));
        return;
    }
    if !seen.insert(id.to_owned()) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: duplicate registry entry"),
        ));
        return;
    }
    if !entry
        .get("source")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
    {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: source must be a non-empty string"),
        ));
    }
    if !entry
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|value| matches!(value, "active" | "retired"))
    {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: status must be active or retired"),
        ));
    }
    let Some(fixture) = entry
        .get("fixture")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: fixture must be a non-empty string"),
        ));
        return;
    };
    let expected_fixture = format!("docs/port-parity/{id}.golden.json");
    if fixture != expected_fixture {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: fixture must be {expected_fixture}, found {fixture}"),
        ));
    }
    let mut registered = BTreeSet::new();
    let targets = entry
        .get("targets")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if targets.is_empty() {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: targets must be a non-empty array"),
        ));
    }
    for target in targets.iter().filter_map(Value::as_str) {
        if !safe_relative(target) {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                registry,
                root,
                format!("{id}: target must stay inside the repository: {target:?}"),
            ));
            continue;
        }
        registered.insert(target.to_owned());
        if !target.ends_with(".rs") {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                registry,
                root,
                format!("{id}: target must be a Rust source file"),
            ));
        } else if !(root.join(target)).is_file() {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                registry,
                root,
                format!("{id}: target {target} is missing"),
            ));
        } else {
            let text = fs::read_to_string(root.join(target)).unwrap_or_default();
            if !text.contains(&format!("port-parity: {id}")) {
                diagnostics.push(Diagnostic::path(
                    "port-parity",
                    registry,
                    root,
                    format!("{id}: target {target} needs the marker port-parity: {id}"),
                ));
            }
        }
    }
    for path in rust_files(&root.join("crates")) {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if registered.contains(&relative) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        if text.contains(&format!("port-parity: {id}")) {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                &path,
                root,
                format!("{id}: unregistered target {relative} carries the marker"),
            ));
        }
    }
    let test = entry.get("test").and_then(Value::as_str).unwrap_or("");
    if !safe_relative(test) || !test.ends_with(".rs") {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: test must be a Rust source file"),
        ));
    } else if !(root.join(test)).is_file() {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: test {test} is missing"),
        ));
    } else {
        let text = fs::read_to_string(root.join(test)).unwrap_or_default();
        if !text.contains(&format!("port-parity test: {id}")) {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                registry,
                root,
                format!("{id}: test needs the marker port-parity test: {id}"),
            ));
        }
        if !text.contains(fixture) {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                registry,
                root,
                format!("{id}: test {test} must name {fixture}"),
            ));
        }
    }
    check_fixture(root, registry, entry, id, fixture, diagnostics);
}

fn check_fixture(
    root: &Path,
    registry: &Path,
    entry: &serde_json::Map<String, Value>,
    id: &str,
    fixture: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path = root.join(fixture);
    if !path.is_file() {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            registry,
            root,
            format!("{id}: fixture {fixture} is missing"),
        ));
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture is not valid JSON"),
        ));
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture must be a JSON object"),
        ));
        return;
    };
    if object.get("schema").and_then(Value::as_str) != Some(FIXTURE_SCHEMA) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture schema must be {FIXTURE_SCHEMA}"),
        ));
    }
    if object.get("id").and_then(Value::as_str) != Some(id) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture id must match the registry entry"),
        ));
    }
    if object.get("source").and_then(Value::as_str) != entry.get("source").and_then(Value::as_str) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture source must match the registry source"),
        ));
    }
    let captured = object.get("captured").and_then(Value::as_str).unwrap_or("");
    if !is_date(captured) {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture captured must be a UTC YYYY-MM-DD date"),
        ));
    }
    let tolerance = object
        .get("tolerance")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if !tolerance.is_finite() || tolerance <= 0.0 {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture tolerance must be a positive number"),
        ));
    } else if tolerance > TOLERANCE {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture tolerance {tolerance} is looser than {TOLERANCE}"),
        ));
    }
    let cases = object
        .get("cases")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if cases.is_empty() {
        diagnostics.push(Diagnostic::path(
            "port-parity",
            &path,
            root,
            format!("{id}: fixture cases must be a non-empty list"),
        ));
    }
    for (index, case) in cases.iter().enumerate() {
        let Some(case) = case.as_object() else {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                &path,
                root,
                format!("{id}: case {index} must be an object"),
            ));
            continue;
        };
        if !case
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| !name.is_empty())
        {
            diagnostics.push(Diagnostic::path(
                "port-parity",
                &path,
                root,
                format!("{id}: case {index} needs a non-empty name"),
            ));
        }
        for field in ["input", "expected"] {
            if !case.contains_key(field) {
                diagnostics.push(Diagnostic::path(
                    "port-parity",
                    &path,
                    root,
                    format!("{id}: case {index} needs {field}"),
                ));
            }
        }
    }
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(directory, &mut found);
    found
}

fn collect(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn is_port_id(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("PORT-") else {
        return false;
    };
    rest.len() == 3 && rest.bytes().all(|byte| byte.is_ascii_digit())
}

fn safe_relative(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('/') && !value.split('/').any(|part| part == "..")
}

fn is_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_registry_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-port-parity-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("tree");
        let diagnostics = check(&root);
        assert_eq!(diagnostics.len(), 1);
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
