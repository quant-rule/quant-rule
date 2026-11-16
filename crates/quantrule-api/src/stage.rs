//! The diagnostic stage execution contract.
//!
//! A stage run executes one engine — model, strategy, or portfolio — outside the
//! coupled run. It is content-addressed, profile-scoped, and diagnostic: it has
//! no canonical run digest, it is never a baseline, and it never enters the
//! evidence path. [`StageRunDetail::evidence`] and [`StageRunSummary::evidence`]
//! are always `false`, so a client cannot mistake a stage projection for a run.
//!
//! The contract carries references only. A stage input is one dataset digest or
//! one frozen artifact digest; a stage artifact is a bundle identity and its
//! file references. No reading series, candle, or decision row crosses here.

use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::run::ArtifactRef;

/// Which engine a stage run executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    /// The model engine over one dataset and configuration.
    Model,
    /// The strategy engine over one frozen model artifact and configuration.
    Strategy,
    /// The execution and portfolio engines over one frozen decisions artifact
    /// and configuration.
    Portfolio,
}

impl StageKind {
    /// The wire and storage token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Strategy => "strategy",
            Self::Portfolio => "portfolio",
        }
    }

    /// Parse a stored token.
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

/// The lifecycle state of a stage run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    /// Accepted and durable, not started.
    Queued,
    /// A worker holds the message.
    Running,
    /// The artifact set is complete and its identity is recorded.
    Succeeded,
    /// The stage failed, or the retry budget was exhausted.
    Failed,
    /// The operator cancelled before completion.
    Cancelled,
}

impl StageState {
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

/// The content-derived identity of one stage run.
///
/// The digest is computed from the stage kind, its input, the configuration
/// hash, the model version, and the image digest. It lives in the `stage`
/// identity namespace and is never a canonical run digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StageRunId(pub Digest);

impl StageRunId {
    /// The digest behind the id.
    #[must_use]
    pub fn digest(&self) -> &Digest {
        &self.0
    }
}

impl std::fmt::Display for StageRunId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The one input a stage consumes.
///
/// The kind and its input are three disjoint cases: a model stage reads a
/// dataset, a strategy stage reads a frozen model artifact, and a portfolio
/// stage reads a frozen decisions artifact. No stage reads two inputs, so the
/// variant is the whole input contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StageInput {
    /// The dataset the model stage consumes.
    DataVersion {
        /// The content-addressed dataset identity.
        data_version: Digest,
    },
    /// The frozen model artifact the strategy stage consumes.
    ModelArtifact {
        /// The model bundle manifest digest.
        model_artifact_id: Digest,
    },
    /// The frozen decisions artifact the portfolio stage consumes.
    DecisionsArtifact {
        /// The decisions bundle digest.
        decisions_artifact_id: Digest,
    },
}

impl StageInput {
    /// The wire and storage token of the input kind.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::DataVersion { .. } => "data_version",
            Self::ModelArtifact { .. } => "model_artifact",
            Self::DecisionsArtifact { .. } => "decisions_artifact",
        }
    }

    /// The digest the input names.
    #[must_use]
    pub fn digest(&self) -> &Digest {
        match self {
            Self::DataVersion { data_version } => data_version,
            Self::ModelArtifact { model_artifact_id } => model_artifact_id,
            Self::DecisionsArtifact {
                decisions_artifact_id,
            } => decisions_artifact_id,
        }
    }

    /// The dataset identity, when this is a model-stage input.
    #[must_use]
    pub fn data_version(&self) -> Option<&Digest> {
        match self {
            Self::DataVersion { data_version } => Some(data_version),
            Self::ModelArtifact { .. } | Self::DecisionsArtifact { .. } => None,
        }
    }

    /// The model artifact identity, when this is a strategy-stage input.
    #[must_use]
    pub fn model_artifact_id(&self) -> Option<&Digest> {
        match self {
            Self::ModelArtifact { model_artifact_id } => Some(model_artifact_id),
            Self::DataVersion { .. } | Self::DecisionsArtifact { .. } => None,
        }
    }

    /// The decisions artifact identity, when this is a portfolio-stage input.
    #[must_use]
    pub fn decisions_artifact_id(&self) -> Option<&Digest> {
        match self {
            Self::DecisionsArtifact {
                decisions_artifact_id,
            } => Some(decisions_artifact_id),
            Self::DataVersion { .. } | Self::ModelArtifact { .. } => None,
        }
    }

    /// Whether this input belongs to a stage of `kind`.
    #[must_use]
    pub fn matches(&self, kind: StageKind) -> bool {
        matches!(
            (kind, self),
            (StageKind::Model, Self::DataVersion { .. })
                | (StageKind::Strategy, Self::ModelArtifact { .. })
                | (StageKind::Portfolio, Self::DecisionsArtifact { .. })
        )
    }
}

/// A request to run one stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRunRequest {
    /// Which engine executes.
    pub kind: StageKind,
    /// The configuration to apply.
    pub config_id: String,
    /// The single input the stage consumes.
    pub input: StageInput,
}

impl StageRunRequest {
    /// Whether the input belongs to the requested stage kind.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        self.input.matches(self.kind)
    }
}

/// What produced a stage run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageProvenance {
    /// The dataset the stage ultimately reads.
    ///
    /// A model stage names its own input. A strategy or portfolio stage names
    /// the dataset the frozen upstream artifact records.
    pub data_version: Digest,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model identity from the run configuration, such as `M1.3`.
    pub model_version: String,
    /// The image digest that executed the stage.
    pub image_digest: Digest,
}

/// Why an artifact set is not a valid stage bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageArtifactError {
    /// The bundle carries no files.
    Empty,
    /// A file has an empty name.
    EmptyName,
    /// Two files share a name.
    DuplicateName(String),
}

impl std::fmt::Display for StageArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a stage artifact has no files"),
            Self::EmptyName => formatter.write_str("a stage artifact file name is empty"),
            Self::DuplicateName(name) => {
                write!(formatter, "the stage artifact names {name:?} twice")
            }
        }
    }
}

impl std::error::Error for StageArtifactError {}

/// A content-addressed stage artifact bundle.
///
/// The identity is computed over the sorted `(name, sha256)` entries, so the
/// same files always name one bundle. It is a stage bundle digest, not the
/// canonical digest of a run; a stage artifact is never evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageArtifact {
    /// The digest of the complete file set.
    pub artifact_id: Digest,
    /// The files, sorted by name.
    pub files: Vec<ArtifactRef>,
}

impl StageArtifact {
    /// Build an artifact bundle, sorting its files and computing the id.
    ///
    /// # Errors
    ///
    /// Returns [`StageArtifactError`] when the bundle is empty, a name is
    /// empty, or two files share a name.
    pub fn from_files(mut files: Vec<ArtifactRef>) -> Result<Self, StageArtifactError> {
        if files.is_empty() {
            return Err(StageArtifactError::Empty);
        }
        if files.iter().any(|file| file.name.is_empty()) {
            return Err(StageArtifactError::EmptyName);
        }
        files.sort_by(|left, right| left.name.cmp(&right.name));
        if let Some(pair) = files.windows(2).find(|pair| pair[0].name == pair[1].name) {
            return Err(StageArtifactError::DuplicateName(pair[0].name.clone()));
        }
        let artifact_id = Self::digest(&files);
        Ok(Self { artifact_id, files })
    }

    /// The bundle digest over a file set.
    ///
    /// Every file is included; a stage bundle has no excluded provenance file,
    /// because it never claims to be the canonical digest of a run. The order
    /// of `files` cannot change the result.
    #[must_use]
    pub fn digest(files: &[ArtifactRef]) -> Digest {
        let entries: Vec<(String, Digest)> = files
            .iter()
            .map(|file| (file.name.clone(), file.sha256.clone()))
            .collect();
        Digest::canonical(&entries)
    }

    /// Whether the files are sorted, unique, non-empty, and match the id.
    #[must_use]
    pub fn is_consistent(&self) -> bool {
        !self.files.is_empty()
            && self.files.iter().all(|file| !file.name.is_empty())
            && self
                .files
                .windows(2)
                .all(|pair| pair[0].name < pair[1].name)
            && self.artifact_id == Self::digest(&self.files)
    }
}

/// A stage run in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRunSummary {
    /// The stage run id.
    pub stage_run_id: StageRunId,
    /// Which engine executed.
    pub kind: StageKind,
    /// Its state.
    pub state: StageState,
    /// The configuration applied.
    pub config_id: String,
    /// Server-assigned submission time, RFC 3339 UTC.
    pub submitted_at: String,
    /// The artifact identity once the stage succeeded.
    pub artifact_id: Option<Digest>,
    /// Always `false`; a stage run is diagnostic and never evidence.
    pub evidence: bool,
}

/// A stage run in detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRunDetail {
    /// The stage run id.
    pub stage_run_id: StageRunId,
    /// Which engine executed.
    pub kind: StageKind,
    /// Its state.
    pub state: StageState,
    /// The single principal that submitted it.
    pub owner: String,
    /// The configuration applied.
    pub config_id: String,
    /// The single input the stage consumed.
    pub input: StageInput,
    /// What produced the stage run.
    pub provenance: StageProvenance,
    /// The artifact bundle, once the stage succeeded.
    pub artifact: Option<StageArtifact>,
    /// Server-assigned submission time, RFC 3339 UTC.
    pub submitted_at: String,
    /// The failure, when the state is `failed`.
    pub error: Option<String>,
    /// The number of deliveries claimed.
    pub attempts: u32,
    /// Always `false`; a stage run is diagnostic and never evidence.
    pub evidence: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    fn file(name: &str, seed: &[u8]) -> ArtifactRef {
        ArtifactRef {
            name: name.to_owned(),
            bytes: seed.len() as u64,
            sha256: digest(seed),
        }
    }

    fn provenance() -> StageProvenance {
        StageProvenance {
            data_version: digest(b"data"),
            config_hash: digest(b"config"),
            model_version: "M1.3".to_owned(),
            image_digest: digest(b"image"),
        }
    }

    fn artifact() -> StageArtifact {
        StageArtifact::from_files(vec![file("model.json", b"{}")]).expect("the artifact")
    }

    fn detail() -> StageRunDetail {
        StageRunDetail {
            stage_run_id: StageRunId(digest(b"stage")),
            kind: StageKind::Model,
            state: StageState::Succeeded,
            owner: "operator".to_owned(),
            config_id: "cfg-1".to_owned(),
            input: StageInput::DataVersion {
                data_version: digest(b"data"),
            },
            provenance: provenance(),
            artifact: Some(artifact()),
            submitted_at: "2026-11-17T00:00:00Z".to_owned(),
            error: None,
            attempts: 1,
            evidence: false,
        }
    }

    #[test]
    fn only_three_states_are_terminal() {
        for state in [
            StageState::Succeeded,
            StageState::Failed,
            StageState::Cancelled,
        ] {
            assert!(state.is_terminal(), "{state:?}");
        }
        assert!(!StageState::Queued.is_terminal());
        assert!(!StageState::Running.is_terminal());
    }

    #[test]
    fn only_active_states_are_cancellable() {
        assert!(StageState::Queued.is_cancellable());
        assert!(StageState::Running.is_cancellable());
        for state in [
            StageState::Succeeded,
            StageState::Failed,
            StageState::Cancelled,
        ] {
            assert!(!state.is_cancellable(), "{state:?}");
        }
    }

    #[test]
    fn the_tokens_round_trip() {
        for kind in [StageKind::Model, StageKind::Strategy, StageKind::Portfolio] {
            assert_eq!(StageKind::parse(kind.as_str()), Some(kind));
        }
        for state in [
            StageState::Queued,
            StageState::Running,
            StageState::Succeeded,
            StageState::Failed,
            StageState::Cancelled,
        ] {
            assert_eq!(StageState::parse(state.as_str()), Some(state));
        }
        assert_eq!(StageKind::parse("backtest"), None);
        assert_eq!(StageState::parse("done"), None);
    }

    #[test]
    fn states_and_kinds_serialize_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&StageKind::Portfolio).expect("encode"),
            "\"portfolio\""
        );
        assert_eq!(
            serde_json::to_string(&StageState::Succeeded).expect("encode"),
            "\"succeeded\""
        );
    }

    #[test]
    fn a_stage_input_round_trips_and_exposes_its_digest() {
        let inputs = [
            StageInput::DataVersion {
                data_version: digest(b"data"),
            },
            StageInput::ModelArtifact {
                model_artifact_id: digest(b"model"),
            },
            StageInput::DecisionsArtifact {
                decisions_artifact_id: digest(b"decisions"),
            },
        ];
        for input in inputs {
            let text = serde_json::to_string(&input).expect("encode");
            assert_eq!(
                serde_json::from_str::<StageInput>(&text).expect("decode"),
                input
            );
        }
        assert_eq!(
            StageInput::DataVersion {
                data_version: digest(b"d")
            }
            .label(),
            "data_version"
        );
        assert_eq!(
            StageInput::ModelArtifact {
                model_artifact_id: digest(b"m")
            }
            .label(),
            "model_artifact"
        );
        assert_eq!(
            StageInput::DecisionsArtifact {
                decisions_artifact_id: digest(b"d")
            }
            .label(),
            "decisions_artifact"
        );
    }

    #[test]
    fn an_input_matches_only_its_stage_kind() {
        let data = StageInput::DataVersion {
            data_version: digest(b"data"),
        };
        let model = StageInput::ModelArtifact {
            model_artifact_id: digest(b"model"),
        };
        let decisions = StageInput::DecisionsArtifact {
            decisions_artifact_id: digest(b"decisions"),
        };
        assert!(data.matches(StageKind::Model));
        assert!(model.matches(StageKind::Strategy));
        assert!(decisions.matches(StageKind::Portfolio));
        assert!(!data.matches(StageKind::Strategy));
        assert!(!model.matches(StageKind::Portfolio));
        assert!(!decisions.matches(StageKind::Model));
        assert_eq!(data.data_version(), Some(&digest(b"data")));
        assert_eq!(data.model_artifact_id(), None);
        assert_eq!(model.model_artifact_id(), Some(&digest(b"model")));
        assert_eq!(model.decisions_artifact_id(), None);
        assert_eq!(
            decisions.decisions_artifact_id(),
            Some(&digest(b"decisions"))
        );
        assert_eq!(decisions.data_version(), None);
    }

    #[test]
    fn a_request_round_trips_and_checks_its_input() {
        let request = StageRunRequest {
            kind: StageKind::Model,
            config_id: "cfg-1".to_owned(),
            input: StageInput::DataVersion {
                data_version: digest(b"data"),
            },
        };
        let text = serde_json::to_string(&request).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageRunRequest>(&text).expect("decode"),
            request
        );
        assert!(request.is_consistent());
        let inconsistent = StageRunRequest {
            kind: StageKind::Strategy,
            ..request
        };
        assert!(!inconsistent.is_consistent());
    }

    #[test]
    fn a_request_rejects_an_unknown_field() {
        let text = format!(
            "{{\"kind\":\"model\",\"config_id\":\"cfg\",\"input\":\
             {{\"kind\":\"data_version\",\"data_version\":\"sha256:{d}\"}},\
             \"path\":\"/tmp\"}}",
            d = "a".repeat(64)
        );
        let error = serde_json::from_str::<StageRunRequest>(&text).expect_err("must reject");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn provenance_rejects_an_unknown_field() {
        let text = format!(
            "{{\"data_version\":\"sha256:{d}\",\"config_hash\":\"sha256:{d}\",\
             \"model_version\":\"M1.3\",\"image_digest\":\"sha256:{d}\",\"host\":\"x\"}}",
            d = "a".repeat(64)
        );
        assert!(serde_json::from_str::<StageProvenance>(&text).is_err());
    }

    #[test]
    fn an_artifact_sorts_its_files_and_computes_one_id() {
        let first = StageArtifact::from_files(vec![
            file("results.json", b"results"),
            file("fills.csv", b"fills"),
        ])
        .expect("the artifact");
        let second = StageArtifact::from_files(vec![
            file("fills.csv", b"fills"),
            file("results.json", b"results"),
        ])
        .expect("the artifact");
        assert_eq!(first, second);
        assert_eq!(
            first
                .files
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["fills.csv", "results.json"]
        );
        assert_eq!(first.artifact_id, second.artifact_id);
        assert!(first.is_consistent());
    }

    #[test]
    fn an_artifact_rejects_an_empty_set_a_blank_name_and_a_duplicate() {
        assert_eq!(
            StageArtifact::from_files(Vec::new()),
            Err(StageArtifactError::Empty)
        );
        assert_eq!(
            StageArtifact::from_files(vec![file("", b"a")]),
            Err(StageArtifactError::EmptyName)
        );
        assert_eq!(
            StageArtifact::from_files(vec![file("a.csv", b"a"), file("a.csv", b"b")]),
            Err(StageArtifactError::DuplicateName("a.csv".to_owned()))
        );
    }

    #[test]
    fn a_changed_file_changes_the_artifact_id() {
        let base = StageArtifact::from_files(vec![file("results.json", b"a")]).expect("artifact");
        let changed =
            StageArtifact::from_files(vec![file("results.json", b"b")]).expect("artifact");
        assert_ne!(base.artifact_id, changed.artifact_id);
        let renamed = StageArtifact::from_files(vec![file("other.json", b"a")]).expect("artifact");
        assert_ne!(base.artifact_id, renamed.artifact_id);
    }

    #[test]
    fn a_forged_artifact_id_is_not_consistent() {
        let mut artifact = artifact();
        artifact.artifact_id = digest(b"forged");
        assert!(!artifact.is_consistent());
        let mut unsorted =
            StageArtifact::from_files(vec![file("a.csv", b"a"), file("b.csv", b"b")])
                .expect("artifact");
        unsorted.files.swap(0, 1);
        assert!(!unsorted.is_consistent());
    }

    #[test]
    fn a_summary_and_a_detail_round_trip_and_are_never_evidence() {
        let detail = detail();
        let text = serde_json::to_string(&detail).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageRunDetail>(&text).expect("decode"),
            detail
        );
        assert!(!detail.evidence);
        let mut with_unknown = serde_json::to_value(&detail).expect("value");
        with_unknown["path"] = serde_json::json!("/tmp");
        assert!(
            serde_json::from_value::<StageRunDetail>(with_unknown).is_err(),
            "a detail accepted an unknown field"
        );
        let summary = StageRunSummary {
            stage_run_id: detail.stage_run_id.clone(),
            kind: detail.kind,
            state: detail.state,
            config_id: detail.config_id.clone(),
            submitted_at: detail.submitted_at.clone(),
            artifact_id: Some(artifact().artifact_id),
            evidence: false,
        };
        let text = serde_json::to_string(&summary).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageRunSummary>(&text).expect("decode"),
            summary
        );
        assert!(!summary.evidence);
    }

    #[test]
    fn a_detail_is_not_a_run_record() {
        let text = serde_json::to_string(&detail()).expect("encode");
        assert!(!text.contains("\"run_id\""));
        assert!(!text.contains("\"digest\""));
        assert!(!text.contains("canonical"));
    }

    #[test]
    fn a_stage_run_id_round_trips() {
        let id = StageRunId(digest(b"stage"));
        let text = serde_json::to_string(&id).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageRunId>(&text).expect("decode"),
            id
        );
        assert_eq!(id.to_string(), digest(b"stage").to_string());
        assert_eq!(id.digest(), &digest(b"stage"));
    }
}
