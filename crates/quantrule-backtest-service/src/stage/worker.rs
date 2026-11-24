//! Execute one claimed diagnostic stage.
//!
//! The stage worker claims the row through evidence, checks cancellation,
//! invokes the executor seam, writes content-addressed blobs and then the
//! manifest to the stage object namespace, and records the artifact through
//! evidence. It publishes `stage.computed`, `stage.failed`, or neither; it never
//! writes a run, a run artifact, a canonical digest, or a label.
//!
//! The manifest is the visible edge of the bundle. A failure before it is
//! written leaves no stage artifact, and a cancellation checked before the first
//! CAS write leaves the stage cancelled with no artifact. A repeated delivery
//! is a lookup: a succeeded stage republishes `stage.computed` and does not run
//! again.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use quantrule_api::{Digest, StageArtifact, StageRunId, StageState};
use quantrule_runtime::adapters::{ObjectStore, Queue};
use quantrule_runtime::client::{ClientError, StageClient};
use quantrule_runtime::queue::{ROUTING_STAGE_COMPUTED, ROUTING_STAGE_FAILED};
use quantrule_service_api::{StageClaimOutcome, StageHeartbeatOutcome, StageManifest, StageRecord};

use crate::backtest::retry::{self, RetryDecision};
use crate::stage::executor::{StageDispatch, StageFile};

/// A stage worker failure with a human-readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageWorkerError(pub String);

impl std::fmt::Display for StageWorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StageWorkerError {}

/// What one stage delivery did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageProcessOutcome {
    /// The artifact was recorded and `stage.computed` was published.
    Computed,
    /// The stage failed terminally on this attempt.
    Failed,
    /// A retryable failure; hold the delivery for the delay and try again.
    Retry {
        /// How long to wait before the next attempt.
        delay: Duration,
        /// The lease term this worker already holds.
        term: u64,
    },
    /// The retry budget is spent; the stage is failed.
    Exhausted,
    /// There was nothing to do: missing, running, terminal, or cancelled.
    Skipped,
}

/// The stores a stage worker needs.
pub struct StageWorkerServices {
    /// The evidence stage lifecycle.
    pub evidence: Arc<dyn StageClient>,
    /// The object store.
    pub objects: Arc<dyn ObjectStore>,
    /// The control-plane queue.
    pub queue: Arc<dyn Queue>,
    /// The worker identity recorded in the stage lease.
    pub holder: String,
}

/// Process one `stage.requested` stage run id.
///
/// `continuation` is the lease term this worker already holds when it retries
/// its own delivery. The lease, not the broker's redelivery flag, decides a
/// claim: a live lease is never stolen, and an expired one is reclaimable.
///
/// # Errors
///
/// Returns [`StageWorkerError`] when the evidence lifecycle or a message publish
/// is unavailable. An executor failure is recorded and returned as an outcome,
/// not an error, so the consumer acknowledges it.
pub async fn process_stage(
    stage_run_id: String,
    continuation: Option<u64>,
    redelivered: bool,
    services: &StageWorkerServices,
    dispatch: &StageDispatch,
) -> Result<StageProcessOutcome, StageWorkerError> {
    let record = match services
        .evidence
        .claim_stage(
            stage_run_id.clone(),
            services.holder.clone(),
            continuation,
            redelivered,
        )
        .await
        .map_err(stage_error)?
    {
        StageClaimOutcome::Claimed(record) => record,
        StageClaimOutcome::Terminal(record) => {
            if record.state == StageState::Succeeded && record.artifact.is_some() {
                // The row is authoritative; republishing closes a lost publish.
                publish(&services.queue, ROUTING_STAGE_COMPUTED, &stage_run_id).await?;
            }
            return Ok(StageProcessOutcome::Skipped);
        }
        StageClaimOutcome::Running(record) => {
            if record.artifact.is_some() {
                publish(&services.queue, ROUTING_STAGE_COMPUTED, &stage_run_id).await?;
            }
            return Ok(StageProcessOutcome::Skipped);
        }
        StageClaimOutcome::Exhausted(record) => {
            return exhaust(
                services,
                &stage_run_id,
                record.lease_term,
                "the retry budget was exhausted",
                record.attempts,
            )
            .await;
        }
        StageClaimOutcome::Missing => return Ok(StageProcessOutcome::Skipped),
    };
    let attempts = record.attempts;
    let term = record.lease_term;
    let lease = StageLeaseHeartbeat::start(
        Arc::clone(&services.evidence),
        &stage_run_id,
        &services.holder,
        term,
    );

    // A cancel before the engine call writes nothing. A lost lease stops this
    // attempt without removing anything: a later attempt may already own the
    // same content-addressed manifest.
    match stopped(services, &lease, &stage_run_id).await? {
        Cancellation::Gone | Cancellation::Cancelled => {
            remove_manifest(&services.objects, &record).await;
            return Ok(StageProcessOutcome::Skipped);
        }
        Cancellation::LeaseLost => return Ok(StageProcessOutcome::Skipped),
        Cancellation::Live => {}
    }

    let Some(executor) = dispatch.executor(record.kind) else {
        return fail(
            services,
            &stage_run_id,
            term,
            &format!(
                "no executor is configured for the {} stage",
                record.kind.as_str()
            ),
        )
        .await;
    };
    let output = match executor.execute(&record).await {
        Ok(output) => output,
        Err(error) => return fail(services, &stage_run_id, term, &error.to_string()).await,
    };

    // A cancel before the first CAS write must leave no manifest. A lost lease
    // only stops this attempt; the row and a later attempt own the manifest.
    match stopped(services, &lease, &stage_run_id).await? {
        Cancellation::Gone | Cancellation::Cancelled => {
            remove_manifest(&services.objects, &record).await;
            return Ok(StageProcessOutcome::Skipped);
        }
        Cancellation::LeaseLost => return Ok(StageProcessOutcome::Skipped),
        Cancellation::Live => {}
    }

    let artifact = match publish_artifact(&services.objects, &record, output.files).await {
        Ok(artifact) => artifact,
        Err(error) => {
            return retry_or_exhaust(services, &stage_run_id, term, &error.to_string(), attempts)
                .await;
        }
    };
    let stored = match services
        .evidence
        .stage_computed(stage_run_id.clone(), term, artifact)
        .await
    {
        Ok(record) => record,
        Err(error) => {
            return retry_or_exhaust(services, &stage_run_id, term, &error.to_string(), attempts)
                .await;
        }
    };
    if stored.state != StageState::Succeeded {
        // A stale term means a later attempt owns the stage; leave its
        // manifest alone. A cancel or competing terminal transition with our
        // own term won the race, so the visible edge is removed.
        if stored.lease_term != term {
            return Ok(StageProcessOutcome::Skipped);
        }
        remove_manifest(&services.objects, &record).await;
        return Ok(StageProcessOutcome::Skipped);
    }
    publish(&services.queue, ROUTING_STAGE_COMPUTED, &stage_run_id).await?;
    Ok(StageProcessOutcome::Computed)
}

/// A stage's claim lease, renewed from a worker task.
///
/// The guard aborts the heartbeat when the attempt returns. A heartbeat that
/// evidence rejects as stale sets `lost`, so the worker stops before its next
/// lifecycle write or manifest.
struct StageLeaseHeartbeat {
    handle: tokio::task::JoinHandle<()>,
    lost: Arc<std::sync::atomic::AtomicBool>,
}

impl StageLeaseHeartbeat {
    fn start(evidence: Arc<dyn StageClient>, stage_run_id: &str, holder: &str, term: u64) -> Self {
        let lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = Arc::clone(&lost);
        let stage_run_id = stage_run_id.to_owned();
        let holder = holder.to_owned();
        let handle = tokio::spawn(async move {
            let interval = Duration::from_secs(quantrule_service_api::STAGE_LEASE_RENEW_SECONDS);
            loop {
                tokio::time::sleep(interval).await;
                match evidence
                    .heartbeat_stage(stage_run_id.clone(), holder.clone(), term)
                    .await
                {
                    Ok(StageHeartbeatOutcome::Renewed(_)) => {}
                    Ok(StageHeartbeatOutcome::Stale | StageHeartbeatOutcome::Missing) => {
                        flag.store(true, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                    Err(error) => {
                        tracing::warn!(stage_run_id = %stage_run_id, %error, "the stage lease heartbeat will retry");
                    }
                }
            }
        });
        Self { handle, lost }
    }

    fn lost(&self) -> bool {
        self.lost.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for StageLeaseHeartbeat {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Whether a stage was cancelled, deleted, or lost its lease between two
/// worker boundaries.
enum Cancellation {
    /// The stage is still running under a live lease.
    Live,
    /// This attempt no longer owns the lease; another attempt may.
    LeaseLost,
    /// The operator cancelled it.
    Cancelled,
    /// Explicit cleanup deleted the row.
    Gone,
}

async fn stopped(
    services: &StageWorkerServices,
    lease: &StageLeaseHeartbeat,
    stage_run_id: &str,
) -> Result<Cancellation, StageWorkerError> {
    if lease.lost() {
        return Ok(Cancellation::LeaseLost);
    }
    let record = services
        .evidence
        .get_stage(stage_run_id.to_owned())
        .await
        .map_err(stage_error)?;
    Ok(match record {
        None => Cancellation::Gone,
        Some(record) if record.state == StageState::Cancelled => Cancellation::Cancelled,
        Some(_) => Cancellation::Live,
    })
}

async fn publish_artifact(
    objects: &Arc<dyn ObjectStore>,
    record: &StageRecord,
    files: Vec<StageFile>,
) -> Result<StageArtifact, StageWorkerError> {
    let files = normalize(files)?;
    let mut entries = Vec::with_capacity(files.len());
    let mut seen: BTreeMap<Digest, Vec<u8>> = BTreeMap::new();
    for file in files {
        let sha256 = Digest::sha256(&file.bytes);
        let bytes = file.bytes.len() as u64;
        entries.push(quantrule_api::ArtifactRef {
            name: file.name,
            bytes,
            sha256: sha256.clone(),
        });
        seen.entry(sha256).or_insert(file.bytes);
    }
    for (sha256, bytes) in seen {
        let key = StageManifest::blob_key(&sha256);
        let present = objects
            .exists(&key)
            .await
            .map_err(|error| StageWorkerError(error.to_string()))?;
        if !present {
            objects
                .put(&key, bytes)
                .await
                .map_err(|error| StageWorkerError(error.to_string()))?;
        }
    }
    let artifact =
        StageArtifact::from_files(entries).map_err(|error| StageWorkerError(error.to_string()))?;
    let manifest = StageManifest::new(
        record.stage_run_id.clone(),
        record.kind,
        record.config_id.clone(),
        &record.provenance,
        record.input.clone(),
        artifact.files.clone(),
    )
    .map_err(|error| StageWorkerError(error.to_string()))?;
    let bytes = manifest
        .to_bytes()
        .map_err(|error| StageWorkerError(error.to_string()))?;
    objects
        .put(&StageManifest::key(&record.stage_run_id), bytes)
        .await
        .map_err(|error| StageWorkerError(error.to_string()))?;
    Ok(artifact)
}

fn normalize(files: Vec<StageFile>) -> Result<Vec<StageFile>, StageWorkerError> {
    if files.is_empty() {
        return Err(StageWorkerError(
            "a stage executor produced no files".to_owned(),
        ));
    }
    let mut sorted = files;
    sorted.sort_by(|left, right| left.name.cmp(&right.name));
    if sorted.windows(2).any(|pair| pair[0].name == pair[1].name) {
        return Err(StageWorkerError(
            "a stage executor named a file twice".to_owned(),
        ));
    }
    if sorted.iter().any(|file| file.name.is_empty()) {
        return Err(StageWorkerError(
            "a stage executor produced an empty file name".to_owned(),
        ));
    }
    Ok(sorted)
}

async fn remove_manifest(objects: &Arc<dyn ObjectStore>, record: &StageRecord) {
    // Best effort: the row is the source of truth, and a failed delete only
    // leaves an unreferenced key that retention cannot expose.
    if let Err(error) = objects
        .delete(&StageManifest::key(&record.stage_run_id))
        .await
    {
        tracing::warn!(stage_run_id = %record.stage_run_id, %error, "the stage manifest was not removed");
    }
}

async fn remove_manifest_by_id(objects: &Arc<dyn ObjectStore>, stage_run_id: &str) {
    let Ok(id) = Digest::parse(stage_run_id) else {
        return;
    };
    if let Err(error) = objects.delete(&StageManifest::key(&StageRunId(id))).await {
        tracing::warn!(%stage_run_id, %error, "the stage manifest was not removed");
    }
}

async fn retry_or_exhaust(
    services: &StageWorkerServices,
    stage_run_id: &str,
    term: u64,
    message: &str,
    attempts: u32,
) -> Result<StageProcessOutcome, StageWorkerError> {
    match retry::decide(attempts) {
        RetryDecision::Retry(delay) => Ok(StageProcessOutcome::Retry { delay, term }),
        RetryDecision::DeadLetter => exhaust(services, stage_run_id, term, message, attempts).await,
    }
}

async fn exhaust(
    services: &StageWorkerServices,
    stage_run_id: &str,
    term: u64,
    message: &str,
    _attempts: u32,
) -> Result<StageProcessOutcome, StageWorkerError> {
    let record = services
        .evidence
        .stage_failed(stage_run_id.to_owned(), term, message.to_owned())
        .await
        .map_err(stage_error)?;
    if record.state != StageState::Failed {
        return Ok(StageProcessOutcome::Skipped);
    }
    remove_manifest_by_id(&services.objects, stage_run_id).await;
    publish(&services.queue, ROUTING_STAGE_FAILED, stage_run_id).await?;
    Ok(StageProcessOutcome::Exhausted)
}

async fn fail(
    services: &StageWorkerServices,
    stage_run_id: &str,
    term: u64,
    message: &str,
) -> Result<StageProcessOutcome, StageWorkerError> {
    let record = services
        .evidence
        .stage_failed(stage_run_id.to_owned(), term, message.to_owned())
        .await
        .map_err(stage_error)?;
    if record.state != StageState::Failed {
        return Ok(StageProcessOutcome::Skipped);
    }
    publish(&services.queue, ROUTING_STAGE_FAILED, stage_run_id).await?;
    Ok(StageProcessOutcome::Failed)
}

async fn publish(
    queue: &Arc<dyn Queue>,
    routing_key: &str,
    stage_run_id: &str,
) -> Result<(), StageWorkerError> {
    queue
        .publish(
            routing_key,
            serde_json::to_vec(&serde_json::json!({ "stage_run_id": stage_run_id }))
                .map_err(|error| StageWorkerError(error.to_string()))?,
        )
        .await
        .map_err(|error| StageWorkerError(error.to_string()))
}

fn stage_error(error: ClientError) -> StageWorkerError {
    StageWorkerError(error.to_string())
}
