//! The run lifecycle contract.
//!
//! A run is the full pipeline. Its identity is content-derived, so resubmitting
//! the same inputs is a lookup rather than new work.

use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// The content-derived identity of a run.
///
/// Computed as `sha256(data_version ‖ config_hash ‖ model_version ‖
/// image_digest)`. The value is supplied by the server; a client never invents
/// one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RunId(pub Digest);

impl RunId {
    /// The digest behind the id.
    #[must_use]
    pub fn digest(&self) -> &Digest {
        &self.0
    }
}

impl StageName {
    /// The wire and storage token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Model => "model",
            Self::Backtest => "backtest",
            Self::Evidence => "evidence",
        }
    }

    /// Parse a stored token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "data" => Some(Self::Data),
            "model" => Some(Self::Model),
            "backtest" => Some(Self::Backtest),
            "evidence" => Some(Self::Evidence),
            _ => None,
        }
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The lifecycle state of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Accepted and durable, not started.
    Queued,
    /// A worker holds the message.
    Running,
    /// The artifact set is complete and the digest is recorded.
    Succeeded,
    /// A stage failed, or the retry budget was exhausted.
    Failed,
    /// The operator cancelled before completion.
    Cancelled,
}

impl RunState {
    /// The wire and storage token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Parse a stored token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Whether the state is terminal.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    /// Whether a cancel request is meaningful.
    #[must_use]
    pub fn is_cancellable(self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }
}

/// One step of the data plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageName {
    /// Ingestion and the timeframe ladder.
    Data,
    /// Readings, states, events, progression, report.
    Model,
    /// Strategy, execution, portfolio.
    Backtest,
    /// Provenance and the canonical digest.
    Evidence,
}

/// The state of one stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// Not started.
    Pending,
    /// In progress.
    Running,
    /// Finished successfully.
    Done,
    /// Failed.
    Failed,
    /// Not run because a later condition was already decided.
    Skipped,
}

impl StageStatus {
    /// The wire and storage token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    /// Parse a stored token.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "done" => Some(Self::Done),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

/// A stage and its progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageProgress {
    /// Which stage.
    pub name: StageName,
    /// Its state.
    pub state: StageStatus,
    /// Elapsed milliseconds, control-plane metadata only.
    ///
    /// Never written into an artifact: a duration is not part of the result.
    pub elapsed_ms: Option<u64>,
}

/// What produced a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// The dataset consumed.
    pub data_version: Digest,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model registry version.
    pub model_version: String,
    /// The image digest that executed the run.
    pub image_digest: Digest,
}

/// One artifact of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// File name, relative to the bundle root.
    pub name: String,
    /// Size in bytes.
    pub bytes: u64,
    /// Content hash.
    pub sha256: Digest,
}

/// A run in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    /// The run id.
    pub run_id: RunId,
    /// Its state.
    pub status: RunState,
    /// The configuration it used.
    pub config_id: String,
    /// The dataset it consumed.
    pub data_version: Digest,
    /// Server-assigned submission time, RFC 3339 UTC.
    pub submitted_at: String,
    /// Elapsed milliseconds once finished.
    pub elapsed_ms: Option<u64>,
}

/// A run in detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunDetail {
    /// The run id.
    pub run_id: RunId,
    /// Its state.
    pub status: RunState,
    /// The single principal that submitted it.
    pub owner: String,
    /// Per-stage progress.
    pub stages: Vec<StageProgress>,
    /// What produced the run.
    pub provenance: Provenance,
    /// The artifact set, once complete.
    pub artifacts: Vec<ArtifactRef>,
    /// The canonical digest over the data artifacts.
    pub digest: Option<Digest>,
    /// The failure, when the state is `failed`.
    pub error: Option<String>,
}

/// A submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitRunRequest {
    /// The dataset to consume.
    pub data_version: Digest,
    /// The configuration to apply.
    pub config_id: String,
}

/// The response to a submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmitRunResponse {
    /// The run id.
    pub run_id: RunId,
    /// `queued` for new work; the existing state for a lookup.
    pub status: RunState,
    /// Whether this submission created the run.
    pub created: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> Digest {
        Digest::parse(&"a".repeat(64)).expect("fixture")
    }

    #[test]
    fn only_three_states_are_terminal() {
        let terminal = [RunState::Succeeded, RunState::Failed, RunState::Cancelled];
        for state in terminal {
            assert!(state.is_terminal(), "{state:?}");
        }
        assert!(!RunState::Queued.is_terminal());
        assert!(!RunState::Running.is_terminal());
    }

    #[test]
    fn only_active_states_are_cancellable() {
        assert!(RunState::Queued.is_cancellable());
        assert!(RunState::Running.is_cancellable());
        for state in [RunState::Succeeded, RunState::Failed, RunState::Cancelled] {
            assert!(!state.is_cancellable(), "{state:?}");
        }
    }

    #[test]
    fn states_serialize_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&RunState::Succeeded).expect("encode"),
            "\"succeeded\""
        );
        assert_eq!(
            serde_json::to_string(&StageName::Backtest).expect("encode"),
            "\"backtest\""
        );
    }

    #[test]
    fn a_submission_rejects_an_unknown_field() {
        let text = format!(
            "{{\"data_version\":\"sha256:{}\",\"config_id\":\"cfg\",\"path\":\"/tmp\"}}",
            "a".repeat(64)
        );
        let error = serde_json::from_str::<SubmitRunRequest>(&text).expect_err("must reject");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn provenance_rejects_an_unknown_field() {
        let text = format!(
            "{{\"data_version\":\"sha256:{d}\",\"config_hash\":\"sha256:{d}\",\
             \"model_version\":\"M1.3\",\"image_digest\":\"sha256:{d}\",\"host\":\"x\"}}",
            d = "a".repeat(64)
        );
        assert!(serde_json::from_str::<Provenance>(&text).is_err());
    }

    #[test]
    fn a_run_detail_round_trips() {
        let detail = RunDetail {
            run_id: RunId(digest()),
            status: RunState::Succeeded,
            owner: "operator".to_owned(),
            stages: vec![StageProgress {
                name: StageName::Model,
                state: StageStatus::Done,
                elapsed_ms: Some(1_200),
            }],
            provenance: Provenance {
                data_version: digest(),
                config_hash: digest(),
                model_version: "M1.3".to_owned(),
                image_digest: digest(),
            },
            artifacts: vec![ArtifactRef {
                name: "results.json".to_owned(),
                bytes: 42,
                sha256: digest(),
            }],
            digest: Some(digest()),
            error: None,
        };
        let text = serde_json::to_string(&detail).expect("encode");
        let back: RunDetail = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, detail);
    }
}
