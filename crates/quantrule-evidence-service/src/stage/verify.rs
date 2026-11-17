//! Verify and record a computed stage artifact.
//!
//! The stage worker writes every blob, then the manifest, then asks evidence to
//! record the result. Evidence reads the manifest back, checks it against the
//! stage row and the artifact the worker reported, and verifies every blob
//! before it marks the stage succeeded. A manifest is the visible edge of the
//! bundle: a vanished or mismatched manifest leaves the stage running, and a
//! repeated request is idempotent because only a running stage can finish.
//!
//! The module never names a run. A stage bundle has its own `stages/` object
//! namespace and its own schema.

use quantrule_api::{Digest, StageArtifact, StageState};
use quantrule_runtime::adapters::{ObjectStore, StoreError};
use quantrule_service_api::StageManifest;

use crate::stage::{StageRecord, StageStore, StageStoreError};

/// Why a stage artifact could not be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageVerifyError {
    /// The stage row does not exist.
    Missing(String),
    /// The manifest is absent or cannot be decoded.
    Manifest(String),
    /// The manifest disagrees with the stage row or the reported artifact.
    Mismatch(String),
    /// A blob is absent or changed.
    Artifact(String),
    /// The stage store refused a statement.
    Store(String),
}

impl std::fmt::Display for StageVerifyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(message)
            | Self::Manifest(message)
            | Self::Mismatch(message)
            | Self::Artifact(message)
            | Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for StageVerifyError {}

/// Verify a stage artifact and finish the stage.
///
/// # Errors
///
/// Returns [`StageVerifyError`] when the stage is missing, the manifest does not
/// match, a blob is absent or changed, or a store fails. A stage that is
/// already terminal is returned unchanged; completion is idempotent.
pub async fn record_computed(
    stages: &dyn StageStore,
    objects: &dyn ObjectStore,
    stage_run_id: &str,
    artifact: StageArtifact,
) -> Result<StageRecord, StageVerifyError> {
    let record = stages
        .get_any(stage_run_id.to_owned())
        .await
        .map_err(store_error)?
        .ok_or_else(|| StageVerifyError::Missing(format!("stage {stage_run_id} is unknown")))?;
    if record.state != StageState::Running {
        return Ok(record);
    }
    let manifest = read_manifest(objects, &record).await?;
    verify_manifest(&record, &artifact, &manifest)?;
    for file in &manifest.artifacts {
        let key = StageManifest::blob_key(&file.sha256);
        let bytes = objects.get(&key).await.map_err(|error| {
            StageVerifyError::Artifact(format!("stage artifact {} is missing: {error}", file.name))
        })?;
        if bytes.len() as u64 != file.bytes || Digest::sha256(&bytes) != file.sha256 {
            return Err(StageVerifyError::Artifact(format!(
                "stage artifact {} does not match its hash",
                file.name
            )));
        }
    }
    stages
        .complete(stage_run_id.to_owned(), artifact)
        .await
        .map_err(store_error)
}

async fn read_manifest(
    objects: &dyn ObjectStore,
    record: &StageRecord,
) -> Result<StageManifest, StageVerifyError> {
    let key = StageManifest::key(&record.stage_run_id);
    let bytes = objects.get(&key).await.map_err(|error| {
        StageVerifyError::Manifest(format!(
            "stage {} has no manifest: {error}",
            record.stage_run_id
        ))
    })?;
    StageManifest::from_bytes(&bytes).map_err(|error| StageVerifyError::Manifest(error.to_string()))
}

fn verify_manifest(
    record: &StageRecord,
    artifact: &StageArtifact,
    manifest: &StageManifest,
) -> Result<(), StageVerifyError> {
    if manifest.stage_run_id != record.stage_run_id {
        return Err(StageVerifyError::Mismatch(
            "the stage manifest belongs to another stage".to_owned(),
        ));
    }
    if manifest.kind != record.kind {
        return Err(StageVerifyError::Mismatch(
            "the stage manifest names another kind".to_owned(),
        ));
    }
    if manifest.input != record.input
        || manifest.config_id != record.config_id
        || manifest.config_hash != record.provenance.config_hash
    {
        return Err(StageVerifyError::Mismatch(
            "the stage manifest disagrees with the stage row".to_owned(),
        ));
    }
    if !artifact.is_consistent() {
        return Err(StageVerifyError::Mismatch(
            "the stage artifact id does not match its files".to_owned(),
        ));
    }
    if manifest.computed_digest() != artifact.artifact_id || manifest.artifacts != artifact.files {
        return Err(StageVerifyError::Mismatch(
            "the stage manifest disagrees with the reported artifact".to_owned(),
        ));
    }
    Ok(())
}

fn store_error(error: StageStoreError) -> StageVerifyError {
    StageVerifyError::Store(error.to_string())
}

impl From<StoreError> for StageVerifyError {
    fn from(error: StoreError) -> Self {
        Self::Manifest(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{ArtifactRef, ProfileId, StageInput, StageKind, StageProvenance};
    use quantrule_runtime::adapters::memory::MemoryObjects;
    use quantrule_service_api::NewStageRun;

    use crate::stage::MemoryStages;

    fn profile() -> ProfileId {
        ProfileId::parse("default").expect("profile")
    }

    fn run() -> NewStageRun {
        let data_version = Digest::sha256(b"data");
        NewStageRun::new(
            StageKind::Model,
            "cfg-1".to_owned(),
            StageInput::DataVersion {
                data_version: data_version.clone(),
            },
            StageProvenance {
                data_version,
                config_hash: Digest::sha256(b"config"),
                model_version: "M1.0".to_owned(),
                image_digest: Digest::sha256(b"image"),
            },
            "default".to_owned(),
            "2026-11-17T00:00:00Z".to_owned(),
        )
    }

    fn artifact() -> (StageArtifact, Vec<u8>) {
        let bytes = b"{\"coverage\":1}".to_vec();
        let file = ArtifactRef {
            name: "results.json".to_owned(),
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(&bytes),
        };
        (
            StageArtifact::from_files(vec![file]).expect("artifact"),
            bytes,
        )
    }

    async fn running_stage() -> (MemoryStages, MemoryObjects, StageRecord, StageArtifact) {
        let stages = MemoryStages::default();
        let run = run();
        let id = run.stage_run_id.to_string();
        stages.create(profile(), run).await.expect("create");
        let record = match stages.claim(id, false).await.expect("claim") {
            quantrule_service_api::StageClaimOutcome::Claimed(record) => record,
            other => panic!("the stage was not claimed: {other:?}"),
        };
        let (artifact, bytes) = artifact();
        let objects = MemoryObjects::default();
        let manifest = StageManifest::new(
            record.stage_run_id.clone(),
            record.kind,
            record.config_id.clone(),
            &record.provenance,
            record.input.clone(),
            artifact.files.clone(),
        )
        .expect("manifest");
        objects
            .put(&StageManifest::blob_key(&artifact.files[0].sha256), bytes)
            .await
            .expect("blob");
        objects
            .put(
                &StageManifest::key(&record.stage_run_id),
                manifest.to_bytes().expect("bytes"),
            )
            .await
            .expect("manifest");
        (stages, objects, record, artifact)
    }

    #[tokio::test]
    async fn a_verified_artifact_finishes_the_stage() {
        let (stages, objects, record, artifact) = running_stage().await;
        let done = record_computed(
            &stages,
            &objects,
            &record.stage_run_id.to_string(),
            artifact,
        )
        .await
        .expect("computed");
        assert_eq!(done.state, StageState::Succeeded);
        assert!(done.artifact.is_some());
    }

    #[tokio::test]
    async fn a_missing_manifest_is_refused() {
        let (stages, objects, record, artifact) = running_stage().await;
        objects.remove(&StageManifest::key(&record.stage_run_id));
        let error = record_computed(
            &stages,
            &objects,
            &record.stage_run_id.to_string(),
            artifact,
        )
        .await
        .expect_err("no manifest");
        assert!(matches!(error, StageVerifyError::Manifest(_)), "{error}");
        let record = stages
            .get_any(record.stage_run_id.to_string())
            .await
            .expect("get")
            .expect("record");
        assert_eq!(record.state, StageState::Running);
    }

    #[tokio::test]
    async fn a_changed_blob_is_refused() {
        let (stages, objects, record, artifact) = running_stage().await;
        let key = StageManifest::blob_key(&artifact.files[0].sha256);
        objects
            .put(&key, b"changed".to_vec())
            .await
            .expect("overwrite");
        let error = record_computed(
            &stages,
            &objects,
            &record.stage_run_id.to_string(),
            artifact,
        )
        .await
        .expect_err("changed blob");
        assert!(matches!(error, StageVerifyError::Artifact(_)), "{error}");
    }

    #[tokio::test]
    async fn a_terminal_stage_is_returned_unchanged() {
        let (stages, objects, record, artifact) = running_stage().await;
        stages
            .cancel(profile(), record.stage_run_id.to_string())
            .await
            .expect("cancel");
        let returned = record_computed(
            &stages,
            &objects,
            &record.stage_run_id.to_string(),
            artifact,
        )
        .await
        .expect("terminal");
        assert_eq!(returned.state, StageState::Cancelled);
    }
}
