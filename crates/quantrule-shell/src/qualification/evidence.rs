//! Read the backtest evidence bundle the Qualification Engine evaluates.
//!
//! The reader consumes `manifest.json`, the four stage provenance files, the
//! portfolio metrics, and the execution and portfolio cost declarations. It
//! checks that every stage provenance hash matches the manifest before a
//! criterion is evaluated. It does not rerun a stage, read a candle, or trust a
//! verdict from upstream.
//!
//! The bundle is the artifact handoff. A missing manifest, an unsupported
//! schema, a tampered provenance hash, or an absent cost field is a refusal, not
//! a criterion result. A measurement that is simply missing from a present
//! artifact is left to the evaluator, which records `insufficient_evidence`.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::config::Partition;
use crate::fingerprint::fnv1a;
use crate::source::DateBounds;

/// The backtest manifest schema.
pub(crate) const MANIFEST_SCHEMA: &str = "quantrule.backtest-manifest/v1";
/// The stage provenance schema for strategy, execution, portfolio, and later
/// stages.
pub(crate) const STAGE_SCHEMA: &str = "quantrule.stage-provenance/v1";
/// The existing model provenance schema.
pub(crate) const MODEL_PROVENANCE_SCHEMA: &str = "quantrule.provenance/v1";
/// The portfolio metrics schema.
pub(crate) const METRICS_SCHEMA: &str = "quantrule.portfolio-metrics/v1";
/// The model result schema.
pub(crate) const MODEL_RESULT_SCHEMA: &str = "quantrule.model-result/v4";

/// The canonical stage order.
const STAGES: [&str; 4] = ["model", "strategy", "execution", "portfolio"];

/// A backtest evidence bundle could not be read.
#[derive(Debug)]
pub(crate) enum EvidenceError {
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
    /// A stage provenance file does not match the manifest's recorded hash.
    HashMismatch {
        /// The stage name.
        stage: String,
        /// The provenance path.
        path: PathBuf,
        /// The hash recorded in the manifest.
        expected: String,
        /// The hash of the bytes on disk.
        observed: String,
    },
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::HashMismatch {
                stage,
                path,
                expected,
                observed,
            } => write!(
                formatter,
                "{} has provenance hash '{observed}', expected '{expected}' for stage {stage}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for EvidenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Invalid { .. } | Self::UnsupportedSchema { .. } | Self::HashMismatch { .. } => {
                None
            }
        }
    }
}

/// One stage's provenance, as recorded by the backtest manifest.
#[derive(Debug, Clone)]
pub(crate) struct StageEvidence {
    /// The stage name.
    pub(crate) stage: String,
    /// The stage's relative directory.
    pub(crate) path: String,
    /// The relative provenance path.
    pub(crate) provenance: String,
    /// The FNV-1a hash recorded in the manifest.
    pub(crate) provenance_hash: String,
    /// The provenance schema.
    pub(crate) schema: String,
    /// The input data version.
    pub(crate) data_version: String,
    /// The stage configuration identity.
    pub(crate) config_id: String,
    /// The parsed provenance value.
    value: Value,
}

/// The declared cost fields carried by the evidence.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Costs {
    /// The execution fee rate.
    pub(crate) fee_rate: f64,
    /// The execution slippage in basis points.
    pub(crate) slippage_bps: f64,
    /// The portfolio commission rate.
    pub(crate) commission_rate: f64,
    /// The portfolio funding rate.
    pub(crate) funding_rate: f64,
    /// The initial cash.
    pub(crate) initial_cash: f64,
    /// The leverage.
    pub(crate) leverage: f64,
}

/// One required partition and its overlap with the evidence period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartitionWindow {
    /// The partition.
    pub(crate) partition: Partition,
    /// The partition start.
    pub(crate) from: &'static str,
    /// The partition end.
    pub(crate) to: &'static str,
    /// The inclusive overlap start, or `None` when the periods do not overlap.
    pub(crate) overlap_from: Option<String>,
    /// The inclusive overlap end, or `None` when the periods do not overlap.
    pub(crate) overlap_to: Option<String>,
}

impl PartitionWindow {
    /// Whether the evidence period overlaps the partition.
    #[must_use]
    pub(crate) fn covered(&self) -> bool {
        self.overlap_from.is_some() && self.overlap_to.is_some()
    }
}

/// A complete backtest evidence bundle.
#[derive(Debug, Clone)]
pub(crate) struct BacktestEvidence {
    /// The bundle root.
    root: PathBuf,
    /// The parsed manifest.
    pub(crate) manifest: Value,
    /// The FNV-1a hash of the manifest bytes.
    pub(crate) manifest_hash: String,
    /// The input data version.
    pub(crate) data_version: String,
    /// The run configuration identity.
    pub(crate) config_id: String,
    /// The code revision recorded by the build.
    pub(crate) code_commit: String,
    /// The explicit seed, or `null`.
    pub(crate) seed: Value,
    /// The requested period start, as written.
    pub(crate) period_from: Option<String>,
    /// The requested period end, as written.
    pub(crate) period_to: Option<String>,
    /// The four stage provenances, in canonical order.
    pub(crate) stages: Vec<StageEvidence>,
    /// The portfolio metrics document.
    pub(crate) metrics: Value,
    /// The execution provenance document.
    pub(crate) execution_provenance: Value,
    /// The declared costs.
    pub(crate) costs: Costs,
    /// The closed-trade count, when the metrics define it.
    pub(crate) trades: Option<u64>,
}

impl BacktestEvidence {
    /// Loads and validates a backtest bundle.
    ///
    /// # Errors
    ///
    /// Returns [`EvidenceError`] when a required artifact is missing,
    /// unreadable, not JSON, carries an unsupported schema, disagrees with the
    /// manifest, or fails a recorded provenance hash.
    pub(crate) fn load(root: &Path) -> Result<Self, EvidenceError> {
        let manifest_path = root.join("manifest.json");
        let (manifest, manifest_bytes) = read_json(&manifest_path)?;
        let manifest_object = as_object(&manifest_path, &manifest)?;
        let schema = required_str(&manifest_path, manifest_object, "schema")?;
        if schema != MANIFEST_SCHEMA {
            return Err(EvidenceError::UnsupportedSchema {
                path: manifest_path,
                expected: MANIFEST_SCHEMA,
                observed: schema.to_owned(),
            });
        }

        let order = manifest_object
            .get("stage_order")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(&manifest_path, "stage_order must be an array"))?;
        let order = order
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| invalid(&manifest_path, "stage_order must contain strings"))?;
        if order.as_slice() != STAGES {
            return Err(invalid(
                &manifest_path,
                "stage_order must be model, strategy, execution, portfolio",
            ));
        }

        let data_version =
            required_non_empty_str(&manifest_path, manifest_object, "data_version")?.to_owned();
        let config_id =
            required_non_empty_str(&manifest_path, manifest_object, "config_id")?.to_owned();
        let code_commit =
            required_non_empty_str(&manifest_path, manifest_object, "code_commit")?.to_owned();
        let seed = manifest_object
            .get("seed")
            .cloned()
            .ok_or_else(|| invalid(&manifest_path, "seed is required"))?;
        if !seed.is_null() && seed.as_u64().is_none() {
            return Err(invalid(
                &manifest_path,
                "seed must be null or a non-negative integer",
            ));
        }

        let period = manifest_object
            .get("period")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid(&manifest_path, "period must be an object"))?;
        let period_from = optional_date(&manifest_path, period, "from")?;
        let period_to = optional_date(&manifest_path, period, "to")?;
        DateBounds::parse(period_from.as_deref(), period_to.as_deref())
            .map_err(|error| invalid(&manifest_path, &format!("period is invalid: {error}")))?;

        let stages_value = manifest_object
            .get("stages")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid(&manifest_path, "stages must be an array"))?;
        if stages_value.len() != STAGES.len() {
            return Err(invalid(
                &manifest_path,
                "stages must name exactly the four pipeline stages",
            ));
        }
        let mut stages = Vec::with_capacity(STAGES.len());
        for (index, value) in stages_value.iter().enumerate() {
            let label = format!("$.stages[{index}]");
            let object = value
                .as_object()
                .ok_or_else(|| invalid(&manifest_path, &format!("{label} must be an object")))?;
            let stage = required_str(&manifest_path, object, "stage")?;
            if !STAGES.contains(&stage) {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.stage '{stage}' is not a pipeline stage"),
                ));
            }
            if stages
                .iter()
                .any(|evidence: &StageEvidence| evidence.stage == stage)
            {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.stage '{stage}' is duplicated"),
                ));
            }
            let path = required_str(&manifest_path, object, "path")?;
            if path != stage {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.path must equal the stage name"),
                ));
            }
            let provenance = required_str(&manifest_path, object, "provenance")?;
            let expected_provenance = format!("{stage}/provenance.json");
            if provenance != expected_provenance {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.provenance must be '{expected_provenance}'"),
                ));
            }
            let hash = required_str(&manifest_path, object, "provenance_hash")?;
            if hash.len() != 16 || !hash.bytes().all(is_lower_hex) {
                return Err(invalid(
                    &manifest_path,
                    &format!("{label}.provenance_hash must be 16 lowercase hex digits"),
                ));
            }

            let provenance_path = root.join(provenance);
            let (provenance_value, provenance_bytes) = read_json(&provenance_path)?;
            let observed = fnv1a(&provenance_bytes);
            if observed != hash {
                return Err(EvidenceError::HashMismatch {
                    stage: stage.to_owned(),
                    path: provenance_path,
                    expected: hash.to_owned(),
                    observed,
                });
            }
            let provenance_object = as_object(&provenance_path, &provenance_value)?;
            let provenance_schema = required_str(&provenance_path, provenance_object, "schema")?;
            let expected_schema = if stage == "model" {
                MODEL_PROVENANCE_SCHEMA
            } else {
                STAGE_SCHEMA
            };
            if provenance_schema != expected_schema {
                return Err(EvidenceError::UnsupportedSchema {
                    path: provenance_path,
                    expected: expected_schema,
                    observed: provenance_schema.to_owned(),
                });
            }
            if stage != "model" {
                let provenance_stage = required_str(&provenance_path, provenance_object, "stage")?;
                if provenance_stage != stage {
                    return Err(invalid(
                        &provenance_path,
                        &format!("stage is '{provenance_stage}', expected '{stage}'"),
                    ));
                }
            }
            let stage_data_version =
                required_non_empty_str(&provenance_path, provenance_object, "data_version")?;
            if stage_data_version != data_version {
                return Err(invalid(
                    &provenance_path,
                    &format!("data_version is '{stage_data_version}', expected '{data_version}'"),
                ));
            }
            let config_field = if stage == "model" {
                "config_hash"
            } else {
                "config_id"
            };
            let stage_config_id =
                required_non_empty_str(&provenance_path, provenance_object, config_field)?;
            if stage != "strategy" && stage_config_id != config_id {
                return Err(invalid(
                    &provenance_path,
                    &format!("{config_field} is '{stage_config_id}', expected '{config_id}'"),
                ));
            }
            stages.push(StageEvidence {
                stage: stage.to_owned(),
                path: path.to_owned(),
                provenance: provenance.to_owned(),
                provenance_hash: hash.to_owned(),
                schema: provenance_schema.to_owned(),
                data_version: stage_data_version.to_owned(),
                config_id: stage_config_id.to_owned(),
                value: provenance_value,
            });
        }

        let execution = stages
            .iter()
            .find(|stage| stage.stage == "execution")
            .ok_or_else(|| invalid(&manifest_path, "the execution stage is absent"))?;
        let portfolio = stages
            .iter()
            .find(|stage| stage.stage == "portfolio")
            .ok_or_else(|| invalid(&manifest_path, "the portfolio stage is absent"))?;
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
            manifest,
            manifest_hash: fnv1a(&manifest_bytes),
            data_version,
            config_id,
            code_commit,
            seed,
            period_from,
            period_to,
            stages,
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
    pub(crate) fn model_result(&self) -> Result<Value, EvidenceError> {
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
    pub(crate) fn partition_windows(&self, partitions: &[Partition]) -> Vec<PartitionWindow> {
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

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
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

fn required_non_empty_str<'a>(
    path: &Path,
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, EvidenceError> {
    let value = required_str(path, object, name)?;
    if value.is_empty() {
        return Err(invalid(path, &format!("{name} must be non-empty")));
    }
    Ok(value)
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

fn optional_date(
    path: &Path,
    object: &Map<String, Value>,
    name: &str,
) -> Result<Option<String>, EvidenceError> {
    match object.get(name) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(invalid(
            path,
            &format!("period.{name} must be a string or null"),
        )),
    }
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
