//! Render the qualification bundle.
//!
//! The renderer copies the evaluated status, observed number, threshold, and
//! stable reason code into `qualification.json`, `criteria.csv`, and
//! `qualification.md`. It computes no statistic and writes no human decision.

use std::fmt::Write as _;

use serde_json::{Value, json};

use quantrule_app::config::{Criteria, Partition};

use super::evaluate::{Status, Verdict};
use super::evidence::{BacktestEvidence, MANIFEST_SCHEMA};

/// Renders `qualification.json`.
///
/// # Errors
///
/// Returns the serializer's error when the value cannot be encoded.
pub(crate) fn qualification_json_bytes(
    criteria: &Criteria,
    evidence: &BacktestEvidence,
    verdict: &Verdict<'_>,
) -> Result<Vec<u8>, serde_json::Error> {
    let value = json!({
        "schema": "quantrule.qualification-result/v1",
        "stage": "qualification",
        "id": &criteria.id,
        "verdict": verdict.status.code(),
        "criteria": verdict.outcomes.iter().map(criterion_value).collect::<Vec<Value>>(),
        "reasons": verdict
            .outcomes
            .iter()
            .filter(|outcome| outcome.status != Status::Pass)
            .map(|outcome| json!({
                "criterion_id": &outcome.criterion.id,
                "code": outcome.reason.code(),
            }))
            .collect::<Vec<Value>>(),
        "evidence": evidence_value(criteria, evidence),
        "not_applicable": criteria.not_applicable.iter().map(|entry| json!({
            "dimension": entry.dimension.code(),
            "reason": &entry.reason,
        })).collect::<Vec<Value>>(),
        "human_decision": Value::Null,
    });
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Renders `criteria.csv`.
#[must_use]
pub(crate) fn criteria_csv(verdict: &Verdict<'_>) -> String {
    let mut csv = String::from(
        "id,dimension,mandatory,status,observed,threshold,operator,evidence_artifact,evidence_field,reason_code\n",
    );
    for outcome in &verdict.outcomes {
        let criterion = outcome.criterion;
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{}",
            criterion.id,
            criterion.dimension.code(),
            criterion.mandatory,
            outcome.status.code(),
            optional_number(outcome.observed),
            number(criterion.threshold),
            criterion.operator.code(),
            criterion.evidence_artifact.code(),
            criterion.evidence_field,
            outcome.reason.code(),
        );
    }
    csv
}

/// Renders `qualification.md`.
#[must_use]
pub(crate) fn qualification_md(
    criteria: &Criteria,
    evidence: &BacktestEvidence,
    verdict: &Verdict<'_>,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Qualification — {}", criteria.id);
    let _ = writeln!(out);
    let _ = writeln!(out, "- Verdict: {}", verdict.status.code());
    let _ = writeln!(
        out,
        "- Criteria: {} total, {} pass, {} fail, {} insufficient_evidence",
        verdict.outcomes.len(),
        verdict.count(Status::Pass),
        verdict.count(Status::Fail),
        verdict.count(Status::InsufficientEvidence),
    );
    let _ = writeln!(
        out,
        "- Human decision: pending; the engine never accepts or rejects."
    );

    let _ = writeln!(out);
    let _ = writeln!(out, "## Evidence");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Data version: {}", evidence.data_version);
    let _ = writeln!(out, "- Config identity: {}", evidence.config_id);
    let _ = writeln!(out, "- Code revision: {}", evidence.code_commit);
    let _ = writeln!(
        out,
        "- Period: {}",
        period_label(
            evidence.period_from.as_deref(),
            evidence.period_to.as_deref()
        )
    );
    let _ = writeln!(out, "- Partitions: {}", partition_label(evidence, criteria));
    let _ = writeln!(
        out,
        "- Costs: fee_rate={} slippage_bps={} commission_rate={} funding_rate={} initial_cash={} leverage={}",
        number(evidence.costs.fee_rate),
        number(evidence.costs.slippage_bps),
        number(evidence.costs.commission_rate),
        number(evidence.costs.funding_rate),
        number(evidence.costs.initial_cash),
        number(evidence.costs.leverage),
    );
    let _ = writeln!(out, "- Seed: {}", seed_label(&evidence.seed));
    let _ = writeln!(out, "- Trades: {}", optional_count(evidence.trades));
    let _ = writeln!(
        out,
        "- Multiple testing: {} primary cells, {} trials, {} correction, family alpha {}, adjusted alpha {}",
        criteria.multiple_testing.primary_cells,
        criteria.multiple_testing.trials,
        criteria.multiple_testing.correction.code(),
        number(criteria.multiple_testing.family_alpha),
        number(criteria.multiple_testing.adjusted_alpha()),
    );

    let _ = writeln!(out);
    let _ = writeln!(out, "## Criteria");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| ID | Dimension | Mandatory | Status | Observed | Operator | Threshold | Evidence artifact | Evidence field | Reason |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|");
    for outcome in &verdict.outcomes {
        let criterion = outcome.criterion;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            criterion.id,
            criterion.dimension.code(),
            criterion.mandatory,
            outcome.status.code(),
            optional_number(outcome.observed),
            criterion.operator.code(),
            number(criterion.threshold),
            criterion.evidence_artifact.code(),
            criterion.evidence_field,
            outcome.reason.code(),
        );
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "## Reasons");
    let _ = writeln!(out);
    let mut reasons = verdict
        .outcomes
        .iter()
        .filter(|outcome| outcome.status != Status::Pass)
        .peekable();
    if reasons.peek().is_none() {
        let _ = writeln!(out, "None.");
    } else {
        for outcome in reasons {
            let _ = writeln!(
                out,
                "- `{}`: `{}`",
                outcome.criterion.id,
                outcome.reason.code()
            );
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "## Not applicable");
    let _ = writeln!(out);
    if criteria.not_applicable.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        for entry in &criteria.not_applicable {
            let _ = writeln!(out, "- `{}`: {}", entry.dimension.code(), entry.reason);
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "## Human acceptance");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "The engine writes `human_decision: null`. Review this record, the evidence bundle, and the residual risks, then record the decision in a separate `decision.md`. A `pass` may still be rejected for residual risk, and a `fail` is a valid research result."
    );
    out
}

/// Renders `provenance.json`.
///
/// # Errors
///
/// Returns the serializer's error when the value cannot be encoded.
pub(crate) fn provenance_json_bytes(
    criteria: &Criteria,
    criteria_id: &str,
    evidence: &BacktestEvidence,
    verdict: &Verdict<'_>,
) -> Result<Vec<u8>, serde_json::Error> {
    let value = json!({
        "schema": super::evidence::STAGE_SCHEMA,
        "stage": "qualification",
        "artifact": "qualification",
        "data_version": &evidence.data_version,
        "config_id": criteria_id,
        "code_commit": &evidence.code_commit,
        "source_artifacts": [{
            "stage": "backtest",
            "artifact": "backtest",
            "schema": MANIFEST_SCHEMA,
            "data_version": &evidence.data_version,
            "config_id": &evidence.config_id,
            "manifest_hash": &evidence.manifest_hash,
        }],
        "seed": &evidence.seed,
        "period": {
            "from": &evidence.period_from,
            "to": &evidence.period_to,
        },
        "artifacts": ["qualification.json", "criteria.csv", "qualification.md", "provenance.json"],
        "qualification": {
            "criteria_id": &criteria.id,
            "verdict": verdict.status.code(),
            "criteria_count": verdict.outcomes.len(),
            "mandatory_count": verdict.outcomes.iter().filter(|outcome| outcome.criterion.mandatory).count(),
            "partitions": criteria.required_partitions.iter().map(|partition| partition.code()).collect::<Vec<_>>(),
            "cost_model": criteria.cost_model.code(),
            "costs": {
                "fee_rate": evidence.costs.fee_rate,
                "slippage_bps": evidence.costs.slippage_bps,
                "commission_rate": evidence.costs.commission_rate,
                "funding_rate": evidence.costs.funding_rate,
            },
        },
    });
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn criterion_value(outcome: &super::evaluate::Outcome<'_>) -> Value {
    let criterion = outcome.criterion;
    json!({
        "id": &criterion.id,
        "dimension": criterion.dimension.code(),
        "mandatory": criterion.mandatory,
        "status": outcome.status.code(),
        "observed": outcome.observed,
        "threshold": criterion.threshold,
        "operator": criterion.operator.code(),
        "evidence_artifact": criterion.evidence_artifact.code(),
        "evidence_field": &criterion.evidence_field,
        "reason_code": outcome.reason.code(),
    })
}

fn evidence_value(criteria: &Criteria, evidence: &BacktestEvidence) -> Value {
    let all = evidence.partition_windows(&Partition::ALL);
    let partitions = all
        .iter()
        .filter(|window| window.covered())
        .map(|window| window.partition.code())
        .collect::<Vec<_>>();
    let required = evidence.partition_windows(&criteria.required_partitions);
    let coverage = required
        .iter()
        .map(|window| {
            json!({
                "partition": window.partition.code(),
                "from": window.from,
                "to": window.to,
                "overlap_from": &window.overlap_from,
                "overlap_to": &window.overlap_to,
                "covered": window.covered(),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "backtest_manifest": {
            "path": "manifest.json",
            "schema": MANIFEST_SCHEMA,
            "hash": &evidence.manifest_hash,
        },
        "data_version": &evidence.data_version,
        "config_id": &evidence.config_id,
        "code_commit": &evidence.code_commit,
        "period": {
            "from": &evidence.period_from,
            "to": &evidence.period_to,
        },
        "partitions": partitions,
        "required_partitions": criteria.required_partitions.iter().map(|partition| partition.code()).collect::<Vec<_>>(),
        "partition_coverage": coverage,
        "cost_model": criteria.cost_model.code(),
        "costs": {
            "fee_rate": evidence.costs.fee_rate,
            "slippage_bps": evidence.costs.slippage_bps,
            "commission_rate": evidence.costs.commission_rate,
            "funding_rate": evidence.costs.funding_rate,
            "initial_cash": evidence.costs.initial_cash,
            "leverage": evidence.costs.leverage,
        },
        "seed": &evidence.seed,
        "minimum_trades": criteria.minimum_trades,
        "observed_trades": evidence.trades,
        "multiple_testing": {
            "primary_cells": criteria.multiple_testing.primary_cells,
            "trials": criteria.multiple_testing.trials,
            "correction": criteria.multiple_testing.correction.code(),
            "family_alpha": criteria.multiple_testing.family_alpha,
            "adjusted_alpha": criteria.multiple_testing.adjusted_alpha(),
        },
        "stage_provenance": evidence.stages.iter().map(|stage| json!({
            "stage": &stage.stage,
            "path": &stage.path,
            "provenance": &stage.provenance,
            "provenance_hash": &stage.provenance_hash,
            "schema": &stage.schema,
            "data_version": &stage.data_version,
            "config_id": &stage.config_id,
        })).collect::<Vec<Value>>(),
    })
}

fn period_label(from: Option<&str>, to: Option<&str>) -> String {
    match (from, to) {
        (Some(from), Some(to)) => format!("{from}..{to}"),
        (Some(from), None) => format!("{from}..open"),
        (None, Some(to)) => format!("open..{to}"),
        (None, None) => "open..open".to_owned(),
    }
}

fn partition_label(evidence: &BacktestEvidence, criteria: &Criteria) -> String {
    let windows = evidence.partition_windows(&criteria.required_partitions);
    if windows.is_empty() {
        return "none".to_owned();
    }
    windows
        .iter()
        .map(|window| window.partition.code())
        .collect::<Vec<_>>()
        .join(", ")
}

fn seed_label(seed: &Value) -> String {
    if seed.is_null() {
        "null".to_owned()
    } else {
        seed.to_string()
    }
}

fn number(value: f64) -> String {
    value.to_string()
}

fn optional_number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), number)
}

fn optional_count(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |count| count.to_string())
}

#[cfg(test)]
mod tests {
    use super::{optional_count, optional_number, period_label, seed_label};
    use serde_json::json;

    #[test]
    fn numbers_are_rendered_without_a_trailing_zero() {
        assert_eq!(optional_number(Some(0.0)), "0");
        assert_eq!(optional_number(Some(1.0)), "1");
        assert_eq!(optional_number(Some(0.25)), "0.25");
        assert_eq!(optional_number(None), "null");
    }

    #[test]
    fn counts_are_rendered_as_integers() {
        assert_eq!(optional_count(Some(0)), "0");
        assert_eq!(
            optional_count(Some(9_007_199_254_740_993)),
            "9007199254740993"
        );
        assert_eq!(optional_count(None), "null");
    }

    #[test]
    fn an_open_period_is_labelled_as_open() {
        assert_eq!(
            period_label(Some("2025-01-01"), Some("2025-12-31")),
            "2025-01-01..2025-12-31"
        );
        assert_eq!(period_label(Some("2025-01-01"), None), "2025-01-01..open");
        assert_eq!(period_label(None, Some("2025-12-31")), "open..2025-12-31");
        assert_eq!(period_label(None, None), "open..open");
    }

    #[test]
    fn a_null_seed_is_labelled_null() {
        assert_eq!(seed_label(&json!(null)), "null");
        assert_eq!(seed_label(&json!(0)), "0");
    }
}
