//! The `quantrule.internal/v1` run lifecycle.
//!
//! A run is content-addressed and shared. The record is the same shape in every
//! service: the gateway owns submission, evidence owns the row, and the worker
//! claims and completes it. Defining it once keeps a field from appearing in one
//! service's JSON and not another's.

use quantrule_api::{
    ArtifactRef, Digest, Provenance, RunDetail, RunId, RunState, RunSummary, StageName,
    StageProgress, StageStatus,
};
use serde::{Deserialize, Serialize};

/// The number of deliveries a run may receive before it is dead-lettered.
pub const MAX_ATTEMPTS: u32 = 3;

/// Derive a run id from the four inputs the contract names.
///
/// The fields are newline-delimited and tagged with the identity version, so a
/// field boundary cannot be forged by concatenation. Evidence computes the id;
/// a worker and a test derive the same value from the same contract.
#[must_use]
pub fn derive_run_id(
    data_version: &Digest,
    config_hash: &Digest,
    model_version: &str,
    image_digest: &Digest,
) -> RunId {
    let mut bytes = b"quantrule.run/v1\n".to_vec();
    for part in [
        data_version.to_string(),
        config_hash.to_string(),
        model_version.to_owned(),
        image_digest.to_string(),
    ] {
        bytes.extend_from_slice(part.as_bytes());
        bytes.push(b'\n');
    }
    RunId(Digest::sha256(&bytes))
}

/// A new submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewRun {
    /// The content-derived id.
    pub run_id: RunId,
    /// The configuration it applies.
    pub config_id: String,
    /// The dataset it consumes.
    pub data_version: Digest,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model identity.
    pub model_version: String,
    /// The image that will execute it.
    pub image_digest: Digest,
    /// The first submitter.
    pub owner: String,
    /// The submission time, RFC 3339 UTC.
    pub submitted_at: String,
}

/// One stored run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecord {
    /// The run id.
    pub run_id: RunId,
    /// Its state.
    pub status: RunState,
    /// The first submitter.
    pub owner: String,
    /// The configuration id.
    pub config_id: String,
    /// The dataset consumed.
    pub data_version: Digest,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model identity.
    pub model_version: String,
    /// The image digest.
    pub image_digest: Digest,
    /// The submission time.
    pub submitted_at: String,
    /// Per-stage progress.
    pub stages: Vec<StageProgress>,
    /// The canonical digest once complete.
    pub digest: Option<Digest>,
    /// The artifact set, ordered by name.
    pub artifacts: Vec<ArtifactRef>,
    /// The failure, when failed.
    pub error: Option<String>,
    /// The number of deliveries claimed.
    pub attempts: u32,
}

impl RunRecord {
    /// A queued record with every stage pending.
    #[must_use]
    pub fn queued(run: &NewRun) -> Self {
        Self {
            run_id: run.run_id.clone(),
            status: RunState::Queued,
            owner: run.owner.clone(),
            config_id: run.config_id.clone(),
            data_version: run.data_version.clone(),
            config_hash: run.config_hash.clone(),
            model_version: run.model_version.clone(),
            image_digest: run.image_digest.clone(),
            submitted_at: run.submitted_at.clone(),
            stages: [
                StageName::Data,
                StageName::Model,
                StageName::Backtest,
                StageName::Evidence,
            ]
            .into_iter()
            .map(|name| StageProgress {
                name,
                state: StageStatus::Pending,
                elapsed_ms: None,
            })
            .collect(),
            digest: None,
            artifacts: Vec::new(),
            error: None,
            attempts: 0,
        }
    }

    /// The list projection.
    #[must_use]
    pub fn summary(&self) -> RunSummary {
        RunSummary {
            run_id: self.run_id.clone(),
            status: self.status,
            config_id: self.config_id.clone(),
            data_version: self.data_version.clone(),
            submitted_at: self.submitted_at.clone(),
            elapsed_ms: None,
        }
    }

    /// The detail projection.
    #[must_use]
    pub fn detail(&self) -> RunDetail {
        RunDetail {
            run_id: self.run_id.clone(),
            status: self.status,
            owner: self.owner.clone(),
            stages: self.stages.clone(),
            provenance: Provenance {
                data_version: self.data_version.clone(),
                config_hash: self.config_hash.clone(),
                model_version: self.model_version.clone(),
                image_digest: self.image_digest.clone(),
            },
            artifacts: self.artifacts.clone(),
            digest: self.digest.clone(),
            error: self.error.clone(),
        }
    }
}

/// A worker's request to take a delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunClaimRequest {
    /// The run the message names.
    pub run_id: RunId,
    /// Whether the broker says this delivery was redelivered.
    pub redelivered: bool,
}

/// What a claim did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunClaimOutcome {
    /// The worker may execute the run.
    Claimed(RunRecord),
    /// Another worker still holds the delivery.
    Running(RunRecord),
    /// The run already reached a terminal state.
    Terminal(RunRecord),
    /// The retry budget is spent.
    Exhausted(RunRecord),
    /// The run does not exist.
    Missing,
}

/// A worker's completion payload, before evidence finalizes the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunComputed {
    /// The run that produced the artifacts.
    pub run_id: RunId,
    /// The canonical digest over the data artifacts.
    pub digest: Digest,
    /// The artifact set, ordered by name.
    pub artifacts: Vec<ArtifactRef>,
    /// Per-stage progress.
    pub stages: Vec<StageProgress>,
}

/// A worker's failure payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunFailed {
    /// The run that failed.
    pub run_id: RunId,
    /// The failure, as recorded on the run row.
    pub error: String,
    /// Per-stage progress at the failure.
    pub stages: Vec<StageProgress>,
}

/// What a cancellation did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunCancelOutcome {
    /// The run moved to `cancelled`.
    Cancelled(RunRecord),
    /// The run was already terminal.
    Conflict(RunRecord),
    /// This profile does not see the run.
    Missing,
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::StageName;

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    fn new_run() -> NewRun {
        NewRun {
            run_id: RunId(digest(b"run")),
            config_id: "cfg-1".to_owned(),
            data_version: digest(b"data"),
            config_hash: digest(b"config"),
            model_version: "M1.3".to_owned(),
            image_digest: digest(b"image"),
            owner: "operator".to_owned(),
            submitted_at: "2026-11-14T00:00:00Z".to_owned(),
        }
    }

    fn record() -> RunRecord {
        RunRecord::queued(&new_run())
    }

    #[test]
    fn a_new_run_round_trips() {
        let run = new_run();
        let text = serde_json::to_string(&run).expect("encode");
        assert_eq!(serde_json::from_str::<NewRun>(&text).expect("decode"), run);
    }

    #[test]
    fn a_queued_record_has_every_stage_pending() {
        let record = record();
        assert_eq!(record.status, RunState::Queued);
        assert_eq!(
            record
                .stages
                .iter()
                .map(|stage| stage.name)
                .collect::<Vec<_>>(),
            vec![
                StageName::Data,
                StageName::Model,
                StageName::Backtest,
                StageName::Evidence
            ]
        );
        assert!(
            record
                .stages
                .iter()
                .all(|stage| stage.state == StageStatus::Pending)
        );
        assert_eq!(record.attempts, 0);
    }

    #[test]
    fn a_record_round_trips_and_projects() {
        let record = record();
        let text = serde_json::to_string(&record).expect("encode");
        let back: RunRecord = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, record);
        assert_eq!(record.summary().run_id, record.run_id);
        assert_eq!(record.detail().provenance.model_version, "M1.3");
    }

    #[test]
    fn a_claim_response_round_trips() {
        let outcome = RunClaimOutcome::Claimed(record());
        let text = serde_json::to_string(&outcome).expect("encode");
        assert!(text.contains("\"claimed\""));
        assert_eq!(
            serde_json::from_str::<RunClaimOutcome>(&text).expect("decode"),
            outcome
        );
        assert_eq!(
            serde_json::to_string(&RunCancelOutcome::Missing).expect("encode"),
            "\"missing\""
        );
    }

    #[test]
    fn completion_and_failure_payloads_round_trip() {
        let computed = RunComputed {
            run_id: RunId(digest(b"run")),
            digest: digest(b"digest"),
            artifacts: vec![ArtifactRef {
                name: "results.json".to_owned(),
                bytes: 2,
                sha256: digest(b"{}"),
            }],
            stages: record().stages,
        };
        let text = serde_json::to_string(&computed).expect("encode");
        assert_eq!(
            serde_json::from_str::<RunComputed>(&text).expect("decode"),
            computed
        );

        let failed = RunFailed {
            run_id: RunId(digest(b"run")),
            error: "the dataset is unknown".to_owned(),
            stages: record().stages,
        };
        let text = serde_json::to_string(&failed).expect("encode");
        assert_eq!(
            serde_json::from_str::<RunFailed>(&text).expect("decode"),
            failed
        );
    }

    #[test]
    fn a_claim_request_rejects_an_unknown_field() {
        let text = format!(
            "{{\"run_id\":\"sha256:{}\",\"redelivered\":false,\"profile\":\"x\"}}",
            "a".repeat(64)
        );
        assert!(serde_json::from_str::<RunClaimRequest>(&text).is_err());
    }

    #[test]
    fn a_profile_is_not_part_of_the_internal_run_record() {
        // The profile is carried on the request, not the shared row: two
        // profiles point at one run without duplicating it.
        let text = serde_json::to_string(&record()).expect("encode");
        assert!(!text.contains("profile_id"));
    }

    #[test]
    fn the_same_inputs_derive_the_same_run_id() {
        let data = digest(b"data");
        let config = digest(b"config");
        let image = digest(b"image");
        assert_eq!(
            derive_run_id(&data, &config, "M1.0", &image),
            derive_run_id(&data, &config, "M1.0", &image)
        );
    }

    #[test]
    fn every_run_identity_input_changes_the_id() {
        let data = digest(b"data");
        let config = digest(b"config");
        let image = digest(b"image");
        let base = derive_run_id(&data, &config, "M1.0", &image);
        assert_ne!(
            base,
            derive_run_id(&digest(b"other"), &config, "M1.0", &image)
        );
        assert_ne!(
            base,
            derive_run_id(&data, &digest(b"other"), "M1.0", &image)
        );
        assert_ne!(base, derive_run_id(&data, &config, "M2.0", &image));
        assert_ne!(
            base,
            derive_run_id(&data, &config, "M1.0", &digest(b"other"))
        );
    }

    #[test]
    fn a_run_identity_field_boundary_is_not_ambiguous() {
        let data = digest(b"data");
        let config = digest(b"config");
        let image = digest(b"image");
        assert_ne!(
            derive_run_id(&data, &config, "ab", &image),
            derive_run_id(&data, &config, "a\nb", &image)
        );
    }

    #[test]
    fn the_retry_budget_is_three_deliveries() {
        assert_eq!(MAX_ATTEMPTS, 3);
    }
}
