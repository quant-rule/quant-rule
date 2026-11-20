//! Improvement-run pre-registration and research-ledger projections.
//!
//! The v3 manifest is the human-owned pre-registration. It carries gateway
//! references only: a dataset digest, a config id, and an optional criteria id.
//! It never carries a candle, a reading, a metric, or a filesystem path. The
//! pipeline run link and the human decision are stored separately by evidence.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::digest::Digest;

/// The current improvement-run manifest schema.
pub const IMPROVEMENT_RUN_SCHEMA: &str = "quantrule.improvement-run/v3";
/// The historical schema, refused by the v3 validator.
pub const IMPROVEMENT_RUN_SCHEMA_V2: &str = "quantrule.improvement-run/v2";

/// One research tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementTier {
    /// Model research.
    Model,
    /// Strategy research.
    Strategy,
    /// Portfolio research.
    Portfolio,
}

impl ImprovementTier {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Strategy => "strategy",
            Self::Portfolio => "portfolio",
        }
    }

    /// The improvement-id prefix.
    #[must_use]
    pub const fn id_prefix(self) -> &'static str {
        match self {
            Self::Model => "MODEL",
            Self::Strategy => "STRATEGY",
            Self::Portfolio => "PORTFOLIO",
        }
    }

    /// Parse the wire token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "model" => Some(Self::Model),
            "strategy" => Some(Self::Strategy),
            "portfolio" => Some(Self::Portfolio),
            _ => None,
        }
    }
}

/// One declared research partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementPartition {
    /// Model development.
    ModelDev,
    /// Model validation.
    ModelValidation,
    /// Strategy development.
    StrategyDev,
    /// Strategy validation.
    StrategyValidation,
    /// Portfolio development.
    PortfolioDev,
    /// Portfolio holdout.
    PortfolioHoldout,
}

impl ImprovementPartition {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelDev => "model_dev",
            Self::ModelValidation => "model_validation",
            Self::StrategyDev => "strategy_dev",
            Self::StrategyValidation => "strategy_validation",
            Self::PortfolioDev => "portfolio_dev",
            Self::PortfolioHoldout => "portfolio_holdout",
        }
    }

    /// Parse the wire token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "model_dev" => Some(Self::ModelDev),
            "model_validation" => Some(Self::ModelValidation),
            "strategy_dev" => Some(Self::StrategyDev),
            "strategy_validation" => Some(Self::StrategyValidation),
            "portfolio_dev" => Some(Self::PortfolioDev),
            "portfolio_holdout" => Some(Self::PortfolioHoldout),
            _ => None,
        }
    }

    /// The declared inclusive date window, as `YYYY-MM-DD` strings.
    ///
    /// The windows mirror `data/partitions.json`; the `check_partitions`
    /// checker and the improvement test keep the two in step.
    #[must_use]
    pub const fn window(self) -> (&'static str, &'static str) {
        match self {
            Self::ModelDev => ("2018-01-01", "2020-12-31"),
            Self::ModelValidation => ("2021-01-01", "2022-12-31"),
            Self::StrategyDev => ("2018-01-01", "2022-12-31"),
            Self::StrategyValidation => ("2024-01-01", "2024-12-31"),
            Self::PortfolioDev => ("2024-01-01", "2024-12-31"),
            Self::PortfolioHoldout => ("2025-01-01", "2025-12-31"),
        }
    }

    /// The half-open Unix-second window, `[start, end)`, for a bar timestamp.
    ///
    /// The declared end date is inclusive, so the exclusive end is the next
    /// UTC midnight.
    #[must_use]
    pub fn unix_window(self) -> (u64, u64) {
        let (from, to) = self.window();
        (date_to_epoch(from), date_to_epoch(to) + 86_400)
    }
}

/// Convert a `YYYY-MM-DD` UTC date to Unix seconds.
///
/// Howard Hinnant's civil-date algorithm; it is pure integer arithmetic and
/// does not read a clock. The input is validated by the manifest/config parser.
fn date_to_epoch(date: &str) -> u64 {
    let mut parts = date.split('-');
    let year: i64 = parts.next().unwrap_or("1970").parse().unwrap_or(1970);
    let month: i64 = parts.next().unwrap_or("1").parse().unwrap_or(1);
    let day: i64 = parts.next().unwrap_or("1").parse().unwrap_or(1);
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_index = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let seconds = (era * 146_097 + day_of_era - 719_468) * 86_400;
    u64::try_from(seconds).unwrap_or(0)
}

/// Whether the one-shot holdout was opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldoutAccess {
    /// The holdout remains closed.
    None,
    /// This is the final portfolio holdout.
    Final,
}

impl HoldoutAccess {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Final => "final",
        }
    }
}

/// The v3 improvement-run pre-registration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImprovementManifest {
    /// `quantrule.improvement-run/v3`.
    pub schema: String,
    /// `TIER-YYYY-MM-DD-NN`.
    pub improvement_id: String,
    /// The tier being changed.
    pub tier: ImprovementTier,
    /// One falsifiable sentence.
    pub hypothesis: String,
    /// RFC 3339 UTC creation time.
    pub created_utc: String,
    /// The tier that changes; must equal `tier`.
    pub changed: ImprovementTier,
    /// Tiers held fixed; must not contain `changed`.
    pub frozen: Vec<ImprovementTier>,
    /// The declared partition.
    pub partition: ImprovementPartition,
    /// The gateway dataset reference.
    pub data_version: Digest,
    /// The gateway configuration reference.
    pub config_id: String,
    /// The qualification-criteria reference, required for strategy/portfolio.
    pub criteria_id: Option<String>,
    /// The non-empty green-criteria object.
    pub green_criteria: Map<String, Value>,
    /// Whether the one-shot holdout is opened.
    pub holdout_access: HoldoutAccess,
    /// The accepted baseline run, when one exists.
    pub baseline: Option<String>,
    /// The repository commit at pre-registration.
    pub code_commit: String,
    /// The deterministic seed.
    pub seed: u64,
    /// The only accepted execution command, when present.
    pub command: Option<String>,
    /// Whether analytics output is part of the run.
    pub analytics: Option<bool>,
}

impl ImprovementManifest {
    /// Parse and validate one v3 manifest.
    ///
    /// # Errors
    ///
    /// Returns [`ImprovementManifestError`] when the schema is historical, a
    /// field is empty, a reference looks like a path, or a tier/partition rule
    /// is broken.
    pub fn parse(value: &Value) -> Result<Self, ImprovementManifestError> {
        if value.get("schema").and_then(Value::as_str) == Some(IMPROVEMENT_RUN_SCHEMA_V2) {
            return Err(ImprovementManifestError(
                "quantrule.improvement-run/v2 is historical; use v3".to_owned(),
            ));
        }
        let manifest: Self = serde_json::from_value(value.clone())
            .map_err(|error| ImprovementManifestError(format!("invalid manifest JSON: {error}")))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest rules.
    ///
    /// # Errors
    ///
    /// Returns [`ImprovementManifestError`] naming the first broken rule.
    pub fn validate(&self) -> Result<(), ImprovementManifestError> {
        if self.schema == IMPROVEMENT_RUN_SCHEMA_V2 {
            return Err(ImprovementManifestError(
                "quantrule.improvement-run/v2 is historical; use v3".to_owned(),
            ));
        }
        if self.schema != IMPROVEMENT_RUN_SCHEMA {
            return Err(ImprovementManifestError(format!(
                "the manifest schema is {}, not {IMPROVEMENT_RUN_SCHEMA}",
                self.schema
            )));
        }
        validate_improvement_id(&self.improvement_id, self.tier)?;
        required("hypothesis", &self.hypothesis)?;
        required("code_commit", &self.code_commit)?;
        required_reference("config_id", &self.config_id, false)?;
        if let Some(criteria) = &self.criteria_id {
            required_reference("criteria_id", criteria, false)?;
        } else if matches!(
            self.tier,
            ImprovementTier::Strategy | ImprovementTier::Portfolio
        ) {
            return Err(ImprovementManifestError(
                "criteria_id is required for a strategy or portfolio run".to_owned(),
            ));
        }
        if self.changed != self.tier {
            return Err(ImprovementManifestError(
                "changed must equal tier".to_owned(),
            ));
        }
        let mut seen = BTreeSet::new();
        for tier in &self.frozen {
            if *tier == self.changed {
                return Err(ImprovementManifestError(
                    "frozen must not contain the changed tier".to_owned(),
                ));
            }
            if !seen.insert(*tier) {
                return Err(ImprovementManifestError(
                    "frozen must not contain a duplicate tier".to_owned(),
                ));
            }
        }
        let expected_frozen: BTreeSet<ImprovementTier> = match self.tier {
            ImprovementTier::Model => [ImprovementTier::Strategy, ImprovementTier::Portfolio],
            ImprovementTier::Strategy => [ImprovementTier::Model, ImprovementTier::Portfolio],
            ImprovementTier::Portfolio => [ImprovementTier::Model, ImprovementTier::Strategy],
        }
        .into_iter()
        .collect();
        if seen != expected_frozen {
            return Err(ImprovementManifestError(
                "frozen must list exactly the tiers held fixed for this run".to_owned(),
            ));
        }
        let expected_tier = match self.partition {
            ImprovementPartition::ModelDev | ImprovementPartition::ModelValidation => {
                ImprovementTier::Model
            }
            ImprovementPartition::StrategyDev | ImprovementPartition::StrategyValidation => {
                ImprovementTier::Strategy
            }
            ImprovementPartition::PortfolioDev | ImprovementPartition::PortfolioHoldout => {
                ImprovementTier::Portfolio
            }
        };
        if self.tier != expected_tier {
            return Err(ImprovementManifestError(format!(
                "partition {} belongs to the {} tier, not {}",
                self.partition.as_str(),
                expected_tier.as_str(),
                self.tier.as_str()
            )));
        }
        let holdout = match self.partition {
            ImprovementPartition::PortfolioHoldout => HoldoutAccess::Final,
            _ => HoldoutAccess::None,
        };
        if self.holdout_access != holdout {
            return Err(ImprovementManifestError(format!(
                "holdout_access must be {} for {}",
                holdout.as_str(),
                self.partition.as_str()
            )));
        }
        if !is_rfc3339_utc(&self.created_utc) {
            return Err(ImprovementManifestError(
                "created_utc must be an RFC 3339 UTC timestamp".to_owned(),
            ));
        }
        let id_date = self
            .improvement_id
            .split('-')
            .skip(1)
            .take(3)
            .collect::<Vec<_>>()
            .join("-");
        let created_date = self
            .created_utc
            .split_once('T')
            .map(|(date, _)| date)
            .unwrap_or("");
        if created_date != id_date {
            return Err(ImprovementManifestError(
                "created_utc must match the date in improvement_id".to_owned(),
            ));
        }
        if self.green_criteria.is_empty() {
            return Err(ImprovementManifestError(
                "green_criteria must be a non-empty object".to_owned(),
            ));
        }
        if let Some(command) = &self.command
            && command != "run submit"
        {
            return Err(ImprovementManifestError(
                "command must be run submit when present".to_owned(),
            ));
        }
        if let Some(baseline) = &self.baseline {
            required_reference("baseline", baseline, true)?;
        }
        Ok(())
    }
}

/// A manifest validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImprovementManifestError(pub String);

impl fmt::Display for ImprovementManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ImprovementManifestError {}

/// The ledger lifecycle of one improvement run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementRunState {
    /// The pre-registration exists; no pipeline run is linked.
    Created,
    /// A pipeline run and its canonical digest are linked.
    Submitted,
    /// The human decision is recorded.
    Decided,
}

impl ImprovementRunState {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Submitted => "submitted",
            Self::Decided => "decided",
        }
    }

    /// Parse the wire token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "created" => Some(Self::Created),
            "submitted" => Some(Self::Submitted),
            "decided" => Some(Self::Decided),
            _ => None,
        }
    }
}

/// The human decision on one improvement run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImprovementDecision {
    /// Accepted as evidence.
    Accepted,
    /// Rejected.
    Rejected,
    /// No edge; stop the line.
    NoEdge,
    /// Invalidated by a later finding.
    Invalidated,
    /// Requires a new hypothesis.
    NewHypothesis,
}

impl ImprovementDecision {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::NoEdge => "no-edge",
            Self::Invalidated => "invalidated",
            Self::NewHypothesis => "new-hypothesis",
        }
    }

    /// Parse the wire token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            "no-edge" => Some(Self::NoEdge),
            "invalidated" => Some(Self::Invalidated),
            "new-hypothesis" => Some(Self::NewHypothesis),
            _ => None,
        }
    }
}

/// One improvement run summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImprovementRunSummary {
    /// The pre-registration identity.
    pub improvement_id: String,
    /// The changed tier.
    pub tier: ImprovementTier,
    /// The lifecycle state.
    pub state: ImprovementRunState,
    /// The decision, once decided.
    pub decision: Option<ImprovementDecision>,
    /// The pre-registration time.
    pub created_at: String,
    /// The pipeline link time.
    pub submitted_at: Option<String>,
    /// The human decision time.
    pub decided_at: Option<String>,
    /// When the profile archived the record, RFC 3339 UTC. Absent while active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,
}

/// One improvement run detail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImprovementRunDetail {
    /// The pre-registration identity.
    pub improvement_id: String,
    /// The changed tier.
    pub tier: ImprovementTier,
    /// The lifecycle state.
    pub state: ImprovementRunState,
    /// The decision, once decided.
    pub decision: Option<ImprovementDecision>,
    /// The human note, once decided.
    pub decision_note: Option<String>,
    /// The immutable pre-registration.
    pub manifest: ImprovementManifest,
    /// The linked pipeline run, after submission.
    pub pipeline_run_id: Option<crate::run::RunId>,
    /// The linked canonical digest, after submission.
    pub canonical_digest: Option<Digest>,
    /// The pre-registration time.
    pub created_at: String,
    /// The pipeline link time.
    pub submitted_at: Option<String>,
    /// The human decision time.
    pub decided_at: Option<String>,
}

impl ImprovementRunDetail {
    /// The list projection.
    #[must_use]
    pub fn summary(&self) -> ImprovementRunSummary {
        ImprovementRunSummary {
            improvement_id: self.improvement_id.clone(),
            tier: self.tier,
            state: self.state,
            decision: self.decision,
            created_at: self.created_at.clone(),
            submitted_at: self.submitted_at.clone(),
            decided_at: self.decided_at.clone(),
            archived_at: None,
        }
    }
}

/// One request to create a v3 improvement-run pre-registration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateImprovementRequest {
    /// The tier that changes.
    pub tier: ImprovementTier,
    /// One falsifiable sentence.
    pub hypothesis: String,
    /// The declared partition.
    pub partition: ImprovementPartition,
    /// The gateway dataset reference.
    pub data_version: Digest,
    /// The gateway configuration reference.
    pub config_id: String,
    /// The qualification-criteria reference, required for strategy/portfolio.
    #[serde(default)]
    pub criteria_id: Option<String>,
    /// The non-empty green-criteria object.
    pub green_criteria: Map<String, Value>,
    /// The accepted baseline reference, when one exists.
    #[serde(default)]
    pub baseline: Option<String>,
    /// The repository commit at pre-registration.
    pub code_commit: String,
    /// The deterministic seed.
    pub seed: u64,
    /// Whether analytics output is part of the run.
    #[serde(default)]
    pub analytics: Option<bool>,
}

/// One human decision request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecideImprovementRequest {
    /// The decision.
    pub decision: ImprovementDecision,
    /// The non-empty note.
    pub note: String,
}

/// One row of a published-number comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonRow {
    /// The published metric name.
    pub metric: String,
    /// The candidate value, when published.
    pub candidate: Option<f64>,
    /// The baseline value, when published.
    pub baseline: Option<f64>,
    /// Candidate minus baseline, when both are published.
    pub delta: Option<f64>,
}

/// One improvement-run comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImprovementComparison {
    /// The improvement run.
    pub improvement_id: String,
    /// The tier.
    pub tier: ImprovementTier,
    /// The candidate pipeline run.
    pub candidate_run_id: crate::run::RunId,
    /// The selected baseline id, when one exists.
    pub baseline_id: Option<String>,
    /// The candidate canonical digest.
    pub candidate_digest: Option<Digest>,
    /// The baseline canonical digest, when the baseline is a pipeline run.
    pub baseline_digest: Option<Digest>,
    /// The published-number rows.
    pub rows: Vec<ComparisonRow>,
    /// A note when the baseline metrics are unavailable.
    pub note: Option<String>,
}

/// One accepted baseline row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineRecord {
    /// The stable baseline id.
    pub baseline_id: String,
    /// The baseline tier.
    pub tier: ImprovementTier,
    /// The accepted pipeline run id or historical run range.
    pub run_id: Option<String>,
    /// The improvement id, when the run is a ledger record.
    pub improvement_id: Option<String>,
    /// The human who accepted it.
    pub accepted_by: String,
    /// The UTC acceptance date.
    pub accepted_on: String,
    /// The accepted scope.
    pub scope: String,
    /// The baseline it supersedes.
    pub supersedes: Option<String>,
    /// The row creation time.
    pub created_at: String,
}

/// One append-only invalidation tombstone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvalidationRecord {
    /// The stable tombstone id.
    pub invalidation_id: String,
    /// The UTC invalidation date.
    pub date: String,
    /// The commit that records it.
    pub commit: String,
    /// The removed run, baseline, or artifact.
    pub removed: String,
    /// The specific reason.
    pub reason: String,
    /// The replacement, when one exists.
    pub superseded_by: Option<String>,
    /// The row creation time.
    pub created_at: String,
}

fn required(field: &str, value: &str) -> Result<(), ImprovementManifestError> {
    if value.trim().is_empty() {
        return Err(ImprovementManifestError(format!(
            "{field} must be non-empty"
        )));
    }
    Ok(())
}

fn required_reference(
    field: &str,
    value: &str,
    allow_range: bool,
) -> Result<(), ImprovementManifestError> {
    required(field, value)?;
    let path_like = value.starts_with('/')
        || value.contains('/')
        || value.contains('\\')
        || value == ".."
        || (!allow_range && value.contains(".."));
    if path_like {
        return Err(ImprovementManifestError(format!(
            "{field} must be a reference, not a path"
        )));
    }
    Ok(())
}

fn validate_improvement_id(
    value: &str,
    tier: ImprovementTier,
) -> Result<(), ImprovementManifestError> {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 5
        || parts[0] != tier.id_prefix()
        || parts[1].len() != 4
        || parts[2].len() != 2
        || parts[3].len() != 2
        || parts[4].len() != 2
        || parts[4] == "00"
        || parts[1..]
            .iter()
            .any(|part| !part.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(ImprovementManifestError(format!(
            "improvement_id must be {}-YYYY-MM-DD-NN with a sequence starting at 01",
            tier.id_prefix()
        )));
    }
    let year = parts[1].parse::<i32>().unwrap_or(0);
    let month = parts[2].parse::<u32>().unwrap_or(0);
    let day = parts[3].parse::<u32>().unwrap_or(0);
    if !valid_date(year, month, day) {
        return Err(ImprovementManifestError(
            "improvement_id carries an invalid date".to_owned(),
        ));
    }
    Ok(())
}

fn is_rfc3339_utc(value: &str) -> bool {
    let Some(rest) = value
        .strip_suffix('Z')
        .or_else(|| value.strip_suffix("+00:00"))
    else {
        return false;
    };
    let Some((date, time)) = rest.split_once('T') else {
        return false;
    };
    let date_parts: Vec<&str> = date.split('-').collect();
    if date_parts.len() != 3
        || date_parts
            .iter()
            .any(|part| !part.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    let Ok(year) = date_parts[0].parse::<i32>() else {
        return false;
    };
    let Ok(month) = date_parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(day) = date_parts[2].parse::<u32>() else {
        return false;
    };
    if !valid_date(year, month, day) {
        return false;
    }
    let time = time.split('.').next().unwrap_or(time);
    let time_parts: Vec<&str> = time.split(':').collect();
    if time_parts.len() != 3
        || time_parts
            .iter()
            .any(|part| !part.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    let Ok(hour) = time_parts[0].parse::<u32>() else {
        return false;
    };
    let Ok(minute) = time_parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(second) = time_parts[2].parse::<u32>() else {
        return false;
    };
    hour < 24 && minute < 60 && second < 60
}

fn valid_date(year: i32, month: u32, day: u32) -> bool {
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1 && day <= days
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Value {
        serde_json::json!({
            "schema": IMPROVEMENT_RUN_SCHEMA,
            "improvement_id": "MODEL-2026-11-17-01",
            "tier": "model",
            "hypothesis": "a retest holds",
            "created_utc": "2026-11-17T00:00:00Z",
            "changed": "model",
            "frozen": ["strategy", "portfolio"],
            "partition": "model_dev",
            "data_version": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "config_id": "cfg_1",
            "criteria_id": null,
            "green_criteria": {"return": 0.1},
            "holdout_access": "none",
            "baseline": null,
            "code_commit": "abc123",
            "seed": 0,
            "command": "run submit",
            "analytics": true
        })
    }

    #[test]
    fn a_v3_manifest_round_trips() {
        let manifest = ImprovementManifest::parse(&manifest()).expect("parse");
        let text = serde_json::to_string(&manifest).expect("encode");
        assert_eq!(
            serde_json::from_str::<ImprovementManifest>(&text).expect("decode"),
            manifest
        );
    }

    #[test]
    fn a_v2_schema_is_refused() {
        let mut value = manifest();
        value["schema"] = serde_json::json!(IMPROVEMENT_RUN_SCHEMA_V2);
        assert!(ImprovementManifest::parse(&value).is_err());
        let historical = serde_json::json!({
            "schema": IMPROVEMENT_RUN_SCHEMA_V2,
            "improvement_id": "MODEL-2026-11-17-01",
            "run_config": "configs/dev.toml"
        });
        let error = ImprovementManifest::parse(&historical).expect_err("v2 is historical");
        assert!(error.to_string().contains("historical"), "{error}");
    }

    #[test]
    fn each_manifest_rule_is_enforced() {
        for mutate in [
            |value: &mut Value| value["improvement_id"] = serde_json::json!("MODEL-2026-11-17"),
            |value: &mut Value| value["improvement_id"] = serde_json::json!("MODEL-2026-02-30-01"),
            |value: &mut Value| value["improvement_id"] = serde_json::json!("MODEL-2026-11-17-00"),
            |value: &mut Value| value["changed"] = serde_json::json!("strategy"),
            |value: &mut Value| value["frozen"] = serde_json::json!(["strategy", "strategy"]),
            |value: &mut Value| value["frozen"] = serde_json::json!(["strategy", "model"]),
            |value: &mut Value| value["frozen"] = serde_json::json!(["strategy"]),
            |value: &mut Value| value["partition"] = serde_json::json!("portfolio_holdout"),
            |value: &mut Value| value["partition"] = serde_json::json!("strategy_dev"),
            |value: &mut Value| value["holdout_access"] = serde_json::json!("final"),
            |value: &mut Value| {
                value["created_utc"] = serde_json::json!("2026-11-17T00:00:00+01:00")
            },
            |value: &mut Value| value["created_utc"] = serde_json::json!("2026-11-18T00:00:00Z"),
            |value: &mut Value| value["green_criteria"] = serde_json::json!({}),
            |value: &mut Value| value["config_id"] = serde_json::json!("../cfg"),
            |value: &mut Value| value["config_id"] = serde_json::json!(".."),
            |value: &mut Value| value["criteria_id"] = serde_json::json!("../criteria"),
            |value: &mut Value| value["command"] = serde_json::json!("backtest run"),
            |value: &mut Value| value["seed"] = serde_json::json!(-1),
            |value: &mut Value| value["hypothesis"] = serde_json::json!(""),
            |value: &mut Value| value["unexpected"] = serde_json::json!(true),
            |value: &mut Value| value["baseline"] = serde_json::json!("../baseline"),
            |value: &mut Value| value["data_version"] = serde_json::json!("/data/set"),
        ] {
            let mut value = manifest();
            mutate(&mut value);
            assert!(ImprovementManifest::parse(&value).is_err(), "{value}");
        }
    }

    #[test]
    fn the_frozen_set_is_exact_for_the_tier() {
        let mut value = manifest();
        value["frozen"] = serde_json::json!(["strategy"]);
        assert!(ImprovementManifest::parse(&value).is_err());
        value["frozen"] = serde_json::json!(["strategy", "portfolio", "strategy"]);
        assert!(ImprovementManifest::parse(&value).is_err());
        value["frozen"] = serde_json::json!(["portfolio", "strategy"]);
        assert!(ImprovementManifest::parse(&value).is_ok());
    }

    #[test]
    fn created_utc_matches_the_improvement_date() {
        let mut value = manifest();
        value["created_utc"] = serde_json::json!("2026-11-18T00:00:00Z");
        let error = ImprovementManifest::parse(&value).expect_err("the date mismatch");
        assert!(error.to_string().contains("match"), "{error}");
    }

    #[test]
    fn a_strategy_manifest_needs_a_criteria_reference() {
        let mut value = manifest();
        value["improvement_id"] = serde_json::json!("STRATEGY-2026-11-17-01");
        value["tier"] = serde_json::json!("strategy");
        value["changed"] = serde_json::json!("strategy");
        value["frozen"] = serde_json::json!(["model", "portfolio"]);
        value["partition"] = serde_json::json!("strategy_dev");
        assert!(ImprovementManifest::parse(&value).is_err());
        value["criteria_id"] = serde_json::json!("criteria_1");
        assert!(ImprovementManifest::parse(&value).is_ok());
    }

    #[test]
    fn the_portfolio_holdout_is_final_only() {
        let mut value = manifest();
        value["improvement_id"] = serde_json::json!("PORTFOLIO-2026-11-17-01");
        value["tier"] = serde_json::json!("portfolio");
        value["changed"] = serde_json::json!("portfolio");
        value["frozen"] = serde_json::json!(["model", "strategy"]);
        value["partition"] = serde_json::json!("portfolio_holdout");
        value["criteria_id"] = serde_json::json!("criteria_1");
        value["holdout_access"] = serde_json::json!("final");
        assert!(ImprovementManifest::parse(&value).is_ok());
        value["holdout_access"] = serde_json::json!("none");
        assert!(ImprovementManifest::parse(&value).is_err());
    }

    #[test]
    fn partitions_match_the_declared_windows() {
        let text = include_str!("../../../data/partitions.json");
        let value: serde_json::Value = serde_json::from_str(text).expect("the partition file");
        let partitions = value["partitions"].as_object().expect("partitions");
        for (name, config) in partitions {
            let partition = ImprovementPartition::parse(name).expect("a declared partition");
            assert_eq!(
                partition.window().0,
                config["window"]["from"].as_str().expect("from"),
                "{name} from"
            );
            assert_eq!(
                partition.window().1,
                config["window"]["to"].as_str().expect("to"),
                "{name} to"
            );
        }
    }

    #[test]
    fn the_partition_unix_window_is_half_open() {
        let (start, end) = ImprovementPartition::ModelDev.unix_window();
        assert_eq!(start, 1_514_764_800, "2018-01-01 UTC");
        assert_eq!(end, 1_609_459_200, "2021-01-01 UTC, exclusive");
    }
}
