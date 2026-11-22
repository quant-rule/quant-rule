//! Check the per-row verification manifest.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::checks::rust_tests;
use crate::report::Diagnostic;

const TOOLS: [&str; 3] = [
    "qr-governance check --checker registry",
    "qr-governance check --checker validation",
    "qr-governance check --checker event-emitters",
];
const CLASSES: [&str; 11] = [
    "formula",
    "causality",
    "determinism",
    "golden",
    "edge",
    "availability",
    "no_double_count",
    "precedence",
    "math",
    "spec",
    "emitter",
];
const SCOPES: [&str; 4] = ["row", "family", "tool", "n/a"];
const REQUIRED: [&str; 4] = ["causality", "determinism", "golden", "edge"];

/// Check the verification manifest against the active registry and Rust tests.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let registry_path = root.join("docs/registry/indicators.json");
    let manifest_path = root.join("docs/registry/verification.json");
    let Ok(registry) = load(&registry_path) else {
        return vec![Diagnostic::path(
            "verification",
            &registry_path,
            root,
            "cannot read the registry",
        )];
    };
    let Ok(manifest) = load(&manifest_path) else {
        return vec![Diagnostic::path(
            "verification",
            &manifest_path,
            root,
            "cannot read the verification manifest",
        )];
    };
    let mut diagnostics = Vec::new();
    if manifest.get("schema").and_then(Value::as_str) != Some("quantrule.verification/v1") {
        diagnostics.push(Diagnostic::path(
            "verification",
            &manifest_path,
            root,
            "manifest schema must be quantrule.verification/v1",
        ));
    }
    let functions = rust_tests::function_names(root);
    let event_functions = rust_tests::event_test_names(root);
    for (kind, active) in [
        ("indicators", active_ids(&registry, "indicators")),
        ("events", active_ids(&registry, "events")),
    ] {
        let Some(entries) = manifest.get(kind).and_then(Value::as_array) else {
            diagnostics.push(Diagnostic::path(
                "verification",
                &manifest_path,
                root,
                format!("{kind} must be an array"),
            ));
            continue;
        };
        let mut seen = BTreeSet::new();
        for entry in entries {
            let Some(ident) = entry.get("id").and_then(Value::as_str) else {
                continue;
            };
            if !seen.insert(ident.to_owned()) {
                diagnostics.push(Diagnostic::path(
                    "verification",
                    &manifest_path,
                    root,
                    format!("{kind} duplicate entry: {ident}"),
                ));
            }
            let expected_kind = if kind == "indicators" {
                "indicator"
            } else {
                "event"
            };
            if entry.get("kind").and_then(Value::as_str) != Some(expected_kind) {
                diagnostics.push(Diagnostic::path(
                    "verification",
                    &manifest_path,
                    root,
                    format!("{ident}: kind must be {expected_kind}"),
                ));
            }
            let Some(evidence) = entry.get("evidence").and_then(Value::as_object) else {
                diagnostics.push(Diagnostic::path(
                    "verification",
                    &manifest_path,
                    root,
                    format!("{ident}: evidence must be an object"),
                ));
                continue;
            };
            for class in CLASSES {
                let Some(item) = evidence.get(class).and_then(Value::as_object) else {
                    diagnostics.push(Diagnostic::path(
                        "verification",
                        &manifest_path,
                        root,
                        format!("{ident}: missing evidence class {class}"),
                    ));
                    continue;
                };
                let scope = item.get("scope").and_then(Value::as_str).unwrap_or("");
                if !SCOPES.contains(&scope) {
                    diagnostics.push(Diagnostic::path(
                        "verification",
                        &manifest_path,
                        root,
                        format!("{ident}.{class}: invalid scope {scope:?}"),
                    ));
                    continue;
                }
                let Some(tests) = item.get("tests").and_then(Value::as_array) else {
                    diagnostics.push(Diagnostic::path(
                        "verification",
                        &manifest_path,
                        root,
                        format!("{ident}.{class}: tests must be an array"),
                    ));
                    continue;
                };
                let tests: Vec<&str> = tests.iter().filter_map(Value::as_str).collect();
                if scope == "n/a" {
                    if REQUIRED.contains(&class) {
                        diagnostics.push(Diagnostic::path(
                            "verification",
                            &manifest_path,
                            root,
                            format!("{ident}.{class}: {class} evidence cannot be n/a"),
                        ));
                    }
                    if !tests.is_empty() {
                        diagnostics.push(Diagnostic::path(
                            "verification",
                            &manifest_path,
                            root,
                            format!("{ident}.{class}: n/a must not carry tests"),
                        ));
                    }
                    continue;
                }
                if tests.is_empty() {
                    diagnostics.push(Diagnostic::path(
                        "verification",
                        &manifest_path,
                        root,
                        format!("{ident}.{class}: tests must be a non-empty string array"),
                    ));
                    continue;
                }
                if scope == "tool" {
                    for test in &tests {
                        if !TOOLS.contains(test) {
                            diagnostics.push(Diagnostic::path(
                                "verification",
                                &manifest_path,
                                root,
                                format!("{ident}.{class}: unknown tool gate {test:?}"),
                            ));
                        }
                    }
                    continue;
                }
                for test in &tests {
                    let name = test.rsplit("::").next().unwrap_or(test);
                    if !functions.contains(name) {
                        diagnostics.push(Diagnostic::path(
                            "verification",
                            &manifest_path,
                            root,
                            format!("{ident}.{class}: missing Rust test {test:?}"),
                        ));
                    }
                }
                if scope == "row" {
                    let suffix = ident
                        .rsplit('.')
                        .next()
                        .unwrap_or(ident)
                        .replace('_', "")
                        .to_lowercase();
                    if !tests
                        .iter()
                        .any(|test| test.replace('_', "").to_lowercase().contains(&suffix))
                    {
                        diagnostics.push(Diagnostic::path(
                            "verification",
                            &manifest_path,
                            root,
                            format!("{ident}.{class}: row scope does not name the row"),
                        ));
                    }
                }
                if kind == "events" && class == "edge" && scope != "tool" {
                    for test in &tests {
                        let name = test.rsplit("::").next().unwrap_or(test);
                        if !event_functions.contains(name) {
                            diagnostics.push(Diagnostic::path(
                                "verification",
                                &manifest_path,
                                root,
                                format!(
                                    "{ident}.{class}: edge evidence does not construct events {test:?}"
                                ),
                            ));
                        }
                    }
                }
            }
        }
        let missing: Vec<_> = active.difference(&seen).cloned().collect();
        let extra: Vec<_> = seen.difference(&active).cloned().collect();
        if !missing.is_empty() {
            diagnostics.push(Diagnostic::path(
                "verification",
                &manifest_path,
                root,
                format!(
                    "{kind} missing entries: {}",
                    missing
                        .iter()
                        .take(10)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        if !extra.is_empty() {
            diagnostics.push(Diagnostic::path(
                "verification",
                &manifest_path,
                root,
                format!(
                    "{kind} unknown entries: {}",
                    extra
                        .iter()
                        .take(10)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    diagnostics
}

fn load(path: &Path) -> Result<Value, ()> {
    let text = fs::read_to_string(path).map_err(|_| ())?;
    serde_json::from_str(&text).map_err(|_| ())
}

fn active_ids(registry: &Value, key: &str) -> BTreeSet<String> {
    registry
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("active"))
        .filter_map(|item| item.get("id")?.as_str().map(str::to_owned))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrong_schema_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-verification-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/registry")).expect("tree");
        fs::write(
            root.join("docs/registry/indicators.json"),
            r#"{"indicators":[],"events":[]}"#,
        )
        .expect("fixture");
        fs::write(
            root.join("docs/registry/verification.json"),
            r#"{"schema":"wrong","indicators":[],"events":[]}"#,
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.iter().any(|d| d.message.contains("schema")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
