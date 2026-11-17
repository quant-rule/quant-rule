//! Execute one claimed run and record its artifacts.
//!
//! The run row is the sequence: claim through evidence, read the config and
//! dataset through data, ask model for a bundle, run the staged strategy,
//! execution, and portfolio use case, write the artifact set to CAS, and record
//! the digest through evidence. The model is never recomputed and no candle,
//! reading, or artifact payload crosses the internal surface.
//!
//! A retryable failure holds the delivery and retries in place, so the broker's
//! redelivery flag is the only way a running run is reclaimed. The worker never
//! renders a label or accepts a client path.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunState, StageKind, StageName, StageProgress, StageState,
    StageStatus,
};
use quantrule_app::service::AppService;
use quantrule_app::stages::backtest_from_model::BacktestFromModelRequest;
use quantrule_service_api::{
    MODEL_BUNDLE_SCHEMA, ModelBundleManifest, RunClaimOutcome, RunManifest, RunRecord,
    StageManifest,
};

use crate::backtest::Workspace;
use crate::backtest::retry::{self, RetryDecision};
use quantrule_runtime::adapters::{ObjectStore, Queue};
use quantrule_runtime::client::{
    ClientError, DataClient, EvidenceClient, ModelClient, StageClient,
};
use quantrule_runtime::queue::{ROUTING_COMPUTED, ROUTING_DEAD, ROUTING_FAILED};

/// A worker failure with a human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerError(pub String);

impl std::fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for WorkerError {}

/// What one delivery did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessOutcome {
    /// The run's artifacts and canonical digest were recorded; evidence still
    /// has to verify and finalize it.
    Computed,
    /// The run failed terminally on this attempt.
    Failed,
    /// A retryable failure; hold the delivery for the delay and try again.
    Retry {
        /// How long to wait before the next attempt.
        delay: Duration,
    },
    /// The retry budget is spent; the run is failed and dead-lettered.
    Exhausted,
    /// There was nothing to do: missing, running, computed, or terminal.
    Skipped,
}

/// The stores a worker needs.
pub struct WorkerServices {
    /// The evidence lifecycle.
    pub evidence: Arc<dyn EvidenceClient>,
    /// The data service's internal reads.
    pub data: Arc<dyn DataClient>,
    /// The model service's bundle reference.
    pub model: Arc<dyn ModelClient>,
    /// The evidence stage lookup, for a promoted run.
    pub stages: Arc<dyn StageClient>,
    /// The object store.
    pub objects: Arc<dyn ObjectStore>,
    /// The control-plane queue.
    pub queue: Arc<dyn Queue>,
}

/// Process one `run.requested` run id.
///
/// `redelivered` is the broker's flag. It is the only proof that the previous
/// holder is gone, so it is what lets a running run be reclaimed.
///
/// # Errors
///
/// Returns [`WorkerError`] when a store or the object store is unavailable. A
/// stage failure is recorded or retried and returned as an outcome, not an
/// error, so the consumer can acknowledge it.
pub async fn process(
    run_id: String,
    redelivered: bool,
    services: &WorkerServices,
) -> Result<ProcessOutcome, WorkerError> {
    let record = match services
        .evidence
        .claim(run_id.clone(), redelivered)
        .await
        .map_err(|error| WorkerError(error.to_string()))?
    {
        RunClaimOutcome::Claimed(record) => record,
        RunClaimOutcome::Terminal(_) | RunClaimOutcome::Missing => {
            return Ok(ProcessOutcome::Skipped);
        }
        RunClaimOutcome::Running(record) => {
            // A delivery after the worker wrote its artifacts must not execute
            // again. The recorded digest is what tells the two `running` states
            // apart; publishing again closes a lost publish.
            if record.digest.is_some() && !record.artifacts.is_empty() {
                publish(&services.queue, ROUTING_COMPUTED, &run_id).await?;
            }
            return Ok(ProcessOutcome::Skipped);
        }
        RunClaimOutcome::Exhausted(record) => {
            return exhaust(
                services,
                &run_id,
                "the retry budget was exhausted",
                record.attempts,
            )
            .await;
        }
    };
    let attempts = record.attempts;
    if cancelled(services, &run_id).await? {
        return Ok(ProcessOutcome::Skipped);
    }

    let profile = ProfileId::parse(&record.owner)
        .map_err(|error| WorkerError(format!("the run owner is invalid: {error}")))?;
    let config = match services
        .data
        .config_bundle(profile.clone(), record.config_id.clone())
        .await
    {
        Ok(Some(config)) => config,
        Ok(None) => return fail(services, &run_id, "the config is unknown").await,
        Err(error) => return client_failure(services, &run_id, &error, attempts).await,
    };
    if !config.valid {
        return fail(
            services,
            &run_id,
            config.message.as_deref().unwrap_or("the config is invalid"),
        )
        .await;
    }
    if config.config_hash.as_ref() != Some(&record.config_hash) {
        return fail(
            services,
            &run_id,
            "the config hash changed after submission",
        )
        .await;
    }
    let dataset = match services
        .data
        .dataset_object(profile.clone(), record.data_version.to_string())
        .await
    {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return fail(services, &run_id, "the dataset is unknown").await,
        Err(error) => return client_failure(services, &run_id, &error, attempts).await,
    };
    let bytes = match services.objects.get(&dataset.object_key).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
    };
    if Digest::sha256(&bytes) != record.data_version {
        return fail(services, &run_id, "the dataset bytes do not match").await;
    }

    // The row is authoritative; a cancel before the model call writes nothing.
    if cancelled(services, &run_id).await? {
        return Ok(ProcessOutcome::Skipped);
    }
    let model_files = if let Some(artifact_id) = record.model_artifact_id.as_ref() {
        match load_promoted_model(profile, artifact_id.clone(), &record, services).await {
            Ok(files) => files,
            Err(ModelRead::Store(message)) => {
                return retry_or_exhaust(services, &run_id, &message, attempts).await;
            }
            Err(ModelRead::Mismatch(message)) => {
                return fail(services, &run_id, &message).await;
            }
        }
    } else {
        let manifest = match services
            .model
            .bundle(
                profile,
                record.data_version.clone(),
                record.config_id.clone(),
            )
            .await
        {
            Ok(manifest) => manifest,
            Err(error) => return client_failure(services, &run_id, &error, attempts).await,
        };
        if let Err(message) = verify_manifest(&manifest, &record) {
            return fail(services, &run_id, &message).await;
        }
        match load_model_artifacts(&services.objects, &manifest).await {
            Ok(files) => files,
            Err(ModelRead::Store(message)) => {
                return retry_or_exhaust(services, &run_id, &message, attempts).await;
            }
            Err(ModelRead::Mismatch(message)) => {
                return fail(services, &run_id, &message).await;
            }
        }
    };

    let workspace = match tokio::task::spawn_blocking(move || {
        let workspace = Workspace::create(&config, &bytes)?;
        for (name, bytes) in model_files {
            workspace.write_model_artifact(&name, &bytes)?;
        }
        Ok::<Workspace, WorkerError>(workspace)
    })
    .await
    {
        Ok(Ok(workspace)) => workspace,
        Ok(Err(error)) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
        Err(error) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
    };
    // A cancel before the backtest must leave the materialized workspace unread.
    if cancelled(services, &run_id).await? {
        return Ok(ProcessOutcome::Skipped);
    }
    let run_path = workspace.run_config();
    let strategy_path = workspace.strategy();
    let model_path = workspace.model();
    let out = workspace.out();
    let outcome = tokio::task::spawn_blocking(move || {
        AppService::run_backtest_from_model(&BacktestFromModelRequest {
            config: &run_path,
            strategy: &strategy_path,
            model: &model_path,
            out: Some(&out),
        })
    })
    .await;
    let outcome = match outcome {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(error)) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
        Err(error) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
    };
    // A cancel after the backtest must leave no bundle. Check before the first
    // CAS write, not after the upload.
    if cancelled(services, &run_id).await? {
        return Ok(ProcessOutcome::Skipped);
    }
    let artifacts = match publish_artifacts(&services.objects, &outcome.root).await {
        Ok(artifacts) => artifacts,
        Err(error) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
    };
    let run_manifest = RunManifest::from_artifacts(&run_id, &artifacts);
    let manifest_bytes = run_manifest
        .to_bytes()
        .map_err(|error| WorkerError(error.to_string()))?;
    if let Err(error) = services
        .objects
        .put(&RunManifest::key(&run_id), manifest_bytes)
        .await
    {
        return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
    }
    let record = services
        .evidence
        .computed(
            run_id.clone(),
            run_manifest.digest.clone(),
            run_manifest.refs(),
            computed_stages(),
        )
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    if record.status != RunState::Running {
        return Ok(ProcessOutcome::Skipped);
    }
    publish(&services.queue, ROUTING_COMPUTED, &run_id).await?;
    Ok(ProcessOutcome::Computed)
}

async fn cancelled(services: &WorkerServices, run_id: &str) -> Result<bool, WorkerError> {
    Ok(services
        .evidence
        .get_run(run_id.to_owned())
        .await
        .map_err(|error| WorkerError(error.to_string()))?
        .is_some_and(|record| record.status == RunState::Cancelled))
}

/// Whether a client failure will not improve with a retry.
///
/// The failure map makes a missing or invalid input terminal and an unreachable
/// or overloaded service retryable. A malformed internal response is a contract
/// defect, so retrying it cannot succeed either.
fn terminal(error: &ClientError) -> bool {
    match error {
        ClientError::Status { status, .. } => {
            (400..500).contains(status) && *status != 408 && *status != 429
        }
        ClientError::InvalidResponse { .. }
        | ClientError::InvalidBase { .. }
        | ClientError::Unconfigured { .. } => true,
        ClientError::Unreachable { .. } | ClientError::Timeout { .. } => false,
    }
}

async fn client_failure(
    services: &WorkerServices,
    run_id: &str,
    error: &ClientError,
    attempts: u32,
) -> Result<ProcessOutcome, WorkerError> {
    if terminal(error) {
        fail(services, run_id, &error.to_string()).await
    } else {
        retry_or_exhaust(services, run_id, &error.to_string(), attempts).await
    }
}

/// Read the frozen model stage a promoted run names.
async fn load_promoted_model(
    profile: ProfileId,
    artifact_id: Digest,
    record: &RunRecord,
    services: &WorkerServices,
) -> Result<Vec<(String, Vec<u8>)>, ModelRead> {
    let stage = services
        .stages
        .get_stage_by_artifact(profile, StageKind::Model, artifact_id.clone())
        .await
        .map_err(|error| {
            if terminal(&error) {
                ModelRead::Mismatch(error.to_string())
            } else {
                ModelRead::Store(error.to_string())
            }
        })?
        .ok_or_else(|| {
            ModelRead::Mismatch(format!(
                "no succeeded model stage publishes artifact {artifact_id}"
            ))
        })?;
    if stage.kind != StageKind::Model || stage.state != StageState::Succeeded {
        return Err(ModelRead::Mismatch(
            "the model artifact is not a succeeded model stage".to_owned(),
        ));
    }
    let artifact = stage
        .artifact
        .as_ref()
        .ok_or_else(|| ModelRead::Mismatch("the model stage carries no artifact".to_owned()))?;
    if artifact.artifact_id != artifact_id {
        return Err(ModelRead::Mismatch(
            "the model stage publishes another artifact".to_owned(),
        ));
    }
    if stage.provenance.data_version != record.data_version {
        return Err(ModelRead::Mismatch(
            "the model artifact belongs to another dataset".to_owned(),
        ));
    }
    if stage.provenance.config_hash != record.config_hash {
        return Err(ModelRead::Mismatch(
            "the model artifact belongs to another config hash".to_owned(),
        ));
    }
    if stage.provenance.model_version != record.model_version {
        return Err(ModelRead::Mismatch(
            "the model artifact belongs to another model version".to_owned(),
        ));
    }
    let manifest_bytes = services
        .objects
        .get(&StageManifest::key(&stage.stage_run_id))
        .await
        .map_err(|error| ModelRead::Store(error.to_string()))?;
    let manifest = StageManifest::from_bytes(&manifest_bytes)
        .map_err(|error| ModelRead::Mismatch(error.to_string()))?;
    if manifest.stage_run_id != stage.stage_run_id
        || manifest.kind != StageKind::Model
        || manifest.config_id != record.config_id
        || manifest.data_version != record.data_version
        || manifest.config_hash != record.config_hash
        || manifest.model_version != record.model_version
        || manifest.artifacts != artifact.files
    {
        return Err(ModelRead::Mismatch(
            "the model stage manifest disagrees with the run record".to_owned(),
        ));
    }
    let mut files = Vec::with_capacity(manifest.artifacts.len());
    for file in &manifest.artifacts {
        let bytes = services
            .objects
            .get(&StageManifest::blob_key(&file.sha256))
            .await
            .map_err(|error| ModelRead::Store(error.to_string()))?;
        if bytes.len() as u64 != file.bytes || Digest::sha256(&bytes) != file.sha256 {
            return Err(ModelRead::Mismatch(format!(
                "the model stage artifact {} does not match its manifest",
                file.name
            )));
        }
        files.push((file.name.clone(), bytes));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

/// Verify that the model bundle is the one the run was submitted for.
fn verify_manifest(manifest: &ModelBundleManifest, record: &RunRecord) -> Result<(), String> {
    if manifest.schema != MODEL_BUNDLE_SCHEMA {
        return Err(format!(
            "the model bundle schema is {}, not {MODEL_BUNDLE_SCHEMA}",
            manifest.schema
        ));
    }
    if manifest.data_version != record.data_version {
        return Err("the model bundle belongs to another dataset".to_owned());
    }
    if manifest.config_id != record.config_id {
        return Err("the model bundle belongs to another config".to_owned());
    }
    if manifest.config_hash != record.config_hash {
        return Err("the model bundle names a different config hash".to_owned());
    }
    Ok(())
}

/// Why a model artifact could not be materialized.
enum ModelRead {
    /// The object store could not be read; retry it.
    Store(String),
    /// The bytes do not match the manifest; stop the run.
    Mismatch(String),
}

/// Read and verify every artifact the model manifest lists.
async fn load_model_artifacts(
    objects: &Arc<dyn ObjectStore>,
    manifest: &ModelBundleManifest,
) -> Result<Vec<(String, Vec<u8>)>, ModelRead> {
    let prefix = manifest
        .manifest_key
        .strip_suffix("/manifest.json")
        .ok_or_else(|| {
            ModelRead::Mismatch(format!(
                "the model manifest key {} is not a bundle manifest",
                manifest.manifest_key
            ))
        })?;
    let mut files = Vec::with_capacity(manifest.artifacts.len());
    for artifact in &manifest.artifacts {
        let key = if prefix.is_empty() {
            artifact.name.clone()
        } else {
            format!("{prefix}/{}", artifact.name)
        };
        let bytes = objects
            .get(&key)
            .await
            .map_err(|error| ModelRead::Store(error.to_string()))?;
        if bytes.len() as u64 != artifact.bytes || Digest::sha256(&bytes) != artifact.sha256 {
            return Err(ModelRead::Mismatch(format!(
                "the model bundle artifact {} does not match its manifest",
                artifact.name
            )));
        }
        files.push((artifact.name.clone(), bytes));
    }
    Ok(files)
}

async fn retry_or_exhaust(
    services: &WorkerServices,
    run_id: &str,
    message: &str,
    attempts: u32,
) -> Result<ProcessOutcome, WorkerError> {
    match retry::decide(attempts) {
        RetryDecision::Retry(delay) => Ok(ProcessOutcome::Retry { delay }),
        RetryDecision::DeadLetter => exhaust(services, run_id, message, attempts).await,
    }
}

async fn exhaust(
    services: &WorkerServices,
    run_id: &str,
    message: &str,
    _attempts: u32,
) -> Result<ProcessOutcome, WorkerError> {
    let record = services
        .evidence
        .failed(run_id.to_owned(), message.to_owned(), failed_stages())
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    if record.status != RunState::Failed {
        return Ok(ProcessOutcome::Skipped);
    }
    publish(&services.queue, ROUTING_DEAD, run_id).await?;
    Ok(ProcessOutcome::Exhausted)
}

async fn fail(
    services: &WorkerServices,
    run_id: &str,
    message: &str,
) -> Result<ProcessOutcome, WorkerError> {
    let record = services
        .evidence
        .failed(run_id.to_owned(), message.to_owned(), failed_stages())
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    if record.status != RunState::Failed {
        return Ok(ProcessOutcome::Skipped);
    }
    publish(&services.queue, ROUTING_FAILED, run_id).await?;
    Ok(ProcessOutcome::Failed)
}

async fn publish(
    queue: &Arc<dyn Queue>,
    routing_key: &str,
    run_id: &str,
) -> Result<(), WorkerError> {
    queue
        .publish(
            routing_key,
            serde_json::to_vec(&serde_json::json!({ "run_id": run_id }))
                .map_err(|error| WorkerError(error.to_string()))?,
        )
        .await
        .map_err(|error| WorkerError(error.to_string()))
}

async fn publish_artifacts(
    objects: &Arc<dyn ObjectStore>,
    root: &Path,
) -> Result<Vec<ArtifactRef>, WorkerError> {
    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut artifacts = Vec::with_capacity(files.len());
    for (name, bytes) in files {
        let sha256 = Digest::sha256(&bytes);
        let key = RunManifest::blob_key(&sha256);
        let present = objects
            .exists(&key)
            .await
            .map_err(|error| WorkerError(error.to_string()))?;
        if !present {
            objects
                .put(&key, bytes.clone())
                .await
                .map_err(|error| WorkerError(error.to_string()))?;
        }
        artifacts.push(ArtifactRef {
            name,
            bytes: bytes.len() as u64,
            sha256,
        });
    }
    Ok(artifacts)
}

fn walk(
    root: &Path,
    directory: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), WorkerError> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(directory)
        .map_err(|error| WorkerError(format!("read the artifact bundle: {error}")))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|error| WorkerError(format!("read the artifact bundle: {error}")))?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(root, &path, files)?;
        } else if path.is_file() {
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path)
                .map_err(|error| WorkerError(format!("read {name}: {error}")))?;
            files.push((name, bytes));
        }
    }
    Ok(())
}

fn computed_stages() -> Vec<StageProgress> {
    [
        StageName::Data,
        StageName::Model,
        StageName::Backtest,
        StageName::Evidence,
    ]
    .into_iter()
    .map(|name| StageProgress {
        name,
        state: if name == StageName::Evidence {
            StageStatus::Pending
        } else {
            StageStatus::Done
        },
        elapsed_ms: None,
    })
    .collect()
}

fn failed_stages() -> Vec<StageProgress> {
    [
        StageName::Data,
        StageName::Model,
        StageName::Backtest,
        StageName::Evidence,
    ]
    .into_iter()
    .map(|name| StageProgress {
        name,
        state: if name == StageName::Backtest {
            StageStatus::Failed
        } else {
            StageStatus::Pending
        },
        elapsed_ms: None,
    })
    .collect()
}
