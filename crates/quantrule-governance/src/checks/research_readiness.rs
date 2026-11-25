//! Check the HA-015 research readiness matrix.
//!
//! The matrix is the one record of the eight research-readiness rows. It must
//! state the research-only boundary, give every row an explicit status, and
//! carry the human-accepted risk for the simulated-lab boundary. The checker
//! does not require an evidence file to exist: an evidence path is a location,
//! and a missing run keeps the row `Unverified`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::report::Diagnostic;

const MATRIX: &str = "docs/config/2026-11-25/research-readiness.json";
const SCHEMA: &str = "quantrule.research-readiness/v1";
const SIMULATED_RISK: &str = "AR-RESEARCH-SIMULATED-LAB";
const ROW_STATUSES: [&str; 6] = [
    "Current",
    "Measured",
    "Simulated HA",
    "Accepted risk",
    "Unverified",
    "Retired",
];
const TIERS: [&str; 6] = [
    "Fast",
    "Composed",
    "Multi-host",
    "Simulated HA",
    "Real HA",
    "Static",
];
const REQUIRED_ROW_FIELDS: [&str; 11] = [
    "id",
    "claim",
    "owner",
    "issue",
    "test",
    "evidence",
    "status",
    "tier",
    "boundary",
    "recorded_on",
    "notes",
];
const OPTIONAL_ROW_FIELDS: [&str; 5] = [
    "accepted_by",
    "accepted_on",
    "residual_risk",
    "next_action",
    "superseded_by",
];
const PROVEN_STATUSES: [&str; 3] = ["Current", "Measured", "Simulated HA"];
const RISK_FIELDS: [&str; 8] = [
    "id",
    "risk",
    "impact",
    "accepted_by",
    "accepted_on",
    "residual_risk",
    "next_action",
    "links",
];

/// Check the HA-015 research readiness matrix.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let path = root.join(MATRIX);
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![Diagnostic::path(
            "research-readiness",
            &path,
            root,
            format!("{MATRIX} is missing"),
        )];
    };
    let Ok(matrix) = serde_json::from_str::<Value>(&text) else {
        return vec![Diagnostic::path(
            "research-readiness",
            &path,
            root,
            "the research readiness matrix is not valid JSON",
        )];
    };
    let mut diagnostics = Vec::new();
    check_top_level(root, &path, &matrix, &mut diagnostics);
    diagnostics
}

fn check_top_level(root: &Path, path: &Path, matrix: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let Some(object) = matrix.as_object() else {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "the matrix must be a JSON object",
        ));
        return;
    };
    if object.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("schema must be {SCHEMA}"),
        ));
    }
    let expected_keys =
        BTreeSet::from(["accepted_risks", "recorded_on", "rows", "schema", "system"]);
    let observed: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    if observed != expected_keys {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "the matrix must contain exactly schema, recorded_on, system, rows, and accepted_risks",
        ));
    }
    if !text(object, "recorded_on") {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "recorded_on must be a non-empty string",
        ));
    }
    check_system(root, path, object.get("system"), diagnostics);
    let Some(rows) = object.get("rows").and_then(Value::as_array) else {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "rows must be a list",
        ));
        return;
    };
    let expected_ids: Vec<String> = (1..=8).map(|n| format!("RR-{n:02}")).collect();
    if rows.len() != expected_ids.len() {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("the matrix has {} rows, expected 8", rows.len()),
        ));
    }
    let mut seen = BTreeSet::new();
    let mut simulated_ha = false;
    for (index, row) in rows.iter().enumerate() {
        let Some(row) = row.as_object() else {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("row {} is not an object", index + 1),
            ));
            continue;
        };
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("row {} has no id", index + 1),
            ));
            continue;
        };
        if expected_ids.get(index).map(String::as_str) != Some(id) {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!(
                    "row {} is {id}, expected {}",
                    index + 1,
                    expected_ids.get(index).map_or("none", String::as_str)
                ),
            ));
        }
        if !seen.insert(id.to_owned()) {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("{id}: duplicate row"),
            ));
        }
        if row.get("status").and_then(Value::as_str) == Some("Simulated HA") {
            simulated_ha = true;
        }
        check_row(root, path, id, row, diagnostics);
    }
    for expected in &expected_ids {
        if !seen.contains(expected) {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("{expected}: missing row"),
            ));
        }
    }
    check_risks(
        root,
        path,
        object.get("accepted_risks"),
        simulated_ha,
        diagnostics,
    );
}

fn check_system(
    root: &Path,
    path: &Path,
    system: Option<&Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(system) = system.and_then(Value::as_object) else {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "system must be an object",
        ));
        return;
    };
    let expected_keys = BTreeSet::from(["kind", "statement", "trading"]);
    let observed: BTreeSet<&str> = system.keys().map(String::as_str).collect();
    if observed != expected_keys {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "system must contain exactly kind, trading, and statement",
        ));
    }
    if system.get("kind").and_then(Value::as_str) != Some("research-only") {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "system.kind must be research-only",
        ));
    }
    if system.get("trading").and_then(Value::as_bool) != Some(false) {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "system.trading must be false; this gate never claims a trading system",
        ));
    }
    if !text(system, "statement") {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "system.statement must be a non-empty string",
        ));
    }
}

fn check_row(
    root: &Path,
    path: &Path,
    id: &str,
    row: &Map<String, Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in REQUIRED_ROW_FIELDS {
        if !row.contains_key(field) {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("{id}: missing {field}"),
            ));
        }
    }
    for key in row.keys() {
        if !REQUIRED_ROW_FIELDS.contains(&key.as_str())
            && !OPTIONAL_ROW_FIELDS.contains(&key.as_str())
        {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("{id}: unknown field {key}"),
            ));
        }
    }
    let Some(status) = row.get("status").and_then(Value::as_str) else {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("{id}: status must be a string"),
        ));
        return;
    };
    if !ROW_STATUSES.contains(&status) {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("{id}: unknown status {status:?}"),
        ));
        return;
    }
    for field in [
        "claim",
        "owner",
        "issue",
        "boundary",
        "recorded_on",
        "notes",
    ] {
        if !text(row, field) {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("{id}: {field} must be a non-empty string"),
            ));
        }
    }
    let Some(tier) = row.get("tier").and_then(Value::as_str) else {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("{id}: tier must be a string"),
        ));
        return;
    };
    if !TIERS.contains(&tier) {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("{id}: unknown tier {tier:?}"),
        ));
    }
    if PROVEN_STATUSES.contains(&status) && (!text(row, "test") || !evidence(row)) {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("{id}: {status} requires a non-empty test and evidence"),
        ));
    }
    if status == "Simulated HA" && tier != "Simulated HA" {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("{id}: Simulated HA requires tier=Simulated HA"),
        ));
    }
    if matches!(status, "Current" | "Measured") && matches!(tier, "Simulated HA" | "Real HA") {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("{id}: {status} cannot use the {tier} tier"),
        ));
    }
    match status {
        "Accepted risk" => {
            for field in ["accepted_by", "accepted_on", "residual_risk", "next_action"] {
                if !text(row, field) {
                    diagnostics.push(Diagnostic::path(
                        "research-readiness",
                        path,
                        root,
                        format!("{id}: Accepted risk requires {field}"),
                    ));
                }
            }
        }
        "Unverified" => {
            if !text(row, "next_action") {
                diagnostics.push(Diagnostic::path(
                    "research-readiness",
                    path,
                    root,
                    format!("{id}: Unverified requires next_action"),
                ));
            }
        }
        "Retired" if !text(row, "superseded_by") => {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("{id}: Retired requires superseded_by"),
            ));
        }
        _ => {}
    }
}

fn check_risks(
    root: &Path,
    path: &Path,
    risks: Option<&Value>,
    simulated_ha: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(risks) = risks.and_then(Value::as_array) else {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            "accepted_risks must be a list",
        ));
        return;
    };
    let mut seen = BTreeSet::new();
    let mut has_simulated = false;
    for (index, risk) in risks.iter().enumerate() {
        let Some(risk) = risk.as_object() else {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("accepted risk {} is not an object", index + 1),
            ));
            continue;
        };
        for field in RISK_FIELDS {
            if !risk.contains_key(field) {
                diagnostics.push(Diagnostic::path(
                    "research-readiness",
                    path,
                    root,
                    format!("accepted risk {}: missing {field}", index + 1),
                ));
            }
        }
        for key in risk.keys() {
            if !RISK_FIELDS.contains(&key.as_str()) {
                diagnostics.push(Diagnostic::path(
                    "research-readiness",
                    path,
                    root,
                    format!("accepted risk {}: unknown field {key}", index + 1),
                ));
            }
        }
        let id = risk.get("id").and_then(Value::as_str).unwrap_or("");
        if !text(risk, "id")
            || !text(risk, "risk")
            || !text(risk, "impact")
            || !text(risk, "residual_risk")
            || !text(risk, "next_action")
            || !text(risk, "accepted_on")
        {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!(
                    "accepted risk {}: required fields must be non-empty",
                    index + 1
                ),
            ));
        }
        if risk.get("accepted_by").and_then(Value::as_str) != Some("human") {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("accepted risk {}: accepted_by must be human", index + 1),
            ));
        }
        if !links(risk.get("links")) {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!(
                    "accepted risk {}: links must be a non-empty list",
                    index + 1
                ),
            ));
        }
        if !id.is_empty() && !seen.insert(id.to_owned()) {
            diagnostics.push(Diagnostic::path(
                "research-readiness",
                path,
                root,
                format!("accepted risk {id}: duplicate"),
            ));
        }
        if id == SIMULATED_RISK {
            has_simulated = true;
        }
    }
    if simulated_ha && !has_simulated {
        diagnostics.push(Diagnostic::path(
            "research-readiness",
            path,
            root,
            format!("a Simulated HA row requires the human-accepted {SIMULATED_RISK} risk"),
        ));
    }
}

fn text(object: &Map<String, Value>, field: &str) -> bool {
    object
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn evidence(row: &Map<String, Value>) -> bool {
    match row.get("evidence") {
        Some(Value::String(value)) => !value.trim().is_empty(),
        Some(Value::Array(items)) => {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|value| !value.trim().is_empty()))
        }
        _ => false,
    }
}

fn links(value: Option<&Value>) -> bool {
    value.and_then(Value::as_array).is_some_and(|items| {
        !items.is_empty()
            && items
                .iter()
                .all(|item| item.as_str().is_some_and(|value| !value.trim().is_empty()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture(name: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qr-research-readiness-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/config/2026-11-25")).expect("tree");
        root
    }

    fn row(id: &str) -> Value {
        json!({
            "id": id,
            "claim": "claim",
            "owner": "owner",
            "issue": "#1113",
            "test": "test",
            "evidence": ["docs/evidence.md"],
            "status": "Current",
            "tier": "Fast",
            "boundary": "boundary",
            "recorded_on": "2026-11-25",
            "notes": "notes"
        })
    }

    fn matrix() -> Value {
        json!({
            "schema": SCHEMA,
            "recorded_on": "2026-11-25",
            "system": {
                "kind": "research-only",
                "trading": false,
                "statement": "The system does not trade."
            },
            "rows": (1..=8).map(|n| row(&format!("RR-{n:02}"))).collect::<Vec<_>>(),
            "accepted_risks": [{
                "id": SIMULATED_RISK,
                "risk": "simulated",
                "impact": "impact",
                "accepted_by": "human",
                "accepted_on": "2026-11-25",
                "residual_risk": "residual",
                "next_action": "next",
                "links": ["docs/lab.md"]
            }]
        })
    }

    fn write(root: &Path, value: &Value) {
        fs::write(
            root.join(MATRIX),
            serde_json::to_string_pretty(value).expect("json"),
        )
        .expect("write");
    }

    #[test]
    fn a_missing_matrix_is_reported() {
        let root = fixture("missing");
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("is missing"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_valid_matrix_passes() {
        let root = fixture("valid");
        write(&root, &matrix());
        assert!(check(&root).is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_row_is_reported() {
        let root = fixture("missing-row");
        let mut value = matrix();
        value["rows"].as_array_mut().expect("rows").pop();
        write(&root, &value);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("the matrix has 7 rows"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_unknown_status_is_reported() {
        let root = fixture("status");
        let mut value = matrix();
        value["rows"][0]["status"] = json!("Done");
        write(&root, &value);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("unknown status"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn simulated_ha_without_an_accepted_risk_is_reported() {
        let root = fixture("simulated-risk");
        let mut value = matrix();
        value["rows"][3]["status"] = json!("Simulated HA");
        value["rows"][3]["tier"] = json!("Simulated HA");
        value["accepted_risks"] = json!([]);
        write(&root, &value);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains(SIMULATED_RISK))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_trading_system_is_reported() {
        let root = fixture("trading");
        let mut value = matrix();
        value["system"]["trading"] = json!(true);
        write(&root, &value);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("system.trading must be false"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_accepted_risk_needs_a_human() {
        let root = fixture("human");
        let mut value = matrix();
        value["accepted_risks"][0]["accepted_by"] = json!("agent");
        write(&root, &value);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("accepted_by must be human"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_duplicate_row_is_reported() {
        let root = fixture("duplicate");
        let mut value = matrix();
        value["rows"][7]["id"] = json!("RR-07");
        write(&root, &value);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("duplicate row"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
