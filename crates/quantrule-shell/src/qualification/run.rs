//! The `qualify` command: evaluate evidence against human-owned criteria.
//!
//! The command loads the criteria and the backtest evidence, resolves every
//! criterion against the named artifact field, evaluates the verdict, and writes
//! the qualification bundle. It never writes `decision.md` and never accepts a
//! strategy.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::render_report::{self, BundleWriteError};
use crate::request::QualifyArgs;
use quantrule_app::config::{ConfigError, EvidenceArtifact, load_qualification};

use super::evaluate::{Status, evaluate, resolve};
use super::evidence::{BacktestEvidence, EvidenceError, PartitionWindow};
use super::render;

/// The qualification command failed.
#[derive(Debug)]
pub(crate) enum QualificationRunError {
    /// The criteria file is invalid.
    Config(ConfigError),
    /// The evidence bundle is missing or invalid.
    Evidence(EvidenceError),
    /// The qualification bundle could not be written.
    Bundle(BundleWriteError),
    /// A qualification artifact could not be encoded.
    Json(serde_json::Error),
    /// An internal invariant was violated.
    Invariant(&'static str),
    /// The writer failed.
    Output(io::Error),
}

impl fmt::Display for QualificationRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Evidence(error) => error.fmt(formatter),
            Self::Bundle(error) => error.fmt(formatter),
            Self::Json(error) => {
                write!(
                    formatter,
                    "cannot encode the qualification artifact: {error}"
                )
            }
            Self::Invariant(message) => {
                write!(formatter, "qualification invariant violated: {message}")
            }
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for QualificationRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Evidence(error) => Some(error),
            Self::Bundle(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Invariant(_) => None,
            Self::Output(error) => Some(error),
        }
    }
}

/// Runs `qualify`.
///
/// # Errors
///
/// Returns [`QualificationRunError`] when the criteria, the evidence bundle, the
/// output target, or the writer fails.
pub(crate) fn run(
    args: &QualifyArgs,
    writer: &mut impl Write,
) -> Result<(), QualificationRunError> {
    let (criteria, criteria_id) =
        load_qualification(&args.criteria).map_err(QualificationRunError::Config)?;
    let evidence =
        BacktestEvidence::load(&args.evidence).map_err(QualificationRunError::Evidence)?;

    let required = evidence.partition_windows(&criteria.required_partitions);
    let partitions_covered = required.iter().all(PartitionWindow::covered);

    let mut documents: BTreeMap<EvidenceArtifact, Value> = BTreeMap::new();
    for criterion in &criteria.criteria {
        if documents.contains_key(&criterion.evidence_artifact) {
            continue;
        }
        let document = match criterion.evidence_artifact {
            EvidenceArtifact::Backtest => evidence.manifest.clone(),
            EvidenceArtifact::Portfolio => evidence.metrics.clone(),
            EvidenceArtifact::Execution => evidence.execution_provenance.clone(),
            EvidenceArtifact::Model => evidence
                .model_result()
                .map_err(QualificationRunError::Evidence)?,
        };
        documents.insert(criterion.evidence_artifact, document);
    }

    let mut observations = Vec::with_capacity(criteria.criteria.len());
    for criterion in &criteria.criteria {
        let document =
            documents
                .get(&criterion.evidence_artifact)
                .ok_or(QualificationRunError::Invariant(
                    "a referenced artifact was not loaded",
                ))?;
        observations.push(resolve(
            document,
            criterion
                .evidence_artifact
                .field_path(&criterion.evidence_field),
        ));
    }

    let verdict = evaluate(
        &criteria,
        &observations,
        partitions_covered,
        evidence.trades,
    );

    let qualification = render::qualification_json_bytes(&criteria, &evidence, &verdict)
        .map_err(QualificationRunError::Json)?;
    let csv = render::criteria_csv(&verdict).into_bytes();
    let markdown = render::qualification_md(&criteria, &evidence, &verdict).into_bytes();
    let provenance = render::provenance_json_bytes(&criteria, &criteria_id, &evidence, &verdict)
        .map_err(QualificationRunError::Json)?;

    let out = args
        .out
        .clone()
        .or_else(|| args.evidence.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    let root = out.join("qualification");
    let artifacts = [
        ("criteria.csv", csv),
        ("qualification.md", markdown),
        ("qualification.json", qualification),
    ];
    render_report::write_stage_bundle(&root, &artifacts, &provenance, "qualification.json")
        .map_err(QualificationRunError::Bundle)?;

    let summary = format!(
        "total={} pass={} fail={} insufficient_evidence={}",
        verdict.outcomes.len(),
        verdict.count(Status::Pass),
        verdict.count(Status::Fail),
        verdict.count(Status::InsufficientEvidence),
    );
    if args.json {
        let value = json!({
            "schema": "quantrule.qualification-result/v1",
            "bundle": &root,
            "qualification": root.join("qualification.json"),
            "verdict": verdict.status.code(),
            "criteria": {
                "total": verdict.outcomes.len(),
                "pass": verdict.count(Status::Pass),
                "fail": verdict.count(Status::Fail),
                "insufficient_evidence": verdict.count(Status::InsufficientEvidence),
            },
            "human_decision": Value::Null,
        });
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&value).map_err(QualificationRunError::Json)?
        )
        .map_err(QualificationRunError::Output)?;
    } else {
        writeln!(writer, "verdict: {}", verdict.status.code())
            .map_err(QualificationRunError::Output)?;
        writeln!(writer, "criteria: {summary}").map_err(QualificationRunError::Output)?;
        writeln!(
            writer,
            "qualification: {}",
            root.join("qualification.json").display()
        )
        .map_err(QualificationRunError::Output)?;
        writeln!(writer, "human_decision: pending").map_err(QualificationRunError::Output)?;
    }
    writer.flush().map_err(QualificationRunError::Output)
}
