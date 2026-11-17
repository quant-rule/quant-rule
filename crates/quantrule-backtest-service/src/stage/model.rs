//! The model stage executor.
//!
//! A model stage publishes the model bundle the model service already computes
//! and stores. It never recomputes a reading: it asks the model client for the
//! content-addressed bundle manifest, verifies the manifest against the stage
//! record, loads every referenced object with its byte count and SHA-256
//! checked, and returns the exact bytes. The worker copies those bytes into the
//! diagnostic stage namespace, so a stage model artifact is byte-identical to
//! the coupled model bundle.
//!
//! The executor is diagnostic only. It writes no run row, canonical digest, or
//! strategy, execution, or portfolio artifact.

use std::sync::Arc;

use quantrule_api::{Digest, ProfileId, StageKind};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::{ClientError, ModelClient};
use quantrule_service_api::{MODEL_BUNDLE_SCHEMA, ModelBundleManifest, StageRecord};

use crate::stage::executor::{StageExecutor, StageExecutorError, StageFile, StageOutput};

/// The model stage executor.
pub struct ModelStageExecutor {
    model: Arc<dyn ModelClient>,
    objects: Arc<dyn ObjectStore>,
}

impl ModelStageExecutor {
    /// Build the executor over the model client and object store.
    #[must_use]
    pub fn new(model: Arc<dyn ModelClient>, objects: Arc<dyn ObjectStore>) -> Self {
        Self { model, objects }
    }
}

impl std::fmt::Debug for ModelStageExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ModelStageExecutor")
            .finish_non_exhaustive()
    }
}

impl StageExecutor for ModelStageExecutor {
    fn execute(&self, record: &StageRecord) -> BoxFuture<Result<StageOutput, StageExecutorError>> {
        let model = Arc::clone(&self.model);
        let objects = Arc::clone(&self.objects);
        let record = record.clone();
        Box::pin(async move { execute(&model, &objects, &record).await })
    }
}

async fn execute(
    model: &Arc<dyn ModelClient>,
    objects: &Arc<dyn ObjectStore>,
    record: &StageRecord,
) -> Result<StageOutput, StageExecutorError> {
    if record.kind != StageKind::Model {
        return Err(StageExecutorError(format!(
            "the model executor cannot run a {} stage",
            record.kind.as_str()
        )));
    }
    let data_version = record
        .input
        .data_version()
        .cloned()
        .ok_or_else(|| StageExecutorError("a model stage needs a dataset input".to_owned()))?;
    let profile = ProfileId::parse(&record.owner).map_err(|error| {
        StageExecutorError(format!("the stage owner is not a profile: {error}"))
    })?;
    let manifest = model
        .bundle(profile, data_version, record.config_id.clone())
        .await
        .map_err(client_error)?;
    verify_manifest(record, &manifest)?;
    let files = load_artifacts(objects, &manifest).await?;
    Ok(StageOutput { files })
}

/// Verify the model bundle belongs to this stage and carries a valid set.
fn verify_manifest(
    record: &StageRecord,
    manifest: &ModelBundleManifest,
) -> Result<(), StageExecutorError> {
    if manifest.schema != MODEL_BUNDLE_SCHEMA {
        return Err(StageExecutorError(format!(
            "the model bundle schema is {}, not {MODEL_BUNDLE_SCHEMA}",
            manifest.schema
        )));
    }
    if manifest.data_version != *record.input.digest() {
        return Err(StageExecutorError(
            "the model bundle belongs to another dataset".to_owned(),
        ));
    }
    if manifest.data_version != record.provenance.data_version {
        return Err(StageExecutorError(
            "the model bundle disagrees with the stage provenance".to_owned(),
        ));
    }
    if manifest.config_id != record.config_id {
        return Err(StageExecutorError(
            "the model bundle belongs to another config".to_owned(),
        ));
    }
    if manifest.config_hash != record.provenance.config_hash {
        return Err(StageExecutorError(
            "the model bundle names a different config hash".to_owned(),
        ));
    }
    if manifest.artifacts.is_empty() {
        return Err(StageExecutorError(
            "the model bundle carries no artifacts".to_owned(),
        ));
    }
    if manifest
        .artifacts
        .windows(2)
        .any(|pair| pair[0].name >= pair[1].name)
    {
        return Err(StageExecutorError(
            "the model bundle artifacts are not sorted by name".to_owned(),
        ));
    }
    if !manifest.manifest_key.ends_with("/manifest.json") {
        return Err(StageExecutorError(
            "the model bundle manifest key is not a bundle manifest".to_owned(),
        ));
    }
    Ok(())
}

/// Read every artifact under the manifest prefix and verify its bytes.
async fn load_artifacts(
    objects: &Arc<dyn ObjectStore>,
    manifest: &ModelBundleManifest,
) -> Result<Vec<StageFile>, StageExecutorError> {
    let prefix = manifest
        .manifest_key
        .strip_suffix("/manifest.json")
        .ok_or_else(|| {
            StageExecutorError("the model bundle manifest key is not a bundle manifest".to_owned())
        })?;
    let mut files = Vec::with_capacity(manifest.artifacts.len());
    for artifact in &manifest.artifacts {
        let key = if prefix.is_empty() {
            artifact.name.clone()
        } else {
            format!("{prefix}/{}", artifact.name)
        };
        let bytes = objects.get(&key).await.map_err(|error| {
            StageExecutorError(format!(
                "the model artifact {} is missing: {error}",
                artifact.name
            ))
        })?;
        if bytes.len() as u64 != artifact.bytes || Digest::sha256(&bytes) != artifact.sha256 {
            return Err(StageExecutorError(format!(
                "the model artifact {} does not match its manifest",
                artifact.name
            )));
        }
        files.push(StageFile {
            name: artifact.name.clone(),
            bytes,
        });
    }
    files.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(files)
}

fn client_error(error: ClientError) -> StageExecutorError {
    StageExecutorError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{ArtifactRef, StageInput, StageProvenance, StageState};
    use quantrule_runtime::adapters::memory::MemoryObjects;
    use quantrule_runtime::client::MemoryModel;

    fn profile() -> ProfileId {
        ProfileId::parse("default").expect("profile")
    }

    fn record(seed: &[u8]) -> StageRecord {
        let data_version = Digest::sha256(seed);
        let run = quantrule_service_api::NewStageRun::new(
            StageKind::Model,
            "cfg_1".to_owned(),
            StageInput::DataVersion {
                data_version: data_version.clone(),
            },
            StageProvenance {
                data_version,
                config_hash: Digest::sha256(b"config"),
                model_version: "M1.3".to_owned(),
                image_digest: Digest::sha256(b"image"),
            },
            "default".to_owned(),
            "2026-11-17T00:00:00Z".to_owned(),
        );
        let mut record = StageRecord::queued(&run);
        record.state = StageState::Running;
        record.attempts = 1;
        record
    }

    fn manifest(record: &StageRecord, files: &[(&str, &[u8])]) -> ModelBundleManifest {
        ModelBundleManifest {
            schema: MODEL_BUNDLE_SCHEMA.to_owned(),
            data_version: record.provenance.data_version.clone(),
            config_id: record.config_id.clone(),
            config_hash: record.provenance.config_hash.clone(),
            model_version: "2026-11-10.2".to_owned(),
            ladder: vec!["1m".to_owned()],
            manifest_key: format!("model/{}/manifest.json", record.stage_run_id),
            artifacts: files
                .iter()
                .map(|(name, bytes)| ArtifactRef {
                    name: (*name).to_owned(),
                    bytes: bytes.len() as u64,
                    sha256: Digest::sha256(bytes),
                })
                .collect(),
        }
    }

    async fn fixture(
        seed: &[u8],
    ) -> (StageRecord, MemoryModel, MemoryObjects, ModelBundleManifest) {
        let record = record(seed);
        let files = [
            ("readings.csv", b"1\n".as_slice()),
            ("report.json", b"{}".as_slice()),
        ];
        let manifest = manifest(&record, &files);
        let model = MemoryModel::new();
        model.add_bundle(&profile(), manifest.clone());
        let objects = MemoryObjects::default();
        for (name, bytes) in files {
            objects
                .put(
                    &format!(
                        "{}/{}",
                        manifest
                            .manifest_key
                            .strip_suffix("/manifest.json")
                            .expect("prefix"),
                        name
                    ),
                    bytes.to_vec(),
                )
                .await
                .expect("artifact");
        }
        (record, model, objects, manifest)
    }

    #[tokio::test]
    async fn the_model_bundle_bytes_are_returned_in_name_order() {
        let (record, model, objects, _) = fixture(b"data").await;
        let executor = ModelStageExecutor::new(Arc::new(model), Arc::new(objects));
        let output = executor.execute(&record).await.expect("execute");
        assert_eq!(
            output
                .files
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["readings.csv", "report.json"]
        );
        assert_eq!(output.files[0].bytes, b"1\n");
    }

    #[tokio::test]
    async fn the_executor_is_deterministic() {
        let (record, model, objects, _) = fixture(b"data").await;
        let executor = ModelStageExecutor::new(Arc::new(model), Arc::new(objects));
        let first = executor.execute(&record).await.expect("first");
        let second = executor.execute(&record).await.expect("second");
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn a_non_model_stage_is_refused() {
        let (mut record, model, objects, _) = fixture(b"data").await;
        record.kind = StageKind::Strategy;
        let executor = ModelStageExecutor::new(Arc::new(model), Arc::new(objects));
        let error = executor.execute(&record).await.expect_err("wrong kind");
        assert!(error.to_string().contains("model executor"), "{error}");
    }

    #[tokio::test]
    async fn another_profile_has_no_bundle() {
        let (mut record, model, objects, _) = fixture(b"data").await;
        record.owner = "alpha".to_owned();
        let executor = ModelStageExecutor::new(Arc::new(model), Arc::new(objects));
        let error = executor.execute(&record).await.expect_err("wrong profile");
        assert!(error.to_string().contains("model"), "{error}");
    }

    #[tokio::test]
    async fn a_manifest_that_disagrees_with_the_record_is_refused() {
        for mutate in [
            |manifest: &mut ModelBundleManifest| manifest.config_hash = Digest::sha256(b"other"),
            |manifest: &mut ModelBundleManifest| manifest.data_version = Digest::sha256(b"other"),
            |manifest: &mut ModelBundleManifest| manifest.schema = "other".to_owned(),
        ] {
            let (record, _, objects, mut manifest) = fixture(b"data").await;
            mutate(&mut manifest);
            let model = MemoryModel::new();
            model.add_bundle(&profile(), manifest);
            let executor = ModelStageExecutor::new(Arc::new(model), Arc::new(objects));
            assert!(executor.execute(&record).await.is_err());
        }
    }

    #[tokio::test]
    async fn a_registry_model_version_different_from_the_run_model_id_is_accepted() {
        let (record, model, objects, manifest) = fixture(b"data").await;
        assert_ne!(manifest.model_version, record.provenance.model_version);
        assert!(!manifest.model_version.is_empty());
        let executor = ModelStageExecutor::new(Arc::new(model), Arc::new(objects));
        let output = executor.execute(&record).await.expect("registry version");
        assert_eq!(output.files.len(), 2);
    }

    #[tokio::test]
    async fn a_missing_or_changed_blob_is_refused() {
        let (record, model, objects, manifest) = fixture(b"data").await;
        let prefix = manifest
            .manifest_key
            .strip_suffix("/manifest.json")
            .expect("prefix");
        let executor = ModelStageExecutor::new(Arc::new(model.clone()), Arc::new(objects.clone()));
        objects
            .delete(&format!("{prefix}/readings.csv"))
            .await
            .expect("delete");
        assert!(executor.execute(&record).await.is_err());

        objects
            .put(&format!("{prefix}/readings.csv"), b"changed\n".to_vec())
            .await
            .expect("put");
        assert!(executor.execute(&record).await.is_err());
    }
}
