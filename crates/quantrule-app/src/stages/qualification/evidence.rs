//! Read the backtest evidence bundle the Qualification Engine evaluates.
//!
//! The shared [`BundleManifest`] reader validates the manifest and every stage
//! provenance hash. This module adds the qualification-specific portfolio
//! metrics and cost declarations, plus the period-partition overlap. It does not
//! rerun a stage, read a candle, or trust a verdict from upstream.
//!
//! A missing manifest, an unsupported schema, a tampered provenance hash, or an
//! absent cost field is a refusal, not a criterion result. A measurement that is
//! simply missing from a present artifact is left to the evaluator, which
//! records `insufficient_evidence`.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::artifact::backtest::{BundleManifest, ManifestError, StageProvenance};
use crate::config::Partition;

pub use crate::artifact::backtest::{MANIFEST_SCHEMA, STAGE_SCHEMA};

/// The portfolio metrics schema.
pub const METRICS_SCHEMA: &str = "quantrule.portfolio-metrics/v1";
/// The model result schema.
pub const MODEL_RESULT_SCHEMA: &str = "quantrule.model-result/v4";

/// A backtest evidence bundle could not be read.
#[derive(Debug)]
pub enum EvidenceError {
    /// The shared manifest or stage provenance is invalid.
    Bundle(ManifestError),
    /// A required artifact could not be read.
    Read {
        /// The artifact path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A required artifact is not shaped like the documented bundle.
    Invalid {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong, with the artifact's own field names.
        message: String,
    },
    /// An artifact uses a schema the engine does not support.
    UnsupportedSchema {
        /// The artifact path.
        path: PathBuf,
        /// The expected schema.
        expected: &'static str,
        /// The observed schema.
        observed: String,
    },
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bundle(error) => error.fmt(formatter),
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "invalid evidence bundle {}: {message}",
                    path.display()
                )
            }
            Self::UnsupportedSchema {
                path,
                expected,
                observed,
            } => write!(
                formatter,
                "{} uses schema '{observed}', expected '{expected}'",
                path.display()
            ),
        }
    }
}

impl std::error::Error for EvidenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bundle(error) => Some(error),
            Self::Read { source, .. } => Some(source),
            Self::Invalid { .. } | Self::UnsupportedSchema { .. } => None,
        }
    }
}

/// The declared cost fields carried by the evidence.
#[derive(Debug, Clone, Copy)]
pub struct Costs {
    /// The execution fee rate.
    pub fee_rate: f64,
    /// The execution slippage in basis points.
    pub slippage_bps: f64,
    /// The portfolio commission rate.
    pub commission_rate: f64,
    /// The portfolio funding rate.
    pub funding_rate: f64,
    /// The initial cash.
    pub initial_cash: f64,
    /// The leverage.
    pub leverage: f64,
}

/// One required partition and its overlap with the evidence period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionWindow {
    /// The partition.
    pub partition: Partition,
    /// The partition start.
    pub from: &'static str,
    /// The partition end.
    pub to: &'static str,
    /// The inclusive overlap start, or `None` when the periods do not overlap.
    pub overlap_from: Option<String>,
    /// The inclusive overlap end, or `None` when the periods do not overlap.
    pub overlap_to: Option<String>,
}

impl PartitionWindow {
    /// Whether the evidence period overlaps the partition.
    #[must_use]
    pub fn covered(&self) -> bool {
        self.overlap_from.is_some() && self.overlap_to.is_some()
    }
}

/// A complete backtest evidence bundle for qualification.
#[derive(Debug, Clone)]
pub struct BacktestEvidence {
    /// The bundle root.
    root: PathBuf,
    /// The parsed manifest.
    pub manifest: Value,
    /// The FNV-1a hash of the manifest bytes.
    pub manifest_hash: String,
    /// The input data version.
    pub data_version: String,
    /// The run configuration identity.
    pub config_id: String,
    /// The code revision recorded by the build.
    pub code_commit: String,
    /// The explicit seed, or `null`.
    pub seed: Value,
    /// The requested period start, as written.
    pub period_from: Option<String>,
    /// The requested period end, as written.
    pub period_to: Option<String>,
    /// The four stage provenances, in canonical order.
    pub stages: Vec<StageProvenance>,
    /// The portfolio metrics document.
    pub metrics: Value,
    /// The execution provenance document.
    pub execution_provenance: Value,
    /// The declared costs.
    pub costs: Costs,
    /// The closed-trade count, when the metrics define it.
    pub trades: Option<u64>,
}

impl BacktestEvidence {
    /// Loads and validates a backtest bundle for qualification.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError`] when the shared bundle reader refuses, or the
    /// portfolio metrics or declared costs are missing or malformed.
    pub fn load(root: &Path) -> Result<Self, EvidenceError> {
        let bundle = BundleManifest::load(root).map_err(EvidenceError::Bundle)?;
        let execution = bundle
            .stage("execution")
            .ok_or_else(|| invalid(root, "the execution stage is absent"))?;
        let portfolio = bundle
            .stage("portfolio")
            .ok_or_else(|| invalid(root, "the portfolio stage is absent"))?;
        let execution_provenance = execution.value.clone();

        let execution_path = root.join(&execution.provenance);
        let execution_object = as_object(&execution_path, &execution_provenance)?;
        let execution_section = execution_object
            .get("execution")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid(&execution_path, "execution must be an object"))?;
        let fee_rate = required_number(&execution_path, execution_section, "fee_rate")?;
        let slippage_bps = required_number(&execution_path, execution_section, "slippage_bps")?;

        let portfolio_path = root.join(&portfolio.provenance);
        let portfolio_object = as_object(&portfolio_path, &portfolio.value)?;
        let portfolio_section = portfolio_object
            .get("portfolio")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid(&portfolio_path, "portfolio must be an object"))?;
        let commission_rate =
            required_number(&portfolio_path, portfolio_section, "commission_rate")?;
        let funding_rate = required_number(&portfolio_path, portfolio_section, "funding_rate")?;
        let initial_cash = required_number(&portfolio_path, portfolio_section, "initial_cash")?;
        let leverage = required_number(&portfolio_path, portfolio_section, "leverage")?;

        let metrics_path = root.join("portfolio/metrics.json");
        let (metrics, _) = read_json(&metrics_path)?;
        let metrics_object = as_object(&metrics_path, &metrics)?;
        let metrics_schema = required_str(&metrics_path, metrics_object, "schema")?;
        if metrics_schema != METRICS_SCHEMA {
            return Err(EvidenceError::UnsupportedSchema {
                path: metrics_path,
                expected: METRICS_SCHEMA,
                observed: metrics_schema.to_owned(),
            });
        }
        for field in ["execution_fees", "commission", "funding_paid"] {
            let present = metrics_object.get(field).and_then(Value::as_f64).is_some();
            if !present {
                return Err(invalid(
                    &metrics_path,
                    &format!("{field} must be a number for the declared cost model"),
                ));
            }
        }
        let trades = metrics_object.get("trades").and_then(Value::as_u64);

        Ok(Self {
            root: root.to_path_buf(),
            manifest: bundle.manifest,
            manifest_hash: bundle.manifest_hash,
            data_version: bundle.data_version,
            config_id: bundle.config_id,
            code_commit: bundle.code_commit,
            seed: bundle.seed,
            period_from: bundle.period_from,
            period_to: bundle.period_to,
            stages: bundle.stages,
            metrics,
            execution_provenance,
            costs: Costs {
                fee_rate,
                slippage_bps,
                commission_rate,
                funding_rate,
                initial_cash,
                leverage,
            },
            trades,
        })
    }

    /// Loads the model result document.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError`] when the result is missing, unreadable, not
    /// JSON, or uses an unsupported schema.
    pub fn model_result(&self) -> Result<Value, EvidenceError> {
        let path = self.root.join("model/result.json");
        let (value, _) = read_json(&path)?;
        let object = as_object(&path, &value)?;
        let schema = required_str(&path, object, "schema")?;
        if schema != MODEL_RESULT_SCHEMA {
            return Err(EvidenceError::UnsupportedSchema {
                path,
                expected: MODEL_RESULT_SCHEMA,
                observed: schema.to_owned(),
            });
        }
        Ok(value)
    }

    /// Computes each partition's overlap with the evidence period.
    #[must_use]
    pub fn partition_windows(&self, partitions: &[Partition]) -> Vec<PartitionWindow> {
        partitions
            .iter()
            .map(|partition| {
                let (from, to) = partition.window();
                let overlap = period_overlap(
                    self.period_from.as_deref(),
                    self.period_to.as_deref(),
                    from,
                    to,
                );
                let (overlap_from, overlap_to) = match overlap {
                    Some((start, end)) => (Some(start), Some(end)),
                    None => (None, None),
                };
                PartitionWindow {
                    partition: *partition,
                    from,
                    to,
                    overlap_from,
                    overlap_to,
                }
            })
            .collect()
    }
}

fn period_overlap(
    evidence_from: Option<&str>,
    evidence_to: Option<&str>,
    partition_from: &str,
    partition_to: &str,
) -> Option<(String, String)> {
    let evidence_from = evidence_from?;
    let evidence_to = evidence_to?;
    let start = evidence_from.max(partition_from);
    let end = evidence_to.min(partition_to);
    if start <= end {
        Some((start.to_owned(), end.to_owned()))
    } else {
        None
    }
}

fn read_json(path: &Path) -> Result<(Value, Vec<u8>), EvidenceError> {
    let bytes = std::fs::read(path).map_err(|source| EvidenceError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let value = serde_json::from_slice(&bytes).map_err(|error| EvidenceError::Invalid {
        path: path.to_path_buf(),
        message: format!("not JSON: {error}"),
    })?;
    Ok((value, bytes))
}

fn as_object<'a>(path: &Path, value: &'a Value) -> Result<&'a Map<String, Value>, EvidenceError> {
    value
        .as_object()
        .ok_or_else(|| invalid(path, "the document root must be an object"))
}

fn required_str<'a>(
    path: &Path,
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, EvidenceError> {
    object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(path, &format!("{name} must be a string")))
}

fn required_number(
    path: &Path,
    object: &Map<String, Value>,
    name: &str,
) -> Result<f64, EvidenceError> {
    let value = object
        .get(name)
        .and_then(Value::as_f64)
        .ok_or_else(|| invalid(path, &format!("{name} must be a finite number")))?;
    if !value.is_finite() {
        return Err(invalid(path, &format!("{name} must be a finite number")));
    }
    Ok(value)
}

fn invalid(path: &Path, message: &str) -> EvidenceError {
    EvidenceError::Invalid {
        path: path.to_path_buf(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::period_overlap;

    #[test]
    fn a_period_inside_a_partition_overlaps_it() {
        assert_eq!(
            period_overlap(
                Some("2025-01-01"),
                Some("2025-01-01"),
                "2025-01-01",
                "2025-12-31"
            ),
            Some(("2025-01-01".to_owned(), "2025-01-01".to_owned()))
        );
    }

    #[test]
    fn a_period_before_a_partition_does_not_overlap_it() {
        assert_eq!(
            period_overlap(
                Some("2023-01-01"),
                Some("2023-12-31"),
                "2025-01-01",
                "2025-12-31"
            ),
            None
        );
    }

    #[test]
    fn an_open_period_never_claims_partition_coverage() {
        assert_eq!(
            period_overlap(None, Some("2025-12-31"), "2025-01-01", "2025-12-31"),
            None
        );
        assert_eq!(
            period_overlap(Some("2025-01-01"), None, "2025-01-01", "2025-12-31"),
            None
        );
    }

    #[test]
    fn a_period_spanning_two_partitions_overlaps_both() {
        assert_eq!(
            period_overlap(
                Some("2024-06-01"),
                Some("2025-06-30"),
                "2024-01-01",
                "2024-12-31"
            ),
            Some(("2024-06-01".to_owned(), "2024-12-31".to_owned()))
        );
        assert_eq!(
            period_overlap(
                Some("2024-06-01"),
                Some("2025-06-30"),
                "2025-01-01",
                "2025-12-31"
            ),
            Some(("2025-01-01".to_owned(), "2025-06-30".to_owned()))
        );
    }
}
