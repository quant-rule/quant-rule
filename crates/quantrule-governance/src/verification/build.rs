//! Build the verification manifest and both evidence matrices.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::docs;
use super::evidence;
use super::json::Json;

/// Evidence classes in the manifest order.
pub(super) const CLASSES: [&str; 11] = [
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

static EMPTY_SLOT: Slot = Slot {
    scope: "n/a",
    tests: Vec::new(),
};

/// One evidence slot.
#[derive(Clone)]
pub(super) struct Slot {
    /// `row`, `family`, `tool`, or `n/a`.
    pub scope: &'static str,
    /// Test names that prove the class.
    pub tests: Vec<String>,
}

/// The evidence map of one registry row.
pub(super) struct Evidence {
    /// Slots in manifest order.
    pub slots: Vec<(&'static str, Slot)>,
}

impl Evidence {
    pub(super) fn new() -> Self {
        Self {
            slots: CLASSES
                .into_iter()
                .map(|name| (name, slot("n/a", Vec::new())))
                .collect(),
        }
    }

    pub(super) fn set(&mut self, name: &str, value: Slot) {
        if let Some((_, target)) = self.slots.iter_mut().find(|(key, _)| *key == name) {
            *target = value;
        }
    }

    pub(super) fn get(&self, name: &str) -> &Slot {
        self.slots
            .iter()
            .find(|(key, _)| *key == name)
            .map_or(&EMPTY_SLOT, |(_, value)| value)
    }
}

/// One indicator or event entry.
pub(super) struct Entry {
    /// Registry identifier.
    pub id: String,
    /// `indicator` or `event`.
    pub kind: &'static str,
    /// Evidence map.
    pub evidence: Evidence,
}

/// The complete manifest.
pub(super) struct Manifest {
    /// Manifest schema.
    pub schema: &'static str,
    /// Registry version.
    pub version: String,
    /// Indicator entries.
    pub indicators: Vec<Entry>,
    /// Event entries.
    pub events: Vec<Entry>,
}

/// Build the manifest and both matrices under `root`.
///
/// # Errors
///
/// Returns a read, parse, or write error.
pub(super) fn run(root: &Path) -> Result<(), String> {
    let registry = load(&root.join("docs/registry/indicators.json"))?;
    let validation = load(&root.join("docs/registry/validation.json"))?;
    let indicators = registry
        .get("indicators")
        .and_then(Value::as_array)
        .ok_or_else(|| "the registry has no indicators array".to_owned())?;
    let events = registry
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| "the registry has no events array".to_owned())?;
    let version = registry
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "the registry has no version".to_owned())?
        .to_owned();
    let claims = claims(&validation);
    let fixtures = fixture_event_tests(root, events)?;
    let manifest = Manifest {
        schema: "quantrule.verification/v1",
        version,
        indicators: indicators
            .iter()
            .map(|indicator| indicator_entry(indicator, &claims, events))
            .collect(),
        events: events
            .iter()
            .map(|event| event_entry(event, &claims, &fixtures))
            .collect(),
    };
    fs::write(
        root.join("docs/registry/verification.json"),
        manifest_json(&manifest).render(),
    )
    .map_err(|error| format!("cannot write verification.json: {error}"))?;
    docs::write_indicators(root, &manifest, indicators, events, &claims)?;
    docs::write_events(root, &manifest, events, &claims)?;
    println!(
        "wrote verification: {} indicators, {} events",
        manifest.indicators.len(),
        manifest.events.len()
    );
    Ok(())
}

fn manifest_json(manifest: &Manifest) -> Json {
    Json::object(vec![
        ("schema".to_owned(), Json::string(manifest.schema)),
        ("version".to_owned(), Json::string(&manifest.version)),
        (
            "indicators".to_owned(),
            Json::Array(manifest.indicators.iter().map(entry_json).collect()),
        ),
        (
            "events".to_owned(),
            Json::Array(manifest.events.iter().map(entry_json).collect()),
        ),
    ])
}

fn entry_json(entry: &Entry) -> Json {
    Json::object(vec![
        ("id".to_owned(), Json::string(&entry.id)),
        ("kind".to_owned(), Json::string(entry.kind)),
        (
            "evidence".to_owned(),
            Json::object(
                entry
                    .evidence
                    .slots
                    .iter()
                    .map(|(name, slot)| {
                        (
                            (*name).to_owned(),
                            Json::object(vec![
                                ("scope".to_owned(), Json::string(slot.scope)),
                                (
                                    "tests".to_owned(),
                                    Json::Array(slot.tests.iter().map(Json::string).collect()),
                                ),
                            ]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}

fn indicator_entry(indicator: &Value, claims: &BTreeMap<String, Value>, events: &[Value]) -> Entry {
    let ident = string(indicator, "id");
    let mut tests = claim_tests(claims, &ident);
    if let Some(extra) = evidence::indicator_formula_extra(&ident) {
        tests.extend(extra.iter().map(|test| (*test).to_owned()));
    }
    let formula_scope = tests
        .first()
        .map_or("family", |test| row_scope(test, &ident));
    let family = indicator_family(&ident);
    let family_tests = indicator_tests(family);
    let has_events = events
        .iter()
        .any(|event| event.get("indicator").and_then(Value::as_str) == Some(ident.as_str()));
    let mut map = Evidence::new();
    map.set(
        "formula",
        if tests.is_empty() {
            slot("n/a", Vec::new())
        } else {
            slot(formula_scope, tests.clone())
        },
    );
    map.set("causality", family_tests[0].clone());
    map.set("determinism", family_tests[1].clone());
    map.set("golden", family_tests[2].clone());
    map.set("edge", family_tests[3].clone());
    map.set(
        "math",
        slot(
            "tool",
            tests_of(&[
                "qr-governance check --checker registry",
                "qr-governance check --checker validation",
            ]),
        ),
    );
    map.set(
        "spec",
        if has_events {
            slot(
                "family",
                tests_of(&[
                    "report_spec_isolation::two_model_specs_with_disjoint_active_events_stay_disjoint",
                ]),
            )
        } else {
            slot("n/a", Vec::new())
        },
    );
    if let Some(rows) = evidence::row_indicator_tests(&ident) {
        for (name, tests) in rows {
            map.set(name, slot("row", tests_of(tests)));
        }
    }
    let row_test = row_test(&ident);
    if let Some(row_test) = &row_test {
        for name in ["formula", "causality", "determinism", "golden", "edge"] {
            map.set(name, slot("row", row_test.clone()));
        }
    }
    if let Some(extra) = evidence::indicator_formula_extra(&ident)
        && let Some(row_test) = &row_test
    {
        let mut tests = row_test.clone();
        tests.extend(extra.iter().map(|test| (*test).to_owned()));
        map.set("formula", slot("row", tests));
    }
    if evidence::ROW_INDICATOR_CONTRACTS.contains(&ident.as_str())
        && let Some(row_test) = &row_test
    {
        let mut tests = row_test.clone();
        if let Some(extra) = evidence::indicator_edge_extra(&ident) {
            tests.extend(extra.iter().map(|test| (*test).to_owned()));
        }
        tests.extend(
            evidence::EXTENDED_EDGE_TESTS
                .iter()
                .map(|test| (*test).to_owned()),
        );
        map.set("edge", slot("row", tests));
    }
    if let Some(tests) = evidence::indicator_availability(&ident) {
        map.set("availability", slot("row", tests_of(tests)));
    }
    Entry {
        id: ident,
        kind: "indicator",
        evidence: map,
    }
}

fn event_entry(
    event: &Value,
    claims: &BTreeMap<String, Value>,
    fixtures: &BTreeMap<String, Vec<String>>,
) -> Entry {
    let ident = string(event, "id");
    let tests = claim_tests(claims, &ident);
    let formula_scope = tests
        .first()
        .map_or("family", |test| row_scope(test, &ident));
    let mut map = Evidence::new();
    map.set(
        "formula",
        if tests.is_empty() {
            slot("n/a", Vec::new())
        } else {
            slot(formula_scope, tests)
        },
    );
    map.set(
        "causality",
        slot(
            "family",
            tests_of(&["event_validation::appending_bars_does_not_change_past_events"]),
        ),
    );
    map.set(
        "determinism",
        slot(
            "family",
            tests_of(&["event_validation::repeated_model_runs_publish_the_same_events"]),
        ),
    );
    map.set(
        "golden",
        slot(
            "family",
            evidence::event_golden_override(&ident)
                .map_or_else(|| tests_of(event_golden(event)), tests_of),
        ),
    );
    let mut edge = tests_of(evidence::EVENT_EDGE_TESTS);
    if let Some(extra) = evidence::event_edge_extra(&ident) {
        edge.extend(extra.iter().map(|test| (*test).to_owned()));
    }
    map.set("edge", slot("family", edge));
    map.set(
        "availability",
        slot(
            "family",
            tests_of(&["event_validation::every_batch_respects_availability_and_order"]),
        ),
    );
    map.set(
        "no_double_count",
        slot(
            "family",
            tests_of(&["event_validation::one_occurrence_is_recorded_once"]),
        ),
    );
    map.set(
        "precedence",
        evidence::row_event_precedence(&ident).map_or_else(
            || slot("n/a", Vec::new()),
            |test| slot("row", tests_of(&[test])),
        ),
    );
    map.set(
        "math",
        slot(
            "tool",
            tests_of(&["qr-governance check --checker registry"]),
        ),
    );
    map.set(
        "spec",
        slot(
            "family",
            tests_of(&[
                "report_spec_isolation::two_model_specs_with_disjoint_active_events_stay_disjoint",
            ]),
        ),
    );
    map.set(
        "emitter",
        slot(
            "tool",
            tests_of(&["qr-governance check --checker event-emitters"]),
        ),
    );
    if let Some(rows) = evidence::row_event_tests(&ident) {
        for name in [
            "formula",
            "causality",
            "determinism",
            "availability",
            "no_double_count",
        ] {
            map.set(name, slot("row", tests_of(rows)));
        }
        if evidence::row_event_golden(&ident) {
            map.set("golden", slot("row", tests_of(rows)));
        }
    }
    if let Some(tests) = fixtures.get(&ident) {
        for name in [
            "formula",
            "causality",
            "determinism",
            "golden",
            "availability",
            "no_double_count",
        ] {
            map.set(name, slot("row", tests.clone()));
        }
    }
    if let Some(extra) = evidence::event_formula_extra(&ident) {
        let base = map.get("formula").tests.clone();
        let mut tests = base;
        tests.extend(extra.iter().map(|test| (*test).to_owned()));
        map.set("formula", slot("row", tests));
    }
    if let Some(tests) = evidence::event_availability_override(&ident) {
        map.set("availability", slot("family", tests_of(tests)));
    }
    Entry {
        id: ident,
        kind: "event",
        evidence: map,
    }
}

fn indicator_family(ident: &str) -> &'static str {
    if evidence::ROW_PANEL_CONTRACTS.contains(&ident) {
        return "panel";
    }
    if evidence::ROW_SESSION_CONTRACTS.contains(&ident) {
        return "session";
    }
    if ident == "progression" {
        return "progression";
    }
    if ident == "context" {
        return "context";
    }
    if evidence::AREAS[0].1.contains(&ident) {
        return "base";
    }
    "extended"
}

fn indicator_tests(family: &str) -> [Slot; 4] {
    match family {
        "base" => [
            row("hardening::every_prefix_keeps_the_output_lengths_and_ranges"),
            row("hardening::the_model_run_is_deterministic_across_repeated_runs"),
            row("readings_golden::every_scalar_reading_matches_the_golden_values"),
            row("hardening::every_prefix_keeps_the_output_lengths_and_ranges"),
        ],
        "extended" => [
            row("extended_readings_golden::appending_bars_does_not_change_past_readings"),
            row("extended_readings_golden::repeated_computation_is_identical"),
            row("extended_readings_golden::the_extended_readings_match_the_golden_fixture"),
            row("edge_cases::an_empty_series_publishes_no_readings"),
        ],
        "panel" => [
            row("panel_readings_golden::appending_bars_does_not_change_past_panel_readings"),
            row("panel_readings_golden::repeated_panel_computation_is_identical"),
            row("panel_readings_golden::the_panel_readings_match_the_golden_fixture"),
            row("edge_cases::a_one_bar_panel_and_session_are_safe"),
        ],
        "session" => [
            row("session_readings_golden::appending_bars_does_not_change_past_session_readings"),
            row("session_readings_golden::repeated_session_computation_is_identical"),
            row("session_readings_golden::the_session_readings_match_the_golden_fixture"),
            row("edge_cases::a_one_bar_panel_and_session_are_safe"),
        ],
        "progression" => [
            row("progression_golden::every_prefix_matches_the_full_series"),
            row("progression_golden::the_per_bar_fixture_matches_the_computed_series"),
            row("progression_golden::the_per_bar_fixture_matches_the_computed_series"),
            row("edge_cases::an_empty_series_publishes_no_readings"),
        ],
        _ => [
            row("context_golden::later_candles_do_not_change_earlier_context"),
            row("context_golden::the_context_is_deterministic"),
            row("context_golden::the_context_matches_the_golden_fixture"),
            row("edge_cases::an_empty_series_publishes_no_readings"),
        ],
    }
}

fn row_test(ident: &str) -> Option<Vec<String>> {
    if evidence::ROW_INDICATOR_CONTRACTS.contains(&ident) {
        return Some(vec![format!(
            "per_row_extended_indicators::{ident}_row_contract"
        )]);
    }
    if evidence::ROW_PANEL_CONTRACTS.contains(&ident) {
        return Some(vec![format!(
            "per_row_panel_indicators::{ident}_row_contract"
        )]);
    }
    if evidence::ROW_SESSION_CONTRACTS.contains(&ident) {
        return Some(vec![format!(
            "per_row_session_indicators::{ident}_row_contract"
        )]);
    }
    evidence::row_base_contract(ident).map(|test| vec![test.to_owned()])
}

fn event_golden(event: &Value) -> &'static [&'static str] {
    let event_type = event.get("event_type").and_then(Value::as_i64).unwrap_or(0);
    let source = event.get("source").and_then(Value::as_i64).unwrap_or(0);
    if event_type == 1 {
        &["level_events_golden::the_level_events_match_the_golden_fixture"]
    } else if event_type == 2 {
        &["divergence_events_golden::the_divergence_events_match_the_golden_fixture_at_both_scales"]
    } else if source >= 19 {
        &["event_validation::extended_and_session_events_match_their_fixtures"]
    } else {
        &["signal_events_golden::the_signal_events_match_the_golden_fixture"]
    }
}

fn fixture_event_tests(
    root: &Path,
    events: &[Value],
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let by_code: BTreeMap<i64, String> = events
        .iter()
        .filter_map(|event| {
            Some((
                event.get("code").and_then(Value::as_i64)?,
                event.get("id").and_then(Value::as_str)?.to_owned(),
            ))
        })
        .collect();
    let mut tests = BTreeMap::new();
    for relative in evidence::EVENT_FIXTURES {
        let text = fs::read_to_string(root.join(relative))
            .map_err(|error| format!("cannot read {relative}: {error}"))?;
        for line in text.lines() {
            if line.is_empty() || line.starts_with('#') || line.starts_with("bar,") {
                continue;
            }
            let Some(code) = line.split(',').nth(1).and_then(|value| value.parse().ok()) else {
                continue;
            };
            let Some(ident) = by_code.get(&code) else {
                continue;
            };
            tests.insert(
                ident.clone(),
                vec![format!(
                    "per_row_extended_session_events::{}_row_contract",
                    ident.replace('.', "_")
                )],
            );
        }
    }
    Ok(tests)
}

fn claims(validation: &Value) -> BTreeMap<String, Value> {
    validation
        .get("claims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|claim| {
            Some((
                claim.get("id").and_then(Value::as_str)?.to_owned(),
                claim.clone(),
            ))
        })
        .collect()
}

fn claim_tests(claims: &BTreeMap<String, Value>, ident: &str) -> Vec<String> {
    claims
        .get(ident)
        .and_then(|claim| claim.get("construct_tests"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn row_scope(test: &str, ident: &str) -> &'static str {
    let suffix = ident
        .rsplit('.')
        .next()
        .unwrap_or(ident)
        .replace('_', "")
        .to_lowercase();
    if !suffix.is_empty() && test.replace('_', "").to_lowercase().contains(&suffix) {
        "row"
    } else {
        "family"
    }
}

fn slot(scope: &'static str, tests: Vec<String>) -> Slot {
    Slot { scope, tests }
}

fn row(test: &str) -> Slot {
    slot("row", vec![test.to_owned()])
}

fn tests_of(tests: &[&str]) -> Vec<String> {
    tests.iter().map(|test| (*test).to_owned()).collect()
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn load(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("cannot parse {}: {error}", path.display()))
}
