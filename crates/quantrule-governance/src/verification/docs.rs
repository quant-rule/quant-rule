//! Render the indicator and event verification matrices.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::build::{Entry, Manifest};
use super::evidence;

/// Write `indicator-verification.md`.
pub(super) fn write_indicators(
    root: &Path,
    manifest: &Manifest,
    indicators: &[Value],
    events: &[Value],
    claims: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let entries: BTreeMap<&str, &Entry> = manifest
        .indicators
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    let mut counts: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for event in events {
        let ident = event.get("indicator").and_then(Value::as_str).unwrap_or("");
        let event_type = event.get("event_type").and_then(Value::as_i64).unwrap_or(1);
        let slot = &mut counts.entry(ident).or_insert([0, 0, 0]);
        if let Some(index) = usize::try_from(event_type - 1)
            .ok()
            .filter(|index| *index < 3)
        {
            slot[index] += 1;
        }
    }
    let mut sorted: Vec<&Value> = indicators.iter().collect();
    sorted.sort_by_key(|indicator| indicator.get("code").and_then(Value::as_i64));
    let mut lines = vec![
        "# Indicator Verification Matrix".to_owned(),
        String::new(),
        "- Date: 2026-11-06 (UTC)".to_owned(),
        "- Status: Generated from `docs/registry/verification.json`".to_owned(),
        "- Claim type: evidence map; no edge claim".to_owned(),
        String::new(),
        "`✅` row-level or tool gate; `◐` family-level evidence; `—` not applicable.".to_owned(),
        String::new(),
        "| # | Indicator | Code | Area | L/D/S | Registry | Construct test | Predictive | F | C | D | G | E | A | N | P | M | Status |".to_owned(),
        "|---:|---|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|".to_owned(),
    ];
    for (number, indicator) in sorted.iter().enumerate() {
        let ident = indicator.get("id").and_then(Value::as_str).unwrap_or("");
        let Some(entry) = entries.get(ident) else {
            continue;
        };
        let claim = claims.get(ident);
        let [l, d, s] = counts.get(ident).copied().unwrap_or([0, 0, 0]);
        let formula_tests = &entry.evidence.get("formula").tests;
        let tests = if formula_tests.is_empty() {
            "—".to_owned()
        } else {
            formula_tests.join(", ").replace('|', "\\|")
        };
        let predictive = claim
            .and_then(|claim| claim.get("predictive_status"))
            .and_then(Value::as_str)
            .unwrap_or("—");
        lines.push(format!(
            "| {} | `{}` | {} | {} | {}/{}/{} | ✅ | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            number + 1,
            ident,
            indicator.get("code").and_then(Value::as_i64).unwrap_or(0),
            area_of(ident),
            l,
            d,
            s,
            tests,
            predictive,
            cell(entry, "formula"),
            cell(entry, "causality"),
            cell(entry, "determinism"),
            cell(entry, "golden"),
            cell(entry, "edge"),
            cell(entry, "availability"),
            cell(entry, "no_double_count"),
            cell(entry, "precedence"),
            cell(entry, "math"),
            status(entry),
        ));
    }
    lines.extend([
        String::new(),
        "## Area totals".to_owned(),
        String::new(),
        "| Area | Indicators | L | D | S | Total |".to_owned(),
        "|---|---:|---:|---:|---:|---:|".to_owned(),
    ]);
    for (area, ids) in evidence::AREAS {
        let mut l = 0;
        let mut d = 0;
        let mut s = 0;
        for ident in *ids {
            let values = counts.get(ident).copied().unwrap_or([0, 0, 0]);
            l += values[0];
            d += values[1];
            s += values[2];
        }
        lines.push(format!(
            "| {area} | {} | {l} | {d} | {s} | {} |",
            ids.len(),
            l + d + s
        ));
    }
    let total_l: usize = counts.values().map(|value| value[0]).sum();
    let total_d: usize = counts.values().map(|value| value[1]).sum();
    let total_s: usize = counts.values().map(|value| value[2]).sum();
    lines.push(format!(
        "| **Total** | **{}** | **{total_l}** | **{total_d}** | **{total_s}** | **{}** |",
        indicators.len(),
        events.len()
    ));
    write(
        &root.join("docs/model/2026-11-06/indicator-verification.md"),
        &lines,
    )
}

/// Write `event-verification.md`.
pub(super) fn write_events(
    root: &Path,
    manifest: &Manifest,
    events: &[Value],
    claims: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let entries: BTreeMap<&str, &Entry> = manifest
        .events
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    let mut sorted: Vec<&Value> = events.iter().collect();
    sorted.sort_by_key(|event| event.get("code").and_then(Value::as_i64));
    let mut lines = vec![
        "# Event Verification Matrix".to_owned(),
        String::new(),
        "- Date: 2026-11-06 (UTC)".to_owned(),
        "- Status: Generated from `docs/registry/verification.json`".to_owned(),
        "- Claim type: evidence map; no edge claim".to_owned(),
        String::new(),
        "`✅` row-level or tool gate; `◐` family-level evidence; `—` not applicable. `event_code` is the identity column.".to_owned(),
        String::new(),
        "| Event ID | Code | Type | Subtype | Sources | Role | Formula | Indicator | Construct test | Predictive | Emitter | Causality | Determinism | Availability | Dedupe | Precedence | Spec | Status |".to_owned(),
        "|---|---:|---:|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|".to_owned(),
    ];
    for event in sorted {
        let ident = event.get("id").and_then(Value::as_str).unwrap_or("");
        let Some(entry) = entries.get(ident) else {
            continue;
        };
        let claim = claims.get(ident);
        let formula_tests = &entry.evidence.get("formula").tests;
        let tests = if formula_tests.is_empty() {
            "—".to_owned()
        } else {
            formula_tests.join(", ").replace('|', "\\|")
        };
        let sources = event
            .get("sources")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(render_value).collect::<Vec<_>>().join(","))
            .unwrap_or_else(|| render_value(event.get("source").unwrap_or(&Value::Null)));
        let formula = event
            .get("formula")
            .and_then(Value::as_str)
            .unwrap_or("")
            .replace('|', "\\|");
        let predictive = claim
            .and_then(|claim| claim.get("predictive_status"))
            .and_then(Value::as_str)
            .unwrap_or("—");
        lines.push(format!(
            "| `{}` | {} | {} | {} | {} | {} | {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            ident,
            event.get("code").and_then(Value::as_i64).unwrap_or(0),
            event.get("event_type").and_then(Value::as_i64).unwrap_or(0),
            event.get("subtype").and_then(Value::as_i64).unwrap_or(0),
            sources,
            event.get("role").and_then(Value::as_str).unwrap_or(""),
            formula,
            event.get("indicator").and_then(Value::as_str).unwrap_or(""),
            tests,
            predictive,
            cell(entry, "emitter"),
            cell(entry, "causality"),
            cell(entry, "determinism"),
            cell(entry, "availability"),
            cell(entry, "no_double_count"),
            cell(entry, "precedence"),
            cell(entry, "spec"),
            status(entry),
        ));
    }
    write(
        &root.join("docs/model/2026-11-06/event-verification.md"),
        &lines,
    )
}

fn area_of(ident: &str) -> &'static str {
    for (area, ids) in evidence::AREAS {
        if ids.contains(&ident) {
            return area;
        }
    }
    "—"
}

fn cell(entry: &Entry, name: &str) -> &'static str {
    match entry.evidence.get(name).scope {
        "row" => "✅",
        "family" => "◐",
        "tool" => "✅",
        "n/a" => "—",
        _ => "☐",
    }
}

fn status(entry: &Entry) -> &'static str {
    let partial = entry
        .evidence
        .slots
        .iter()
        .filter(|(name, _)| !matches!(*name, "formula" | "math" | "emitter"))
        .any(|(name, _)| cell(entry, name) == "◐");
    if partial {
        "partial (family)"
    } else {
        "covered"
    }
}

fn render_value(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

fn write(path: &Path, lines: &[String]) -> Result<(), String> {
    fs::write(path, format!("{}\n", lines.join("\n")))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}
