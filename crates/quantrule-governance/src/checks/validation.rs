//! Check that every active indicator and event has a validation claim.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::checks::rust_tests;
use crate::report::Diagnostic;

const CLAIM_KEYS: [&str; 10] = [
    "id",
    "kind",
    "construct_tests",
    "construct_status",
    "primary_outcome",
    "horizon_bars",
    "target_atr",
    "stop_atr",
    "baseline",
    "predictive_status",
];
const REQUIRED: [&str; 6] = [
    "id",
    "kind",
    "construct_tests",
    "construct_status",
    "primary_outcome",
    "predictive_status",
];
const KINDS: [&str; 2] = ["indicator", "event"];
const STATUSES: [&str; 6] = [
    "pending",
    "pass",
    "no_edge",
    "defect",
    "inconclusive",
    "not_applicable",
];
const OUTCOMES: [&str; 8] = [
    "path_directional",
    "structure",
    "reversal",
    "volatility",
    "participation",
    "progression",
    "trend_strength",
    "none",
];

/// Check the validation manifest against the active registry.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let registry_path = root.join("docs/registry/indicators.json");
    let validation_path = root.join("docs/registry/validation.json");
    let Ok(registry) = load(&registry_path) else {
        return vec![Diagnostic::path(
            "validation",
            &registry_path,
            root,
            "cannot read the registry",
        )];
    };
    let Ok(validation) = load(&validation_path) else {
        return vec![Diagnostic::path(
            "validation",
            &validation_path,
            root,
            "cannot read the validation manifest",
        )];
    };
    let mut diagnostics = Vec::new();
    if validation.get("schema").and_then(Value::as_str) != Some("quantrule.validation/v1") {
        diagnostics.push(Diagnostic::path(
            "validation",
            &validation_path,
            root,
            "validation schema must be quantrule.validation/v1",
        ));
    }
    let Some(claims) = validation.get("claims").and_then(Value::as_array) else {
        diagnostics.push(Diagnostic::path(
            "validation",
            &validation_path,
            root,
            "validation.claims must be an array",
        ));
        return diagnostics;
    };
    let active_indicators = active_ids(&registry, "indicators");
    let active_events = active_ids(&registry, "events");
    let event_roles = event_roles(&registry);
    let functions = rust_tests::function_names(root);
    let mut seen = BTreeSet::new();
    let mut claimed_indicators = BTreeSet::new();
    let mut claimed_events = BTreeSet::new();
    for (index, claim) in claims.iter().enumerate() {
        let label = format!("claims[{index}]");
        let Some(claim) = claim.as_object() else {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label} must be an object"),
            ));
            continue;
        };
        let keys: BTreeSet<&str> = claim.keys().map(String::as_str).collect();
        let missing: Vec<_> = REQUIRED
            .iter()
            .filter(|key| !keys.contains(**key))
            .collect();
        let extra: Vec<_> = keys
            .iter()
            .filter(|key| !CLAIM_KEYS.contains(key))
            .collect();
        if !missing.is_empty() {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label} missing keys: {missing:?}"),
            ));
        }
        if !extra.is_empty() {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label} unknown keys: {extra:?}"),
            ));
        }
        let Some(ident) = claim
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        else {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label}.id must be a non-empty string"),
            ));
            continue;
        };
        if !seen.insert(ident.to_owned()) {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label}.id duplicate: {ident}"),
            ));
        }
        let Some(kind) = claim.get("kind").and_then(Value::as_str) else {
            continue;
        };
        if !KINDS.contains(&kind) {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label}.kind is invalid"),
            ));
            continue;
        }
        if kind == "indicator" {
            claimed_indicators.insert(ident.to_owned());
            if !active_indicators.contains(ident) {
                diagnostics.push(Diagnostic::path(
                    "validation",
                    &validation_path,
                    root,
                    format!("{label} names an unknown or inactive indicator: {ident}"),
                ));
            }
        } else {
            claimed_events.insert(ident.to_owned());
            if !active_events.contains(ident) {
                diagnostics.push(Diagnostic::path(
                    "validation",
                    &validation_path,
                    root,
                    format!("{label} names an unknown or inactive event: {ident}"),
                ));
            }
        }
        let tests = claim.get("construct_tests").and_then(Value::as_array);
        if tests.is_none_or(|tests| tests.is_empty()) {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label}.construct_tests must be a non-empty string array"),
            ));
        } else if let Some(tests) = tests {
            for test in tests.iter().filter_map(Value::as_str) {
                let name = test.rsplit("::").next().unwrap_or(test);
                if !functions.contains(name) {
                    diagnostics.push(Diagnostic::path(
                        "validation",
                        &validation_path,
                        root,
                        format!("{label}.construct_tests names a missing function: {test}"),
                    ));
                }
            }
        }
        if claim.get("construct_status").and_then(Value::as_str) != Some("covered") {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label}.construct_status must be covered"),
            ));
        }
        let outcome = claim
            .get("primary_outcome")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !OUTCOMES.contains(&outcome) {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label}.primary_outcome is invalid"),
            ));
        }
        let status = claim
            .get("predictive_status")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !STATUSES.contains(&status) {
            diagnostics.push(Diagnostic::path(
                "validation",
                &validation_path,
                root,
                format!("{label}.predictive_status is invalid"),
            ));
        }
        if kind == "event" {
            let role = event_roles.get(ident).map(String::as_str).unwrap_or("");
            if role == "signal" {
                if outcome == "none" {
                    diagnostics.push(Diagnostic::path(
                        "validation",
                        &validation_path,
                        root,
                        format!("{label} is a signal event with no primary outcome"),
                    ));
                }
                if status == "not_applicable" {
                    diagnostics.push(Diagnostic::path(
                        "validation",
                        &validation_path,
                        root,
                        format!("{label} is a signal event and cannot be not_applicable"),
                    ));
                }
                for field in ["horizon_bars", "target_atr", "stop_atr"] {
                    if !claim
                        .get(field)
                        .and_then(Value::as_f64)
                        .is_some_and(|value| value > 0.0)
                    {
                        diagnostics.push(Diagnostic::path(
                            "validation",
                            &validation_path,
                            root,
                            format!("{label}.{field} must be positive"),
                        ));
                    }
                }
                if !claim
                    .get("baseline")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
                {
                    diagnostics.push(Diagnostic::path(
                        "validation",
                        &validation_path,
                        root,
                        format!("{label}.baseline must be a non-empty string"),
                    ));
                }
            } else if role == "context" || role == "diagnostic" {
                if outcome != "none" {
                    diagnostics.push(Diagnostic::path(
                        "validation",
                        &validation_path,
                        root,
                        format!("{label} is a {role} event; primary_outcome must be none"),
                    ));
                }
                if status != "not_applicable" {
                    diagnostics.push(Diagnostic::path(
                        "validation",
                        &validation_path,
                        root,
                        format!("{label} is a {role} event; predictive_status must be \"not_applicable\""),
                    ));
                }
            }
        }
    }
    for ident in active_indicators.difference(&claimed_indicators) {
        diagnostics.push(Diagnostic::path(
            "validation",
            &validation_path,
            root,
            format!("active indicator has no validation claim: {ident}"),
        ));
    }
    for ident in active_events.difference(&claimed_events) {
        diagnostics.push(Diagnostic::path(
            "validation",
            &validation_path,
            root,
            format!("active event has no validation claim: {ident}"),
        ));
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

fn event_roles(registry: &Value) -> std::collections::BTreeMap<String, String> {
    registry
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("active"))
        .filter_map(|item| {
            Some((
                item.get("id")?.as_str()?.to_owned(),
                item.get("role")?.as_str()?.to_owned(),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrong_schema_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-validation-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/registry")).expect("tree");
        fs::write(
            root.join("docs/registry/indicators.json"),
            r#"{"indicators":[],"events":[]}"#,
        )
        .expect("fixture");
        fs::write(
            root.join("docs/registry/validation.json"),
            r#"{"schema":"wrong","claims":[]}"#,
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.iter().any(|d| d.message.contains("schema")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
