//! Execute one claimed run and record its artifacts.
//!
//! A retryable failure holds the delivery and retries in place, so the broker's
//! redelivery flag is the only way a running run is reclaimed. The worker never
//! renders a label or accepts a client path.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunState, StageName, StageProgress, StageStatus,
};
use quantrule_app::service::AppService;
use quantrule_app::stages::backtest::BacktestRequest;

use crate::backtest::Workspace;
use crate::backtest::retry::{self, RetryDecision};
use crate::configs::ConfigCatalog;
use crate::data::DatasetCatalog;
use crate::evidence::RunManifest;
use crate::runs::{ClaimOutcome, RunStore};
use crate::runtime::adapters::{ObjectStore, Queue};
use crate::runtime::queue::{ROUTING_COMPUTED, ROUTING_DEAD, ROUTING_FAILED};

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
    /// The run store.
    pub runs: Arc<dyn RunStore>,
    /// The dataset catalog.
    pub catalog: Arc<dyn DatasetCatalog>,
    /// The config catalog.
    pub configs: Arc<dyn ConfigCatalog>,
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
        .runs
        .claim(run_id.clone(), redelivered)
        .await
        .map_err(|error| WorkerError(error.to_string()))?
    {
        ClaimOutcome::Claimed(record) => record,
        ClaimOutcome::Terminal(_) | ClaimOutcome::Missing => {
            return Ok(ProcessOutcome::Skipped);
        }
        ClaimOutcome::Running(record) => {
            // A delivery after the worker wrote its artifacts must not execute
            // again. The recorded digest is what tells the two `running` states
            // apart; publishing again closes a lost publish.
            if record.digest.is_some() && !record.artifacts.is_empty() {
                publish(&services.queue, ROUTING_COMPUTED, &run_id).await?;
            }
            return Ok(ProcessOutcome::Skipped);
        }
        ClaimOutcome::Exhausted(record) => {
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
    let dataset = match services
        .catalog
        .get(profile.clone(), record.data_version.to_string())
        .await
    {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return fail(services, &run_id, "the dataset is unknown").await,
        Err(error) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
    };
    let config = match services
        .configs
        .get(profile, record.config_id.clone())
        .await
    {
        Ok(Some(config)) => config,
        Ok(None) => return fail(services, &run_id, "the config is unknown").await,
        Err(error) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
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
    let bytes = match services.objects.get(&dataset.object_key).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return retry_or_exhaust(services, &run_id, &error.to_string(), attempts).await;
        }
    };
    if Digest::sha256(&bytes) != record.data_version {
        return fail(services, &run_id, "the dataset bytes do not match").await;
    }

    let workspace = match tokio::task::spawn_blocking(move || {
        Workspace::create(&config, &bytes, &record.data_version)
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
    if cancelled(services, &run_id).await? {
        return Ok(ProcessOutcome::Skipped);
    }
    let run_path = workspace.run_config();
    let strategy_path = workspace.strategy();
    let out = workspace.out();
    let outcome = tokio::task::spawn_blocking(move || {
        AppService::run_backtest(&BacktestRequest {
            config: &run_path,
            strategy: &strategy_path,
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
    let manifest = RunManifest::from_artifacts(&run_id, &artifacts);
    let manifest_bytes = manifest
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
        .runs
        .computed(
            run_id.clone(),
            manifest.digest.clone(),
            manifest.refs(),
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
        .runs
        .get_any(run_id.to_owned())
        .await
        .map_err(|error| WorkerError(error.to_string()))?
        .is_some_and(|record| record.status == RunState::Cancelled))
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
        .runs
        .fail(run_id.to_owned(), message.to_owned(), failed_stages())
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
        .runs
        .fail(run_id.to_owned(), message.to_owned(), failed_stages())
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
