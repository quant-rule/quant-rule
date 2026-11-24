//! The `quantrule.internal/v1` stage contract.
//!
//! A stage run is a diagnostic unit: one engine, one input reference, one
//! content-addressed artifact bundle. The manifest is the atomic publish point
//! for that bundle and carries `evidence: false`; the record is the durable
//! lifecycle row. Neither type is a run: a stage has no canonical digest and no
//! `run_id`, and the evidence service keeps its rows in a separate schema.
//!
//! Defining the record and the manifest once keeps the worker that writes them
//! and the evidence service that stores them from drifting.

use quantrule_api::{
    ArtifactRef, Digest, StageArtifact, StageInput, StageKind, StageProvenance, StageRunDetail,
    StageRunId, StageRunSummary, StageState,
};
use serde::{Deserialize, Serialize};

/// How long a claimed stage's lease stays valid without a heartbeat.
pub const STAGE_LEASE_TTL_SECONDS: u64 = 60;

/// How often a stage worker renews its lease while it executes.
pub const STAGE_LEASE_RENEW_SECONDS: u64 = 20;

/// The stage manifest schema version.
pub const STAGE_MANIFEST_SCHEMA: &str = "quantrule.stage-manifest/v1";

/// Derive a stage run id from the stage identity inputs.
///
/// The fields are newline-delimited and tagged with the identity version, so a
/// field boundary cannot be forged by concatenation. The id lives in the stage
/// namespace and is never a canonical run digest. Evidence derives the same
/// value; a worker and a test derive it from this one contract.
#[must_use]
pub fn derive_stage_run_id(
    kind: StageKind,
    input: &StageInput,
    config_hash: &Digest,
    model_version: &str,
    image_digest: &Digest,
) -> StageRunId {
    let mut bytes = b"quantrule.stage-run/v1\n".to_vec();
    for part in [
        kind.as_str().to_owned(),
        input.label().to_owned(),
        input.digest().to_string(),
        config_hash.to_string(),
        model_version.to_owned(),
        image_digest.to_string(),
    ] {
        bytes.extend_from_slice(part.as_bytes());
        bytes.push(b'\n');
    }
    StageRunId(Digest::sha256(&bytes))
}

/// A new stage submission.
///
/// [`NewStageRun::new`] derives the id from the same identity the store and the
/// manifest verify. A hand-built value can be checked with
/// [`NewStageRun::has_derived_id`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewStageRun {
    /// The content-derived stage id.
    pub stage_run_id: StageRunId,
    /// Which engine executes.
    pub kind: StageKind,
    /// The configuration to apply.
    pub config_id: String,
    /// The single input the stage consumes.
    pub input: StageInput,
    /// What produced the stage run.
    pub provenance: StageProvenance,
    /// The first submitter.
    pub owner: String,
    /// The submission time, RFC 3339 UTC.
    pub submitted_at: String,
}

impl NewStageRun {
    /// Build a submission and derive its content-addressed id.
    #[must_use]
    pub fn new(
        kind: StageKind,
        config_id: String,
        input: StageInput,
        provenance: StageProvenance,
        owner: String,
        submitted_at: String,
    ) -> Self {
        let stage_run_id = derive_stage_run_id(
            kind,
            &input,
            &provenance.config_hash,
            &provenance.model_version,
            &provenance.image_digest,
        );
        Self {
            stage_run_id,
            kind,
            config_id,
            input,
            provenance,
            owner,
            submitted_at,
        }
    }

    /// Whether the stored id equals the id its identity inputs derive.
    #[must_use]
    pub fn has_derived_id(&self) -> bool {
        self.stage_run_id
            == derive_stage_run_id(
                self.kind,
                &self.input,
                &self.provenance.config_hash,
                &self.provenance.model_version,
                &self.provenance.image_digest,
            )
    }
}

/// One stored stage run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRecord {
    /// The stage run id.
    pub stage_run_id: StageRunId,
    /// Which engine executed.
    pub kind: StageKind,
    /// Its state.
    pub state: StageState,
    /// The first submitter.
    pub owner: String,
    /// The configuration applied.
    pub config_id: String,
    /// The single input the stage consumed.
    pub input: StageInput,
    /// What produced the stage run.
    pub provenance: StageProvenance,
    /// The artifact bundle, once the stage succeeded.
    pub artifact: Option<StageArtifact>,
    /// The submission time.
    pub submitted_at: String,
    /// The failure, when the stage failed.
    pub error: Option<String>,
    /// The number of deliveries claimed.
    pub attempts: u32,
    /// The monotonic lease term of the current attempt.
    ///
    /// A completion or failure must carry this term; a stale worker whose lease
    /// expired and was reclaimed cannot write the new attempt's artifact.
    pub lease_term: u64,
}

impl StageRecord {
    /// A queued record with no artifact.
    #[must_use]
    pub fn queued(run: &NewStageRun) -> Self {
        Self {
            stage_run_id: run.stage_run_id.clone(),
            kind: run.kind,
            state: StageState::Queued,
            owner: run.owner.clone(),
            config_id: run.config_id.clone(),
            input: run.input.clone(),
            provenance: run.provenance.clone(),
            artifact: None,
            submitted_at: run.submitted_at.clone(),
            error: None,
            attempts: 0,
            lease_term: 0,
        }
    }

    /// The list projection.
    #[must_use]
    pub fn summary(&self) -> StageRunSummary {
        StageRunSummary {
            stage_run_id: self.stage_run_id.clone(),
            kind: self.kind,
            state: self.state,
            config_id: self.config_id.clone(),
            submitted_at: self.submitted_at.clone(),
            artifact_id: self
                .artifact
                .as_ref()
                .map(|artifact| artifact.artifact_id.clone()),
            evidence: false,
        }
    }

    /// The detail projection.
    #[must_use]
    pub fn detail(&self) -> StageRunDetail {
        StageRunDetail {
            stage_run_id: self.stage_run_id.clone(),
            kind: self.kind,
            state: self.state,
            owner: self.owner.clone(),
            config_id: self.config_id.clone(),
            input: self.input.clone(),
            provenance: self.provenance.clone(),
            artifact: self.artifact.clone(),
            submitted_at: self.submitted_at.clone(),
            error: self.error.clone(),
            attempts: self.attempts,
            evidence: false,
        }
    }

    /// Whether the stored id equals the id its identity inputs derive.
    #[must_use]
    pub fn has_derived_id(&self) -> bool {
        self.stage_run_id
            == derive_stage_run_id(
                self.kind,
                &self.input,
                &self.provenance.config_hash,
                &self.provenance.model_version,
                &self.provenance.image_digest,
            )
    }
}

/// A worker's request to take one delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageClaim {
    /// The stage run the message names.
    pub stage_run_id: StageRunId,
    /// Whether the broker says this delivery was redelivered.
    ///
    /// The lease is the claim right; this flag is recorded for the worker log
    /// and compatibility, and it can never steal a live lease.
    pub redelivered: bool,
    /// The worker identity recorded in the stage lease.
    pub holder: String,
    /// The worker's own previous term when it retries the same delivery.
    #[serde(default)]
    pub continuation: Option<u64>,
}

/// A worker's lease renewal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageHeartbeat {
    /// The stage run the lease belongs to.
    pub stage_run_id: StageRunId,
    /// The worker identity that holds the lease.
    pub holder: String,
    /// The lease term being renewed.
    pub term: u64,
}

/// What a stage heartbeat did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageHeartbeatOutcome {
    /// The lease is renewed and the stage is still running.
    Renewed(Box<StageRecord>),
    /// The term or holder no longer owns the lease.
    Stale,
    /// The stage does not exist.
    Missing,
}

/// What a stage claim did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageClaimOutcome {
    /// The worker may execute the stage.
    Claimed(StageRecord),
    /// Another worker still holds the delivery.
    Running(StageRecord),
    /// The stage already reached a terminal state.
    Terminal(StageRecord),
    /// The retry budget is spent.
    Exhausted(StageRecord),
    /// The stage does not exist.
    Missing,
}

/// A worker's completion payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageComputed {
    /// The stage that produced the artifact.
    pub stage_run_id: StageRunId,
    /// The lease term the worker was granted.
    pub lease_term: u64,
    /// The content-addressed artifact bundle.
    pub artifact: StageArtifact,
}

/// A worker's failure payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageFailed {
    /// The stage that failed.
    pub stage_run_id: StageRunId,
    /// The lease term the worker was granted.
    pub lease_term: u64,
    /// The failure, as recorded on the stage row.
    pub error: String,
}

/// What a stage cancellation did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageCancelOutcome {
    /// The stage moved to `cancelled`.
    Cancelled(StageRecord),
    /// The stage was already terminal.
    Conflict(StageRecord),
    /// This profile does not see the stage.
    Missing,
}

/// Why a stage manifest could not be built or decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageManifestError(pub String);

impl std::fmt::Display for StageManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StageManifestError {}

/// The complete artifact set of one stage run.
///
/// The manifest is immutable and always diagnostic. Its digest is computed
/// over every file it names; it is never compared with, or promoted to, the
/// canonical digest of a coupled run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageManifest {
    /// The schema token.
    pub schema: String,
    /// The stage run it belongs to.
    pub stage_run_id: StageRunId,
    /// Which engine produced it.
    pub kind: StageKind,
    /// The dataset the stage ultimately reads.
    pub data_version: Digest,
    /// The configuration applied.
    pub config_id: String,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model registry version.
    pub model_version: String,
    /// The image digest that executed the stage.
    pub image_digest: Digest,
    /// The single input the stage consumed.
    pub input: StageInput,
    /// The artifact entries, sorted by name.
    pub artifacts: Vec<ArtifactRef>,
    /// Always `false`; a stage artifact is diagnostic and never evidence.
    pub evidence: bool,
}

impl StageManifest {
    /// Build a manifest for one stage run.
    ///
    /// # Errors
    ///
    /// Returns [`StageManifestError`] when the input does not belong to the
    /// kind, the id is not the one the identity derives, or the artifact set is
    /// empty, duplicated, or has a blank name.
    pub fn new(
        stage_run_id: StageRunId,
        kind: StageKind,
        config_id: String,
        provenance: &StageProvenance,
        input: StageInput,
        artifacts: Vec<ArtifactRef>,
    ) -> Result<Self, StageManifestError> {
        if !input.matches(kind) {
            return Err(StageManifestError(format!(
                "a {} stage cannot consume a {} input",
                kind.as_str(),
                input.label()
            )));
        }
        if kind == StageKind::Model && provenance.data_version != *input.digest() {
            return Err(StageManifestError(
                "a model manifest must record the dataset it consumes".to_owned(),
            ));
        }
        let derived = derive_stage_run_id(
            kind,
            &input,
            &provenance.config_hash,
            &provenance.model_version,
            &provenance.image_digest,
        );
        if stage_run_id != derived {
            return Err(StageManifestError(
                "the stage run id is not the one its identity derives".to_owned(),
            ));
        }
        let artifact = StageArtifact::from_files(artifacts)
            .map_err(|error| StageManifestError(error.to_string()))?;
        Ok(Self {
            schema: STAGE_MANIFEST_SCHEMA.to_owned(),
            stage_run_id,
            kind,
            data_version: provenance.data_version.clone(),
            config_id,
            config_hash: provenance.config_hash.clone(),
            model_version: provenance.model_version.clone(),
            image_digest: provenance.image_digest.clone(),
            input,
            artifacts: artifact.files,
            evidence: false,
        })
    }

    /// Build the manifest for a succeeded stage record.
    ///
    /// # Errors
    ///
    /// Returns [`StageManifestError`] when the record is not succeeded, carries
    /// no artifact, or its bundle is inconsistent.
    pub fn from_record(record: &StageRecord) -> Result<Self, StageManifestError> {
        if record.state != StageState::Succeeded {
            return Err(StageManifestError(format!(
                "stage {} is {}, not succeeded",
                record.stage_run_id,
                record.state.as_str()
            )));
        }
        let artifact = record
            .artifact
            .as_ref()
            .ok_or_else(|| StageManifestError("a succeeded stage has no artifact".to_owned()))?;
        if !artifact.is_consistent() {
            return Err(StageManifestError(
                "the stage artifact id does not match its files".to_owned(),
            ));
        }
        Self::new(
            record.stage_run_id.clone(),
            record.kind,
            record.config_id.clone(),
            &record.provenance,
            record.input.clone(),
            artifact.files.clone(),
        )
    }

    /// Decode and validate a manifest.
    ///
    /// # Errors
    ///
    /// Returns [`StageManifestError`] when the bytes are not JSON, the schema is
    /// not the current one, `evidence` is not `false`, the input and kind
    /// disagree, the run id is not the derived one, or the entries are empty,
    /// duplicated, or not sorted by name.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StageManifestError> {
        let manifest: Self = serde_json::from_slice(bytes)
            .map_err(|error| StageManifestError(format!("the manifest is not JSON: {error}")))?;
        if manifest.schema != STAGE_MANIFEST_SCHEMA {
            return Err(StageManifestError(format!(
                "the manifest schema is {}, not {STAGE_MANIFEST_SCHEMA}",
                manifest.schema
            )));
        }
        if manifest.evidence {
            return Err(StageManifestError(
                "a stage manifest is never evidence".to_owned(),
            ));
        }
        if !manifest.input.matches(manifest.kind) {
            return Err(StageManifestError(format!(
                "a {} stage cannot consume a {} input",
                manifest.kind.as_str(),
                manifest.input.label()
            )));
        }
        if manifest.kind == StageKind::Model && manifest.data_version != *manifest.input.digest() {
            return Err(StageManifestError(
                "a model manifest must record the dataset it consumes".to_owned(),
            ));
        }
        let derived = derive_stage_run_id(
            manifest.kind,
            &manifest.input,
            &manifest.config_hash,
            &manifest.model_version,
            &manifest.image_digest,
        );
        if manifest.stage_run_id != derived {
            return Err(StageManifestError(
                "the stage run id is not the one its identity derives".to_owned(),
            ));
        }
        let artifact = StageArtifact::from_files(manifest.artifacts.clone())
            .map_err(|error| StageManifestError(error.to_string()))?;
        if artifact.files != manifest.artifacts {
            return Err(StageManifestError(
                "the manifest entries are not sorted by name".to_owned(),
            ));
        }
        Ok(manifest)
    }

    /// Encode the manifest deterministically.
    ///
    /// # Errors
    ///
    /// Returns [`StageManifestError`] when serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StageManifestError> {
        serde_json::to_vec(self).map_err(|error| StageManifestError(error.to_string()))
    }

    /// The object key of the manifest for a stage run.
    #[must_use]
    pub fn key(stage_run_id: &StageRunId) -> String {
        format!("stages/{stage_run_id}/manifest.json")
    }

    /// The object key of one content-addressed stage blob.
    ///
    /// The blob namespace is disjoint from a run's `artifacts/{sha256}` key, so
    /// stage retention can delete a stage blob without ever reaching a run
    /// artifact. Identical stage bytes share one blob.
    #[must_use]
    pub fn blob_key(sha256: &Digest) -> String {
        format!("stages/blobs/{}", sha256.as_hex())
    }

    /// The bundle digest recomputed from the entries.
    ///
    /// This is the `model_artifact_id`, `decisions_artifact_id`, or
    /// `portfolio_artifact_id` the Engine Execution workflow names for the
    /// stage's output.
    #[must_use]
    pub fn computed_digest(&self) -> Digest {
        StageArtifact::digest(&self.artifacts)
    }
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

    fn input() -> StageInput {
        StageInput::DataVersion {
            data_version: digest(b"data"),
        }
    }

    fn new_run() -> NewStageRun {
        NewStageRun::new(
            StageKind::Model,
            "cfg-1".to_owned(),
            input(),
            provenance(),
            "operator".to_owned(),
            "2026-11-17T00:00:00Z".to_owned(),
        )
    }

    fn record() -> StageRecord {
        StageRecord::queued(&new_run())
    }

    fn manifest() -> StageManifest {
        StageManifest::new(
            new_run().stage_run_id,
            StageKind::Model,
            "cfg-1".to_owned(),
            &provenance(),
            input(),
            vec![file("model.json", b"{}")],
        )
        .expect("the manifest")
    }

    #[test]
    fn the_same_identity_derives_the_same_stage_id() {
        let first = new_run();
        let second = new_run();
        assert_eq!(first.stage_run_id, second.stage_run_id);
        assert!(first.has_derived_id());
    }

    #[test]
    fn every_identity_input_changes_the_stage_id() {
        let base = new_run().stage_run_id;
        let with_provenance = |config_hash: Digest, model_version: &str, image_digest: Digest| {
            NewStageRun::new(
                StageKind::Model,
                "cfg-1".to_owned(),
                input(),
                StageProvenance {
                    data_version: digest(b"data"),
                    config_hash,
                    model_version: model_version.to_owned(),
                    image_digest,
                },
                "operator".to_owned(),
                "2026-11-17T00:00:00Z".to_owned(),
            )
        };
        assert_ne!(
            base,
            with_provenance(digest(b"other"), "M1.3", digest(b"image")).stage_run_id
        );
        assert_ne!(
            base,
            with_provenance(digest(b"config"), "M2.0", digest(b"image")).stage_run_id
        );
        assert_ne!(
            base,
            with_provenance(digest(b"config"), "M1.3", digest(b"other")).stage_run_id
        );
        assert_ne!(
            base,
            NewStageRun::new(
                StageKind::Model,
                "cfg-1".to_owned(),
                StageInput::DataVersion {
                    data_version: digest(b"other"),
                },
                provenance(),
                "operator".to_owned(),
                "2026-11-17T00:00:00Z".to_owned(),
            )
            .stage_run_id
        );
        assert_ne!(
            base,
            NewStageRun::new(
                StageKind::Strategy,
                "cfg-1".to_owned(),
                StageInput::ModelArtifact {
                    model_artifact_id: digest(b"model"),
                },
                provenance(),
                "operator".to_owned(),
                "2026-11-17T00:00:00Z".to_owned(),
            )
            .stage_run_id
        );
    }

    #[test]
    fn a_stage_id_field_boundary_is_not_ambiguous() {
        let with = |model_version: &str| {
            derive_stage_run_id(
                StageKind::Model,
                &input(),
                &digest(b"config"),
                model_version,
                &digest(b"image"),
            )
        };
        assert_ne!(with("ab"), with("a\nb"));
    }

    #[test]
    fn a_forged_stage_id_is_detected() {
        let mut run = new_run();
        run.stage_run_id = StageRunId(digest(b"forged"));
        assert!(!run.has_derived_id());
        let mut record = StageRecord::queued(&new_run());
        record.stage_run_id = StageRunId(digest(b"forged"));
        assert!(!record.has_derived_id());
    }

    #[test]
    fn a_new_run_and_a_record_round_trip() {
        let run = new_run();
        let text = serde_json::to_string(&run).expect("encode");
        assert_eq!(
            serde_json::from_str::<NewStageRun>(&text).expect("decode"),
            run
        );
        let record = StageRecord::queued(&run);
        let text = serde_json::to_string(&record).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageRecord>(&text).expect("decode"),
            record
        );
        assert_eq!(record.state, StageState::Queued);
        assert_eq!(record.attempts, 0);
        assert_eq!(record.artifact, None);
        assert_eq!(record.error, None);
    }

    #[test]
    fn the_record_projections_are_never_evidence() {
        let mut record = record();
        record.state = StageState::Succeeded;
        record.attempts = 1;
        record.artifact =
            Some(StageArtifact::from_files(vec![file("model.json", b"{}")]).expect("the artifact"));
        assert!(!record.summary().evidence);
        assert!(!record.detail().evidence);
        assert_eq!(
            record.summary().artifact_id,
            record
                .artifact
                .as_ref()
                .map(|artifact| artifact.artifact_id.clone())
        );
        assert_eq!(record.detail().artifact, record.artifact);
    }

    #[test]
    fn a_record_is_not_a_run() {
        let text = serde_json::to_string(&record()).expect("encode");
        assert!(!text.contains("\"run_id\""));
        assert!(!text.contains("\"digest\""));
    }

    #[test]
    fn a_manifest_round_trips_and_exposes_its_digest() {
        let manifest = manifest();
        let text = manifest.to_bytes().expect("bytes");
        let decoded = StageManifest::from_bytes(&text).expect("decode");
        assert_eq!(decoded, manifest);
        assert_eq!(decoded.computed_digest(), manifest.computed_digest());
        assert_eq!(
            decoded.computed_digest(),
            StageArtifact::digest(&manifest.artifacts)
        );
        assert_eq!(decoded.artifacts, vec![file("model.json", b"{}")]);
        assert!(!decoded.evidence);
        assert_eq!(decoded.schema, STAGE_MANIFEST_SCHEMA);
        assert!(StageManifest::key(&decoded.stage_run_id).starts_with("stages/"));
        assert!(StageManifest::blob_key(&digest(b"bytes")).starts_with("stages/blobs/"));
        assert!(!StageManifest::blob_key(&digest(b"bytes")).starts_with("artifacts/"));
    }

    #[test]
    fn the_manifest_sorts_its_artifacts() {
        let manifest = StageManifest::new(
            new_run().stage_run_id,
            StageKind::Model,
            "cfg-1".to_owned(),
            &provenance(),
            input(),
            vec![file("results.json", b"r"), file("events.ndjson", b"e")],
        )
        .expect("the manifest");
        assert_eq!(
            manifest
                .artifacts
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["events.ndjson", "results.json"]
        );
        let reversed = StageManifest::new(
            new_run().stage_run_id,
            StageKind::Model,
            "cfg-1".to_owned(),
            &provenance(),
            input(),
            vec![file("events.ndjson", b"e"), file("results.json", b"r")],
        )
        .expect("the manifest");
        assert_eq!(manifest.computed_digest(), reversed.computed_digest());
    }

    #[test]
    fn a_model_manifest_that_changes_its_dataset_is_refused() {
        let mut provenance = provenance();
        provenance.data_version = digest(b"other");
        let error = StageManifest::new(
            new_run().stage_run_id,
            StageKind::Model,
            "cfg-1".to_owned(),
            &provenance,
            input(),
            vec![file("model.json", b"{}")],
        )
        .expect_err("a model manifest with another dataset");
        assert!(error.to_string().contains("dataset"), "{error}");
    }

    #[test]
    fn a_manifest_for_the_wrong_input_is_refused() {
        let error = StageManifest::new(
            new_run().stage_run_id,
            StageKind::Strategy,
            "cfg-1".to_owned(),
            &provenance(),
            input(),
            vec![file("model.json", b"{}")],
        )
        .expect_err("a strategy stage with a dataset input");
        assert!(error.to_string().contains("cannot consume"), "{error}");
    }

    #[test]
    fn a_manifest_with_a_foreign_id_is_refused() {
        let error = StageManifest::new(
            StageRunId(digest(b"forged")),
            StageKind::Model,
            "cfg-1".to_owned(),
            &provenance(),
            input(),
            vec![file("model.json", b"{}")],
        )
        .expect_err("a forged stage id");
        assert!(error.to_string().contains("derives"), "{error}");
    }

    #[test]
    fn a_manifest_rejects_a_wrong_schema_or_evidence_true() {
        let mut wrong = manifest();
        wrong.schema = "quantrule.stage-manifest/v2".to_owned();
        assert!(StageManifest::from_bytes(&serde_json::to_vec(&wrong).expect("bytes")).is_err());
        let mut evidence = manifest();
        evidence.evidence = true;
        let error = StageManifest::from_bytes(&serde_json::to_vec(&evidence).expect("bytes"))
            .expect_err("an evidence stage manifest");
        assert!(error.to_string().contains("never evidence"), "{error}");
    }

    #[test]
    fn a_manifest_rejects_unsorted_entries_and_unknown_fields() {
        let mut unsorted = manifest();
        unsorted.artifacts = vec![file("results.json", b"r"), file("events.ndjson", b"e")];
        let text = serde_json::to_vec(&unsorted).expect("bytes");
        let error = StageManifest::from_bytes(&text).expect_err("an unsorted manifest");
        assert!(error.to_string().contains("sorted"), "{error}");

        let text = format!(
            "{{\"schema\":{schema:?},\"stage_run_id\":\"sha256:{d}\",\"kind\":\"model\",\
             \"data_version\":\"sha256:{d}\",\"config_id\":\"cfg\",\
             \"config_hash\":\"sha256:{d}\",\"model_version\":\"M1.3\",\
             \"image_digest\":\"sha256:{d}\",\
             \"input\":{{\"kind\":\"data_version\",\"data_version\":\"sha256:{d}\"}},\
             \"artifacts\":[],\"evidence\":false,\"path\":\"/tmp\"}}",
            schema = STAGE_MANIFEST_SCHEMA,
            d = "a".repeat(64)
        );
        assert!(StageManifest::from_bytes(text.as_bytes()).is_err());
    }

    #[test]
    fn a_manifest_is_built_only_from_a_succeeded_record() {
        let record = record();
        assert!(StageManifest::from_record(&record).is_err());
        let mut record = record;
        record.state = StageState::Succeeded;
        assert!(StageManifest::from_record(&record).is_err(), "no artifact");
        record.artifact =
            Some(StageArtifact::from_files(vec![file("model.json", b"{}")]).expect("the artifact"));
        let manifest = StageManifest::from_record(&record).expect("the manifest");
        assert_eq!(manifest.stage_run_id, record.stage_run_id);
        assert_eq!(manifest.data_version, record.provenance.data_version);
    }

    #[test]
    fn the_worker_payloads_round_trip() {
        let claim = StageClaim {
            stage_run_id: new_run().stage_run_id,
            redelivered: false,
            holder: "worker".to_owned(),
            continuation: None,
        };
        let text = serde_json::to_string(&claim).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageClaim>(&text).expect("decode"),
            claim
        );
        let computed = StageComputed {
            stage_run_id: new_run().stage_run_id,
            lease_term: 1,
            artifact: StageArtifact::from_files(vec![file("model.json", b"{}")])
                .expect("the artifact"),
        };
        let text = serde_json::to_string(&computed).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageComputed>(&text).expect("decode"),
            computed
        );
        let failed = StageFailed {
            stage_run_id: new_run().stage_run_id,
            lease_term: 1,
            error: "the model failed".to_owned(),
        };
        let text = serde_json::to_string(&failed).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageFailed>(&text).expect("decode"),
            failed
        );
        assert!(serde_json::from_str::<StageClaim>(
            "{\"stage_run_id\":\"sha256:0000000000000000000000000000000000000000000000000000000000000000\",\
             \"redelivered\":false,\"profile\":\"x\"}"
        )
        .is_err());
    }

    #[test]
    fn a_claim_and_a_cancel_outcome_round_trip() {
        let outcome = StageClaimOutcome::Claimed(record());
        let text = serde_json::to_string(&outcome).expect("encode");
        assert!(text.contains("\"claimed\""));
        assert_eq!(
            serde_json::from_str::<StageClaimOutcome>(&text).expect("decode"),
            outcome
        );
        assert_eq!(
            serde_json::to_string(&StageCancelOutcome::Missing).expect("encode"),
            "\"missing\""
        );
        let cancelled = StageCancelOutcome::Cancelled(record());
        let text = serde_json::to_string(&cancelled).expect("encode");
        assert_eq!(
            serde_json::from_str::<StageCancelOutcome>(&text).expect("decode"),
            cancelled
        );
    }
}
