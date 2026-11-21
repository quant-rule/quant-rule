//! Validate the indicator and parameter registry and its generated table.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::report::Diagnostic;

mod defaults;
mod markdown;
mod model_codes;

const ROOT_KEYS: [&str; 6] = [
    "schema",
    "version",
    "indicators",
    "events",
    "parameters",
    "codes",
];
const INDICATOR_KEYS: [&str; 14] = [
    "id",
    "code",
    "name",
    "taxonomy",
    "role",
    "claim",
    "formula",
    "inputs",
    "parameters",
    "outputs",
    "availability",
    "events",
    "tests",
    "status",
];
const EVENT_KEYS: [&str; 14] = [
    "id",
    "code",
    "event_type",
    "subtype",
    "source",
    "indicator",
    "role",
    "claim",
    "formula",
    "parameters",
    "tests",
    "status",
    "sources",
    "precedence",
];
const PARAMETER_KEYS: [&str; 11] = [
    "id",
    "path",
    "type",
    "default",
    "min",
    "max",
    "enum",
    "unit",
    "configurable",
    "affects",
    "status",
];
const CODE_KEYS: [&str; 11] = [
    "event_types",
    "level_subtypes",
    "divergence_subtypes",
    "signal_subtypes",
    "sources",
    "roles",
    "scales",
    "target_bases",
    "lifecycles",
    "stages",
    "unavailable_reasons",
];
const ROLES: [&str; 3] = ["signal", "context", "diagnostic"];
const STATUSES: [&str; 4] = ["active", "planned", "retired", "experimental"];
const TAXONOMIES: [&str; 13] = [
    "trend",
    "momentum",
    "structure",
    "progression",
    "volatility",
    "volume",
    "participation",
    "flow",
    "price_action",
    "path",
    "cross_asset",
    "session",
    "calendar",
];
const AFFECTS: [&str; 5] = ["model", "policy", "execution", "portfolio", "benchmark"];
const PARAMETER_TYPES: [&str; 9] = [
    "integer",
    "number",
    "boolean",
    "string",
    "array<integer>",
    "array<number>",
    "array<string>",
    "array<array<integer>>",
    "object",
];
const UNITS: [&str; 9] = [
    "bars", "records", "fraction", "atr", "seconds", "bps", "currency", "period", "none",
];
const TEST_KINDS: [&str; 8] = [
    "golden",
    "causality",
    "determinism",
    "edge_cases",
    "property",
    "event",
    "boundary",
    "benchmark",
];

/// Validate the registry, its schema, the Rust codes, defaults, and table.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let path = root.join("docs/registry/indicators.json");
    let Ok(registry) = load(&path, root) else {
        return vec![Diagnostic::path(
            "registry",
            &path,
            root,
            "cannot read the registry",
        )];
    };
    let mut diagnostics = Vec::new();
    validate(root, &path, &registry, &mut diagnostics);
    diagnostics
}

/// Regenerate `docs/registry/indicators.md` from the registry.
///
/// # Errors
///
/// Returns the read or write error.
pub fn regenerate(root: &Path) -> Result<(), String> {
    let source = root.join("docs/registry/indicators.json");
    let registry = load(&source, root).map_err(|_| "cannot read the registry".to_owned())?;
    let markdown = markdown::render_markdown(&registry);
    let target = root.join("docs/registry/indicators.md");
    fs::write(&target, markdown).map_err(|error| format!("cannot write indicators.md: {error}"))
}

fn validate(root: &Path, path: &Path, registry: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let Some(object) = registry.as_object() else {
        diagnostics.push(finding(path, root, "registry root must be an object"));
        return;
    };
    required_and_unknown(
        path,
        root,
        "root",
        object,
        &ROOT_KEYS,
        &ROOT_KEYS,
        diagnostics,
    );
    if object.get("schema").and_then(Value::as_str) != Some("quantrule.registry/v1") {
        diagnostics.push(finding(path, root, "schema must be quantrule.registry/v1"));
    }
    if !object
        .get("version")
        .and_then(Value::as_str)
        .is_some_and(|version| !version.is_empty())
    {
        diagnostics.push(finding(path, root, "version must be a non-empty string"));
    }
    let (Some(indicators), Some(events), Some(parameters)) = (
        object.get("indicators").and_then(Value::as_array),
        object.get("events").and_then(Value::as_array),
        object.get("parameters").and_then(Value::as_array),
    ) else {
        diagnostics.push(finding(
            path,
            root,
            "indicators, events, and parameters must be arrays",
        ));
        return;
    };
    let Some(codes) = object.get("codes").and_then(Value::as_object) else {
        diagnostics.push(finding(path, root, "codes must be an object"));
        return;
    };
    check_codes(path, root, codes, diagnostics);
    check_schema_file(root, diagnostics);
    model_codes::check_model_codes(root, path, object, events, diagnostics);
    model_codes::check_source_codes(root, path, object, diagnostics);

    let mut indicator_ids = BTreeSet::new();
    let mut indicator_codes = BTreeSet::new();
    let mut indicator_events: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (index, indicator) in indicators.iter().enumerate() {
        let label = format!("indicators[{index}]");
        let Some(indicator) = indicator.as_object() else {
            diagnostics.push(finding(path, root, format!("{label} must be an object")));
            continue;
        };
        required_and_unknown(
            path,
            root,
            &label,
            indicator,
            &INDICATOR_KEYS,
            &INDICATOR_KEYS,
            diagnostics,
        );
        match indicator.get("id").and_then(Value::as_str) {
            Some(ident) if !ident.is_empty() => {
                if !indicator_ids.insert(ident.to_owned()) {
                    diagnostics.push(finding(
                        path,
                        root,
                        format!("{label}.id duplicate: {ident}"),
                    ));
                } else {
                    let listed = indicator
                        .get("events")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();
                    indicator_events.insert(ident.to_owned(), listed);
                }
            }
            _ => diagnostics.push(finding(
                path,
                root,
                format!("{label}.id must be a non-empty string"),
            )),
        }
        if !positive_integer(indicator.get("code")) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.code must be a positive integer"),
            ));
        } else if let Some(code) = indicator.get("code").and_then(Value::as_i64)
            && !indicator_codes.insert(code)
        {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.code duplicate: {code}"),
            ));
        }
        if !member(indicator.get("taxonomy"), &TAXONOMIES) {
            diagnostics.push(finding(path, root, format!("{label}.taxonomy is invalid")));
        }
        if !member(indicator.get("role"), &ROLES) {
            diagnostics.push(finding(path, root, format!("{label}.role is invalid")));
        }
        if !member(indicator.get("status"), &STATUSES) {
            diagnostics.push(finding(path, root, format!("{label}.status is invalid")));
        }
        for field in ["name", "claim", "formula", "availability"] {
            if !non_empty_string(indicator.get(field)) {
                diagnostics.push(finding(
                    path,
                    root,
                    format!("{label}.{field} must be a non-empty string"),
                ));
            }
        }
        for field in ["inputs", "parameters", "outputs", "events", "tests"] {
            let values = indicator.get(field).and_then(Value::as_array);
            if values.is_none_or(Vec::is_empty) && field != "events" {
                diagnostics.push(finding(
                    path,
                    root,
                    format!("{label}.{field} must be a non-empty array"),
                ));
            }
            if let Some(values) = values {
                for value in values {
                    if !non_empty_string(Some(value)) {
                        diagnostics.push(finding(
                            path,
                            root,
                            format!("{label}.{field} contains an invalid value"),
                        ));
                    }
                }
            }
        }
        check_tests(
            path,
            root,
            &format!("{label}.tests"),
            indicator.get("tests"),
            diagnostics,
        );
        if let Some(listed) = indicator.get("events").and_then(Value::as_array) {
            for event_id in listed.iter().filter_map(Value::as_str) {
                let known = events
                    .iter()
                    .any(|event| event.get("id").and_then(Value::as_str) == Some(event_id));
                if !known {
                    diagnostics.push(finding(
                        path,
                        root,
                        format!("{label}.events references unknown event {event_id}"),
                    ));
                }
            }
        }
    }

    let mut parameter_ids = BTreeSet::new();
    for (index, parameter) in parameters.iter().enumerate() {
        let label = format!("parameters[{index}]");
        let Some(parameter) = parameter.as_object() else {
            diagnostics.push(finding(path, root, format!("{label} must be an object")));
            continue;
        };
        required_and_unknown(
            path,
            root,
            &label,
            parameter,
            &required_keys(&PARAMETER_KEYS, &["min", "max", "enum"]),
            &PARAMETER_KEYS,
            diagnostics,
        );
        match parameter.get("id").and_then(Value::as_str) {
            Some(ident) if !ident.is_empty() => {
                if !parameter_ids.insert(ident.to_owned()) {
                    diagnostics.push(finding(
                        path,
                        root,
                        format!("{label}.id duplicate: {ident}"),
                    ));
                }
            }
            _ => diagnostics.push(finding(
                path,
                root,
                format!("{label}.id must be a non-empty string"),
            )),
        }
        if !non_empty_string(parameter.get("path")) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.path must be a non-empty string"),
            ));
        }
        if !member(parameter.get("type"), &PARAMETER_TYPES) {
            diagnostics.push(finding(path, root, format!("{label}.type is invalid")));
        }
        if !parameter.contains_key("default") {
            diagnostics.push(finding(path, root, format!("{label}.default is required")));
        }
        if !parameter.get("configurable").is_some_and(Value::is_boolean) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.configurable must be a boolean"),
            ));
        }
        if !member(parameter.get("unit"), &UNITS) {
            diagnostics.push(finding(path, root, format!("{label}.unit is invalid")));
        }
        if !member(parameter.get("status"), &STATUSES) {
            diagnostics.push(finding(path, root, format!("{label}.status is invalid")));
        }
        let affects_valid = parameter
            .get("affects")
            .and_then(Value::as_array)
            .is_some_and(|items| {
                !items.is_empty()
                    && items
                        .iter()
                        .all(|value| value.as_str().is_some_and(|item| AFFECTS.contains(&item)))
            });
        if !affects_valid {
            diagnostics.push(finding(path, root, format!("{label}.affects is invalid")));
        }
    }

    let mut event_codes = BTreeSet::new();
    for (index, event) in events.iter().enumerate() {
        let label = format!("events[{index}]");
        let Some(event) = event.as_object() else {
            diagnostics.push(finding(path, root, format!("{label} must be an object")));
            continue;
        };
        required_and_unknown(
            path,
            root,
            &label,
            event,
            &required_keys(&EVENT_KEYS, &["sources", "precedence"]),
            &EVENT_KEYS,
            diagnostics,
        );
        let ident = event.get("id").and_then(Value::as_str);
        if !non_empty_string(event.get("id")) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.id must be a non-empty string"),
            ));
        }
        if !positive_integer(event.get("code")) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.code must be a positive integer"),
            ));
        } else if let Some(code) = event.get("code").and_then(Value::as_i64)
            && !event_codes.insert(code)
        {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.code duplicate: {code}"),
            ));
        }
        let event_type = event.get("event_type").and_then(Value::as_i64);
        let subtype = event.get("subtype").and_then(Value::as_i64);
        match event_type {
            Some(1) if matches!(subtype, Some(1..=5)) => {}
            Some(2) if matches!(subtype, Some(1..=4)) => {}
            Some(3) if matches!(subtype, Some(1..=24)) => {}
            Some(1..=3) => diagnostics.push(finding(
                path,
                root,
                format!("{label}.subtype is outside the type's range"),
            )),
            _ => diagnostics.push(finding(
                path,
                root,
                format!("{label}.event_type is invalid"),
            )),
        }
        let owner = event.get("indicator").and_then(Value::as_str);
        match owner {
            Some(owner) if indicator_ids.contains(owner) => {
                let listed = ident.is_some_and(|ident| {
                    indicator_events
                        .get(owner)
                        .is_some_and(|events| events.contains(ident))
                });
                if !listed {
                    diagnostics.push(finding(
                        path,
                        root,
                        format!(
                            "{label}.indicator {owner} does not list {} in events",
                            ident.unwrap_or("")
                        ),
                    ));
                }
            }
            _ => diagnostics.push(finding(path, root, format!("{label}.indicator is unknown"))),
        }
        if !member(event.get("role"), &ROLES) {
            diagnostics.push(finding(path, root, format!("{label}.role is invalid")));
        }
        if !member(event.get("status"), &STATUSES) {
            diagnostics.push(finding(path, root, format!("{label}.status is invalid")));
        }
        if !non_empty_string(event.get("claim")) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.claim must be a non-empty string"),
            ));
        }
        let role = event.get("role").and_then(Value::as_str);
        let claim = event.get("claim").and_then(Value::as_str);
        if matches!(role, Some("signal" | "context")) && claim == Some("none") {
            diagnostics.push(finding(
                path,
                root,
                format!(
                    "{label}.claim must describe a {role} event; \"none\" is only for diagnostics",
                    role = role.unwrap_or("")
                ),
            ));
        }
        if role == Some("diagnostic") && claim != Some("none") {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.claim must be \"none\" for a diagnostic event"),
            ));
        }
        if !non_empty_string(event.get("formula")) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.formula must be a non-empty string"),
            ));
        }
        let sources = event
            .get("sources")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_else(|| vec![event.get("source").cloned().unwrap_or(Value::Null)]);
        if sources.is_empty() {
            diagnostics.push(finding(
                path,
                root,
                format!("{label}.sources must be a non-empty array"),
            ));
        } else {
            let declared = codes.get("sources").and_then(Value::as_object);
            for source in &sources {
                let numeric = source.as_i64().is_some_and(|code| (1..=57).contains(&code));
                let registered = source
                    .as_i64()
                    .map(|code| code.to_string())
                    .is_some_and(|code| {
                        declared.is_some_and(|declared| declared.contains_key(&code))
                    });
                if !numeric || !registered {
                    diagnostics.push(finding(
                        path,
                        root,
                        format!("{label} references unknown source {source}"),
                    ));
                }
            }
        }
        if let Some(items) = event.get("parameters").and_then(Value::as_array) {
            for parameter_id in items.iter().filter_map(Value::as_str) {
                if !parameter_ids.contains(parameter_id) {
                    diagnostics.push(finding(
                        path,
                        root,
                        format!("{label}.parameters references unknown parameter {parameter_id}"),
                    ));
                }
            }
        }
        check_tests(
            path,
            root,
            &format!("{label}.tests"),
            event.get("tests"),
            diagnostics,
        );
        if matches!(role, Some("signal" | "context")) {
            let has_kind = event
                .get("tests")
                .and_then(Value::as_array)
                .is_some_and(|tests| {
                    tests
                        .iter()
                        .any(|test| test.as_str().is_some_and(|test| TEST_KINDS.contains(&test)))
                });
            if !has_kind {
                diagnostics.push(finding(
                    path,
                    root,
                    format!("{label} needs at least one test kind"),
                ));
            }
        }
    }

    defaults::check_defaults(root, path, parameters, diagnostics);
    check_markdown(root, registry, diagnostics);
}

fn check_codes(
    path: &Path,
    root: &Path,
    codes: &Map<String, Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let declared: BTreeSet<&str> = codes.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = CODE_KEYS.into_iter().collect();
    report_difference(path, root, "codes", &declared, &expected, diagnostics);
    for key in CODE_KEYS {
        let Some(value) = codes.get(key).and_then(Value::as_object) else {
            diagnostics.push(finding(
                path,
                root,
                format!("codes.{key} must be a non-empty object"),
            ));
            continue;
        };
        if value.is_empty() {
            diagnostics.push(finding(
                path,
                root,
                format!("codes.{key} must be a non-empty object"),
            ));
            continue;
        }
        for (code, label) in value {
            if !code.chars().all(|ch| ch.is_ascii_digit()) {
                diagnostics.push(finding(
                    path,
                    root,
                    format!("codes.{key} has a non-numeric key {code}"),
                ));
            }
            if !non_empty_string(Some(label)) {
                diagnostics.push(finding(
                    path,
                    root,
                    format!("codes.{key}.{code} must be a non-empty string"),
                ));
            }
        }
    }
}

fn check_schema_file(root: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let path = root.join("docs/registry/schema.json");
    let Ok(schema) = load(&path, root) else {
        diagnostics.push(finding(&path, root, "cannot read schema.json"));
        return;
    };
    let Some(schema) = schema.as_object() else {
        diagnostics.push(finding(&path, root, "schema.json must be an object"));
        return;
    };
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|values| string_set(values))
        .unwrap_or_default();
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|object| object.keys().map(String::as_str).collect())
        .unwrap_or_default();
    let expected: BTreeSet<&str> = ROOT_KEYS.into_iter().collect();
    if required != expected {
        diagnostics.push(finding(
            &path,
            root,
            format!(
                "schema.json required keys diverge: {}",
                symmetric(&required, &expected)
            ),
        ));
    }
    if properties != expected {
        diagnostics.push(finding(
            &path,
            root,
            format!(
                "schema.json properties diverge: {}",
                symmetric(&properties, &expected)
            ),
        ));
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        diagnostics.push(finding(
            &path,
            root,
            "schema.json must set additionalProperties false",
        ));
    }
    let Some(defs) = schema.get("$defs").and_then(Value::as_object) else {
        diagnostics.push(finding(&path, root, "schema.json $defs must be an object"));
        return;
    };
    for (name, expected) in [
        ("indicator", required_keys(&INDICATOR_KEYS, &[])),
        (
            "event",
            required_keys(&EVENT_KEYS, &["sources", "precedence"]),
        ),
    ] {
        let actual = defs
            .get(name)
            .and_then(|definition| definition.get("required"))
            .and_then(Value::as_array)
            .map(|values| string_set(values))
            .unwrap_or_default();
        let expected: BTreeSet<&str> = expected.iter().copied().collect();
        if actual != expected {
            diagnostics.push(finding(
                &path,
                root,
                format!(
                    "schema.json {name} required keys diverge: {}",
                    symmetric(&actual, &expected)
                ),
            ));
        }
    }
    let actual = defs
        .get("parameter")
        .and_then(|definition| definition.get("required"))
        .and_then(Value::as_array)
        .map(|values| string_set(values))
        .unwrap_or_default();
    let expected = required_keys(&PARAMETER_KEYS, &["min", "max", "enum"]);
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected {
        diagnostics.push(finding(
            &path,
            root,
            format!(
                "schema.json parameter required keys diverge: {}",
                symmetric(&actual, &expected)
            ),
        ));
    }
}

fn check_markdown(root: &Path, registry: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let path = root.join("docs/registry/indicators.md");
    let current = fs::read_to_string(&path).unwrap_or_default();
    let expected = markdown::render_markdown(registry);
    if current != expected {
        diagnostics.push(finding(
            &path,
            root,
            "is stale; run `qr-governance registry --write`",
        ));
    }
}

fn check_tests(
    path: &Path,
    root: &Path,
    label: &str,
    tests: Option<&Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(tests) = tests.and_then(Value::as_array) else {
        diagnostics.push(finding(
            path,
            root,
            format!("{label} must be a non-empty array"),
        ));
        return;
    };
    if tests.is_empty() {
        diagnostics.push(finding(
            path,
            root,
            format!("{label} must be a non-empty array"),
        ));
        return;
    }
    for test in tests {
        if !test.as_str().is_some_and(|test| TEST_KINDS.contains(&test)) {
            diagnostics.push(finding(
                path,
                root,
                format!("{label} has unknown test kind {test}"),
            ));
        }
    }
}

fn required_and_unknown(
    path: &Path,
    root: &Path,
    label: &str,
    value: &Map<String, Value>,
    required: &[&str],
    allowed: &[&str],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let actual: BTreeSet<&str> = value.keys().map(String::as_str).collect();
    let required: BTreeSet<&str> = required.iter().copied().collect();
    let allowed: BTreeSet<&str> = allowed.iter().copied().collect();
    let missing: Vec<&str> = required.difference(&actual).copied().collect();
    let extra: Vec<&str> = actual.difference(&allowed).copied().collect();
    if !missing.is_empty() {
        diagnostics.push(finding(
            path,
            root,
            format!("{label} missing keys: [{}]", missing.join(", ")),
        ));
    }
    if !extra.is_empty() {
        diagnostics.push(finding(
            path,
            root,
            format!("{label} unknown keys: [{}]", extra.join(", ")),
        ));
    }
}

fn report_difference(
    path: &Path,
    root: &Path,
    label: &str,
    actual: &BTreeSet<&str>,
    expected: &BTreeSet<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let missing: Vec<&str> = expected.difference(actual).copied().collect();
    let extra: Vec<&str> = actual.difference(expected).copied().collect();
    if !missing.is_empty() {
        diagnostics.push(finding(
            path,
            root,
            format!("{label} missing keys: [{}]", missing.join(", ")),
        ));
    }
    if !extra.is_empty() {
        diagnostics.push(finding(
            path,
            root,
            format!("{label} unknown keys: [{}]", extra.join(", ")),
        ));
    }
}

fn required_keys<'a>(allowed: &[&'a str], excluded: &[&str]) -> Vec<&'a str> {
    allowed
        .iter()
        .copied()
        .filter(|key| !excluded.contains(key))
        .collect()
}

fn string_set(values: &[Value]) -> BTreeSet<&str> {
    values.iter().filter_map(Value::as_str).collect()
}

fn symmetric<'a>(left: &BTreeSet<&'a str>, right: &BTreeSet<&'a str>) -> String {
    let mut values: Vec<&str> = left.symmetric_difference(right).copied().collect();
    values.sort_unstable();
    format!("[{}]", values.join(", "))
}

fn positive_integer(value: Option<&Value>) -> bool {
    value.and_then(Value::as_i64).is_some_and(|code| code > 0)
}

fn non_empty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.is_empty())
}

fn member(value: Option<&Value>, allowed: &[&str]) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| allowed.contains(&text))
}

fn load(path: &Path, root: &Path) -> Result<Value, ()> {
    let text = fs::read_to_string(path).map_err(|_| ())?;
    let _ = root;
    serde_json::from_str(&text).map_err(|_| ())
}

fn finding(path: &Path, root: &Path, message: impl Into<String>) -> Diagnostic {
    Diagnostic::path("registry", path, root, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_registry_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-registry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("tree");
        let diagnostics = check(&root);
        assert_eq!(diagnostics.len(), 1);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_stale_table_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-registry-stale-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/registry")).expect("tree");
        fs::create_dir_all(root.join("crates/quantrule-model/src/events")).expect("tree");
        fs::write(
            root.join("docs/registry/indicators.json"),
            r#"{"schema":"quantrule.registry/v1","version":"1","indicators":[],"events":[],"parameters":[],"codes":{"event_types":{},"level_subtypes":{},"divergence_subtypes":{},"signal_subtypes":{},"sources":{},"roles":{},"scales":{},"target_bases":{},"lifecycles":{},"stages":{},"unavailable_reasons":{}}}"#,
        )
        .expect("fixture");
        fs::write(
            root.join("docs/registry/schema.json"),
            r#"{"required":["schema","version","indicators","events","parameters","codes"],"properties":{"schema":{},"version":{},"indicators":{},"events":{},"parameters":{},"codes":{}},"additionalProperties":false,"$defs":{"indicator":{"required":["id","code","name","taxonomy","role","claim","formula","inputs","parameters","outputs","availability","events","tests","status"]},"event":{"required":["id","code","event_type","subtype","source","indicator","role","claim","formula","parameters","tests","status"]},"parameter":{"required":["id","path","type","default","unit","configurable","affects","status"]}}}"#,
        )
        .expect("fixture");
        fs::write(
            root.join("crates/quantrule-model/src/events/codes.rs"),
            "pub const COUNT: usize = 0;\n",
        )
        .expect("fixture");
        fs::write(
            root.join("crates/quantrule-model/src/events/source_codes.rs"),
            "pub const EXAMPLE_SOURCE_CODE: u16 = 1;\n",
        )
        .expect("fixture");
        fs::write(
            root.join("crates/quantrule-model/src/lib.rs"),
            "pub const REGISTRY_VERSION: &str = \"1\";\n",
        )
        .expect("fixture");
        fs::write(root.join("docs/registry/indicators.md"), "stale\n").expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("stale"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
