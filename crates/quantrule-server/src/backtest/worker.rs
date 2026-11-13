//! Execute one claimed run and finalize it.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunState, StageName, StageProgress, StageStatus,
};
use quantrule_app::service::AppService;
use quantrule_app::stages::backtest::BacktestRequest;

use crate::backtest::Workspace;
use crate::configs::ConfigCatalog;
use crate::data::DatasetCatalog;
use crate::runs::{ClaimOutcome, RunStore};
use crate::runtime::adapters::{ObjectStore, Queue};

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
    /// The run completed and its artifacts were written.
    Succeeded,
    /// The run failed; its error was recorded.
    Failed,
    /// There was nothing to do: missing, running, or already terminal.
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
/// # Errors
///
/// Returns [`WorkerError`] when a store or the object store is unavailable. A
/// failed run is recorded and returned as [`ProcessOutcome::Failed`], not an
/// error, so the consumer can acknowledge it.
pub async fn process(
    run_id: String,
    services: &WorkerServices,
) -> Result<ProcessOutcome, WorkerError> {
    let Some(record) = services
        .runs
        .get_any(run_id.clone())
        .await
        .map_err(|error| WorkerError(error.to_string()))?
    else {
        return Ok(ProcessOutcome::Skipped);
    };
    match services
        .runs
        .claim(run_id.clone())
        .await
        .map_err(|error| WorkerError(error.to_string()))?
    {
        ClaimOutcome::Claimed(_) => {}
        ClaimOutcome::Terminal(record) => {
            republish_terminal(&services.queue, &record).await?;
            return Ok(ProcessOutcome::Skipped);
        }
        ClaimOutcome::Running(_) | ClaimOutcome::Missing => {
            return Ok(ProcessOutcome::Skipped);
        }
    }
    let profile = ProfileId::parse(&record.owner)
        .map_err(|error| WorkerError(format!("the run owner is invalid: {error}")))?;
    let Some(dataset) = services
        .catalog
        .get(profile.clone(), record.data_version.to_string())
        .await
        .map_err(|error| WorkerError(error.to_string()))?
    else {
        return fail(services, &run_id, "the dataset is unknown").await;
    };
    let Some(config) = services
        .configs
        .get(profile, record.config_id.clone())
        .await
        .map_err(|error| WorkerError(error.to_string()))?
    else {
        return fail(services, &run_id, "the config is unknown").await;
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
    let bytes = services
        .objects
        .get(&dataset.object_key)
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    if Digest::sha256(&bytes) != record.data_version {
        return fail(services, &run_id, "the dataset bytes do not match").await;
    }
    let workspace = tokio::task::spawn_blocking(move || {
        Workspace::create(&config, &bytes, &record.data_version)
    })
    .await
    .map_err(|error| WorkerError(error.to_string()))?
    .map_err(|error| WorkerError(error.to_string()))?;
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
    .await
    .map_err(|error| WorkerError(error.to_string()))?;
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => return fail(services, &run_id, &error.to_string()).await,
    };
    let Some(current) = services
        .runs
        .get_any(run_id.clone())
        .await
        .map_err(|error| WorkerError(error.to_string()))?
    else {
        return Ok(ProcessOutcome::Skipped);
    };
    if current.status == RunState::Cancelled {
        return Ok(ProcessOutcome::Skipped);
    }
    let artifacts = collect_artifacts(&services.objects, &run_id, &outcome.root).await?;
    let entries: Vec<(String, Digest)> = artifacts
        .iter()
        .filter(|artifact| canonical(&artifact.name))
        .map(|artifact| (artifact.name.clone(), artifact.sha256.clone()))
        .collect();
    let digest = Digest::canonical(&entries);
    let record = services
        .runs
        .complete(run_id.clone(), digest.clone(), artifacts, done_stages())
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    if record.status != RunState::Succeeded {
        return Ok(ProcessOutcome::Skipped);
    }
    services
        .queue
        .publish(
            "run.computed",
            serde_json::to_vec(&serde_json::json!({
                "run_id": run_id,
                "digest": digest.to_string(),
            }))
            .map_err(|error| WorkerError(error.to_string()))?,
        )
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    Ok(ProcessOutcome::Succeeded)
}

async fn fail(
    services: &WorkerServices,
    run_id: &str,
    message: &str,
) -> Result<ProcessOutcome, WorkerError> {
    services
        .runs
        .fail(run_id.to_owned(), message.to_owned(), failed_stages())
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    services
        .queue
        .publish(
            "run.failed",
            serde_json::to_vec(&serde_json::json!({
                "run_id": run_id,
                "error": message,
            }))
            .map_err(|error| WorkerError(error.to_string()))?,
        )
        .await
        .map_err(|error| WorkerError(error.to_string()))?;
    Ok(ProcessOutcome::Failed)
}

async fn republish_terminal(
    queue: &Arc<dyn Queue>,
    record: &crate::runs::RunRecord,
) -> Result<(), WorkerError> {
    match record.status {
        RunState::Succeeded => {
            if let Some(digest) = &record.digest {
                queue
                    .publish(
                        "run.computed",
                        serde_json::to_vec(&serde_json::json!({
                            "run_id": record.run_id.to_string(),
                            "digest": digest.to_string(),
                        }))
                        .map_err(|error| WorkerError(error.to_string()))?,
                    )
                    .await
                    .map_err(|error| WorkerError(error.to_string()))?;
            }
        }
        RunState::Failed => {
            if let Some(error) = &record.error {
                queue
                    .publish(
                        "run.failed",
                        serde_json::to_vec(&serde_json::json!({
                            "run_id": record.run_id.to_string(),
                            "error": error,
                        }))
                        .map_err(|error| WorkerError(error.to_string()))?,
                    )
                    .await
                    .map_err(|error| WorkerError(error.to_string()))?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn collect_artifacts(
    objects: &Arc<dyn ObjectStore>,
    run_id: &str,
    root: &Path,
) -> Result<Vec<ArtifactRef>, WorkerError> {
    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut artifacts = Vec::with_capacity(files.len());
    for (name, bytes) in files {
        let key = format!("runs/{run_id}/{name}");
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
            sha256: Digest::sha256(&bytes),
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

fn canonical(name: &str) -> bool {
    name == "results.json" || name.ends_with(".csv") || name.ends_with(".ndjson")
}

fn done_stages() -> Vec<StageProgress> {
    [
        StageName::Data,
        StageName::Model,
        StageName::Backtest,
        StageName::Evidence,
    ]
    .into_iter()
    .map(|name| StageProgress {
        name,
        state: StageStatus::Done,
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
