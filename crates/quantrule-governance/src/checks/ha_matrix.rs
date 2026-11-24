//! Check the HA-014 chaos matrix.
//!
//! The matrix is the one record of the 23 failure rows. A row must carry an
//! explicit status, a test/evidence reference when it claims a proof, and the
//! fields that make a simulated, real-host, accepted-risk, or retired claim
//! honest. The checker never requires an evidence file to exist: a planned
//! path is a location, and `Unverified` is the honest status until a run writes
//! it.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::report::Diagnostic;

const MATRIX: &str = "docs/config/2026-11-24/chaos-matrix.json";
const SCHEMA: &str = "quantrule.ha-matrix/v1";
const ROW_STATUSES: [&str; 6] = [
    "Current",
    "Simulated HA",
    "Real HA",
    "Unverified",
    "Accepted risk",
    "Retired",
];
const REQUIRED_ROW_FIELDS: [&str; 14] = [
    "id",
    "claim",
    "owner",
    "issue",
    "test",
    "evidence",
    "status",
    "tier",
    "topology",
    "physical_hosts",
    "failure_domains",
    "recorded_on",
    "notes",
    "live_evidence",
];
const OPTIONAL_ROW_FIELDS: [&str; 3] = ["fingerprints", "attestation", "superseded_by"];
const PROVEN_STATUSES: [&str; 3] = ["Current", "Simulated HA", "Real HA"];
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

/// Check the HA-014 chaos matrix.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let path = root.join(MATRIX);
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![Diagnostic::path(
            "ha-matrix",
            &path,
            root,
            format!("{MATRIX} is missing"),
        )];
    };
    let Ok(matrix) = serde_json::from_str::<Value>(&text) else {
        return vec![Diagnostic::path(
            "ha-matrix",
            &path,
            root,
            "the chaos matrix is not valid JSON",
        )];
    };
    let mut diagnostics = Vec::new();
    check_top_level(root, &path, &matrix, &mut diagnostics);
    diagnostics
}

fn check_top_level(root: &Path, path: &Path, matrix: &Value, diagnostics: &mut Vec<Diagnostic>) {
    let Some(object) = matrix.as_object() else {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            "the chaos matrix must be a JSON object",
        ));
        return;
    };
    if object.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("schema must be {SCHEMA}"),
        ));
    }
    let expected_keys = BTreeSet::from(["accepted_risks", "rows", "schema"]);
    let observed: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    if observed != expected_keys {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            "the matrix must contain exactly schema, rows, and accepted_risks",
        ));
    }
    let Some(rows) = object.get("rows").and_then(Value::as_array) else {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            "rows must be a list",
        ));
        return;
    };
    let expected_ids: Vec<String> = (1..=23).map(|n| format!("HA-{n:02}")).collect();
    if rows.len() != expected_ids.len() {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("the matrix has {} rows, expected 23", rows.len()),
        ));
    }
    let mut seen = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        let Some(row) = row.as_object() else {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("row {} is not an object", index + 1),
            ));
            continue;
        };
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("row {} has no id", index + 1),
            ));
            continue;
        };
        if expected_ids.get(index).map(String::as_str) != Some(id) {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
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
                "ha-matrix",
                path,
                root,
                format!("{id}: duplicate row"),
            ));
        }
        check_row(root, path, id, row, diagnostics);
    }
    for expected in &expected_ids {
        if !seen.contains(expected) {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("{expected}: missing row"),
            ));
        }
    }
    check_risks(root, path, object.get("accepted_risks"), diagnostics);
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
                "ha-matrix",
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
                "ha-matrix",
                path,
                root,
                format!("{id}: unknown field {key}"),
            ));
        }
    }
    let status = row.get("status").and_then(Value::as_str).unwrap_or("");
    if !ROW_STATUSES.contains(&status) {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
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
        "tier",
        "topology",
        "recorded_on",
        "notes",
        "live_evidence",
    ] {
        if !text(row, field) {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("{id}: {field} must be a non-empty string"),
            ));
        }
    }
    if !number(row, "physical_hosts") {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: physical_hosts must be a non-negative integer"),
        ));
    }
    if !number(row, "failure_domains") {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: failure_domains must be a non-negative integer"),
        ));
    }
    if PROVEN_STATUSES.contains(&status) && (!text(row, "test") || !text(row, "evidence")) {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: {status} requires a non-empty test and evidence"),
        ));
    }
    match status {
        "Simulated HA" => {
            if row.get("topology").and_then(Value::as_str) != Some("lab-single-host")
                || row.get("physical_hosts").and_then(Value::as_u64) != Some(1)
            {
                diagnostics.push(Diagnostic::path(
                    "ha-matrix",
                    path,
                    root,
                    format!(
                        "{id}: Simulated HA requires topology=lab-single-host and physical_hosts=1"
                    ),
                ));
            }
        }
        "Real HA" => check_real_ha(root, path, id, row, diagnostics),
        "Accepted risk" => check_accepted_risk(root, path, id, row, diagnostics),
        "Retired" => {
            if !text(row, "superseded_by") {
                diagnostics.push(Diagnostic::path(
                    "ha-matrix",
                    path,
                    root,
                    format!("{id}: Retired requires superseded_by"),
                ));
            }
        }
        "Unverified" if !text(row, "notes") => {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("{id}: Unverified requires a reason in notes"),
            ));
        }
        _ => {}
    }
}

fn check_real_ha(
    root: &Path,
    path: &Path,
    id: &str,
    row: &Map<String, Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if row.get("topology").and_then(Value::as_str) == Some("lab-single-host") {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: Real HA cannot cite the lab-single-host topology"),
        ));
    }
    if row
        .get("physical_hosts")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < 3
    {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: Real HA requires physical_hosts >= 3"),
        ));
    }
    if row
        .get("failure_domains")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        < 3
    {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: Real HA requires failure_domains >= 3"),
        ));
    }
    let fingerprints = row
        .get("fingerprints")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.is_empty())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let total = row
        .get("fingerprints")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if fingerprints.len() < 3 || fingerprints.len() < total {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: Real HA requires at least three distinct non-empty fingerprints"),
        ));
    }
    let attestation = row.get("attestation").and_then(Value::as_object);
    let attested = attestation.is_some_and(|value| {
        value.get("by").and_then(Value::as_str) == Some("human")
            && value
                .get("signed_on")
                .and_then(Value::as_str)
                .is_some_and(|date| !date.is_empty())
            && value
                .get("statement")
                .and_then(Value::as_str)
                .is_some_and(|statement| !statement.is_empty())
    });
    if !attested {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: Real HA requires a human attestation with signed_on and statement"),
        ));
    }
}

fn check_accepted_risk(
    root: &Path,
    path: &Path,
    id: &str,
    row: &Map<String, Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in ["accepted_on", "residual_risk", "next_action"] {
        if !text(row, field) {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("{id}: Accepted risk requires {field}"),
            ));
        }
    }
    if row.get("accepted_by").and_then(Value::as_str) != Some("human") {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            format!("{id}: Accepted risk requires accepted_by=human"),
        ));
    }
}

fn check_risks(root: &Path, path: &Path, risks: Option<&Value>, diagnostics: &mut Vec<Diagnostic>) {
    let Some(risks) = risks.and_then(Value::as_array) else {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            "accepted_risks must be a list",
        ));
        return;
    };
    if risks.is_empty() {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            "accepted_risks is empty; the AZ1 MinIO/ingress risk is required",
        ));
        return;
    }
    let mut seen = BTreeSet::new();
    let mut az1 = false;
    for risk in risks {
        let Some(risk) = risk.as_object() else {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                "an accepted risk is not an object",
            ));
            continue;
        };
        for field in RISK_FIELDS {
            if !risk.contains_key(field) {
                diagnostics.push(Diagnostic::path(
                    "ha-matrix",
                    path,
                    root,
                    format!("an accepted risk is missing {field}"),
                ));
            }
        }
        for key in risk.keys() {
            if !RISK_FIELDS.contains(&key.as_str()) {
                diagnostics.push(Diagnostic::path(
                    "ha-matrix",
                    path,
                    root,
                    format!("an accepted risk has unknown field {key}"),
                ));
            }
        }
        let id = risk.get("id").and_then(Value::as_str).unwrap_or("");
        if id.is_empty() {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                "an accepted risk has no id",
            ));
        } else if !seen.insert(id.to_owned()) {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("{id}: duplicate accepted risk"),
            ));
        }
        if risk.get("accepted_by").and_then(Value::as_str) != Some("human") {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("{id}: accepted_by must be human"),
            ));
        }
        for field in [
            "accepted_on",
            "residual_risk",
            "next_action",
            "risk",
            "impact",
        ] {
            if !text(risk, field) {
                diagnostics.push(Diagnostic::path(
                    "ha-matrix",
                    path,
                    root,
                    format!("{id}: {field} must be a non-empty string"),
                ));
            }
        }
        if risk
            .get("links")
            .and_then(Value::as_array)
            .is_none_or(|links| {
                links.is_empty()
                    || links
                        .iter()
                        .any(|link| link.as_str().is_none_or(str::is_empty))
            })
        {
            diagnostics.push(Diagnostic::path(
                "ha-matrix",
                path,
                root,
                format!("{id}: links must be a non-empty list of strings"),
            ));
        }
        let text = format!(
            "{} {} {}",
            risk.get("risk").and_then(Value::as_str).unwrap_or(""),
            risk.get("impact").and_then(Value::as_str).unwrap_or(""),
            risk.get("residual_risk")
                .and_then(Value::as_str)
                .unwrap_or("")
        );
        if text.contains("AZ1") && text.contains("MinIO") && text.contains("ingress") {
            az1 = true;
        }
    }
    if !az1 {
        diagnostics.push(Diagnostic::path(
            "ha-matrix",
            path,
            root,
            "the AZ1 MinIO/ingress accepted risk is missing",
        ));
    }
}

fn text(row: &Map<String, Value>, field: &str) -> bool {
    row.get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

fn number(row: &Map<String, Value>, field: &str) -> bool {
    row.get(field).and_then(Value::as_u64).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture(name: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qr-ha-matrix-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/config/2026-11-24")).expect("tree");
        root
    }

    fn row(id: &str) -> Value {
        json!({
            "id": id,
            "claim": "a test claim",
            "owner": "HA-014",
            "issue": "#1112",
            "test": "deploy/v3/test.sh --chaos --confirm",
            "evidence": "docs/reports/2026-11-24/ha-014/test.json",
            "status": "Current",
            "tier": "static",
            "topology": "n/a",
            "physical_hosts": 0,
            "failure_domains": 0,
            "recorded_on": "2026-11-24",
            "notes": "a static proof; no live failure",
            "live_evidence": "Unverified"
        })
    }

    fn sample() -> Value {
        let rows: Vec<Value> = (1..=23).map(|n| row(&format!("HA-{n:02}"))).collect();
        json!({
            "schema": SCHEMA,
            "rows": rows,
            "accepted_risks": [{
                "id": "AR-AZ1-MINIO-INGRESS",
                "risk": "AZ1 holds two MinIO nodes and the ingress pair",
                "impact": "AZ1 loss blocks MinIO writes and removes ingress",
                "accepted_by": "human",
                "accepted_on": "2026-11-24",
                "residual_risk": "AZ1 loss is not survivable",
                "next_action": "PLAT-009 topology review",
                "links": ["docs/config/2026-11-24/reference-topologies.md"]
            }]
        })
    }

    fn write(root: &Path, matrix: &Value) -> PathBuf {
        let path = root.join(MATRIX);
        fs::write(&path, serde_json::to_string(matrix).expect("json")).expect("matrix");
        path
    }

    #[test]
    fn a_valid_matrix_passes() {
        let root = fixture("valid");
        write(&root, &sample());
        assert!(check(&root).is_empty(), "{:?}", check(&root));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_row_is_reported() {
        let root = fixture("missing-row");
        let mut matrix = sample();
        matrix["rows"].as_array_mut().expect("rows").pop();
        write(&root, &matrix);
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("23") || d.message.contains("HA-23")),
            "{diagnostics:?}"
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_unknown_status_is_reported() {
        let root = fixture("status");
        let mut matrix = sample();
        matrix["rows"][0]["status"] = json!("Blocked");
        write(&root, &matrix);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("unknown status"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_current_row_without_a_test_is_reported() {
        let root = fixture("no-test");
        let mut matrix = sample();
        matrix["rows"][0]["test"] = json!("");
        write(&root, &matrix);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("non-empty test and evidence"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_real_ha_row_citing_the_lab_is_reported() {
        let root = fixture("real-lab");
        let mut matrix = sample();
        matrix["rows"][21]["status"] = json!("Real HA");
        matrix["rows"][21]["topology"] = json!("lab-single-host");
        matrix["rows"][21]["physical_hosts"] = json!(1);
        matrix["rows"][21]["failure_domains"] = json!(3);
        matrix["rows"][21]["fingerprints"] = json!(["a", "b", "c"]);
        matrix["rows"][21]["attestation"] = json!({
            "by": "human",
            "signed_on": "2026-11-24",
            "statement": "observed"
        });
        write(&root, &matrix);
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("lab-single-host"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_az1_risk_is_reported() {
        let root = fixture("az1");
        let mut matrix = sample();
        matrix["accepted_risks"] = json!([]);
        write(&root, &matrix);
        assert!(check(&root).iter().any(|d| d.message.contains("AZ1")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
