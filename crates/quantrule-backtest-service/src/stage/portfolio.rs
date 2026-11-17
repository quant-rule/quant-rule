//! The portfolio stage executor.
//!
//! A portfolio stage reads one succeeded strategy stage's decisions bundle, the
//! run configuration, and the catalogued dataset. It materializes them into a
//! private workspace, runs the existing execution and portfolio use cases, and
//! returns the two published bundles under `execution/` and `portfolio/`.
//!
//! The executor does not run the strategy engine and does not decide a signal.
//! The decisions artifact is the frozen handoff; the portfolio stage replays
//! execution fills into accounting and publishes no canonical digest, run row,
//! or evidence record.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::{Digest, ProfileId, StageKind, StageState};
use quantrule_app::stages::execution::{ExecutionRequest, ExecutionRunError, run as run_execution};
use quantrule_app::stages::portfolio::{
    PortfolioRequest, PortfolioRunError, replay as run_portfolio,
};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::{ClientError, DataClient, StageClient};
use quantrule_service_api::{StageManifest, StageRecord};
use serde_json::Value;

use crate::backtest::workspace::{safe_key, source_csv};
use crate::stage::executor::{StageExecutor, StageExecutorError, StageFile, StageOutput};

/// The decisions files a portfolio stage consumes.
const DECISIONS_FILES: [&str; 3] = ["decisions.ndjson", "provenance.json", "summary.txt"];

/// A counter that keeps two staging directories in one process distinct.
static NEXT: AtomicU64 = AtomicU64::new(1);

/// The portfolio stage executor.
pub struct PortfolioStageExecutor {
    evidence: Arc<dyn StageClient>,
    data: Arc<dyn DataClient>,
    objects: Arc<dyn ObjectStore>,
}

impl PortfolioStageExecutor {
    /// Build the executor over the evidence, data, and object-store seams.
    #[must_use]
    pub fn new(
        evidence: Arc<dyn StageClient>,
        data: Arc<dyn DataClient>,
        objects: Arc<dyn ObjectStore>,
    ) -> Self {
        Self {
            evidence,
            data,
            objects,
        }
    }
}

impl std::fmt::Debug for PortfolioStageExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortfolioStageExecutor")
            .finish_non_exhaustive()
    }
}

impl StageExecutor for PortfolioStageExecutor {
    fn execute(&self, record: &StageRecord) -> BoxFuture<Result<StageOutput, StageExecutorError>> {
        let evidence = Arc::clone(&self.evidence);
        let data = Arc::clone(&self.data);
        let objects = Arc::clone(&self.objects);
        let record = record.clone();
        Box::pin(async move { execute(&evidence, &data, &objects, &record).await })
    }
}

async fn execute(
    evidence: &Arc<dyn StageClient>,
    data: &Arc<dyn DataClient>,
    objects: &Arc<dyn ObjectStore>,
    record: &StageRecord,
) -> Result<StageOutput, StageExecutorError> {
    if record.kind != StageKind::Portfolio {
        return Err(StageExecutorError(format!(
            "the portfolio executor cannot run a {} stage",
            record.kind.as_str()
        )));
    }
    let artifact_id = record
        .input
        .decisions_artifact_id()
        .cloned()
        .ok_or_else(|| {
            StageExecutorError("a portfolio stage needs a decisions artifact".to_owned())
        })?;
    let profile = ProfileId::parse(&record.owner).map_err(|error| {
        StageExecutorError(format!("the stage owner is not a profile: {error}"))
    })?;
    let upstream = evidence
        .get_stage_by_artifact(profile.clone(), StageKind::Strategy, artifact_id.clone())
        .await
        .map_err(client_error)?
        .ok_or_else(|| {
            StageExecutorError(format!(
                "no succeeded strategy stage publishes artifact {artifact_id} in this profile"
            ))
        })?;
    verify_upstream(record, &upstream, &artifact_id)?;

    let config = data
        .config_bundle(profile.clone(), record.config_id.clone())
        .await
        .map_err(client_error)?
        .ok_or_else(|| StageExecutorError(format!("config {} is unknown", record.config_id)))?;
    if !config.valid {
        return Err(StageExecutorError(
            config
                .message
                .unwrap_or_else(|| "the portfolio config is invalid".to_owned()),
        ));
    }
    if config.config_hash.as_ref() != Some(&record.provenance.config_hash) {
        return Err(StageExecutorError(
            "the portfolio config hash changed after submission".to_owned(),
        ));
    }
    let dataset = data
        .dataset_object(profile, record.provenance.data_version.to_string())
        .await
        .map_err(client_error)?
        .ok_or_else(|| {
            StageExecutorError(format!(
                "dataset {} is unknown",
                record.provenance.data_version
            ))
        })?;
    let dataset_bytes = objects
        .get(&dataset.object_key)
        .await
        .map_err(|error| StageExecutorError(format!("the dataset object is missing: {error}")))?;
    if dataset.data_version != record.provenance.data_version
        || Digest::sha256(&dataset_bytes) != dataset.sha256
        || dataset_bytes.len() as u64 != dataset.bytes
    {
        return Err(StageExecutorError(
            "the dataset bytes do not match the catalogued record".to_owned(),
        ));
    }

    let decisions_files = load_upstream_artifacts(objects, &upstream).await?;
    let config_files: Vec<(String, Vec<u8>)> = config
        .files
        .iter()
        .map(|(name, text)| (name.clone(), text.as_bytes().to_vec()))
        .collect();

    let result = tokio::task::spawn_blocking(move || {
        let workspace = Workspace::create(&config_files, &dataset_bytes, &decisions_files)?;
        let execution = run_execution(&ExecutionRequest {
            config: &workspace.run,
            decisions: &workspace.decisions,
            out: Some(&workspace.out),
        })
        .map_err(execution_error)?;
        let portfolio = run_portfolio(&PortfolioRequest {
            config: &workspace.run,
            fills: &execution.root,
            out: Some(&workspace.out),
        })
        .map_err(portfolio_error)?;
        let mut files = Vec::with_capacity(execution.artifacts.len() + portfolio.artifacts.len());
        for name in execution.artifacts {
            files.push(stage_file(&execution.root, "execution", &name)?);
        }
        for name in portfolio.artifacts {
            files.push(stage_file(&portfolio.root, "portfolio", &name)?);
        }
        files.sort_by(|left, right| left.name.cmp(&right.name));
        Ok::<StageOutput, StageExecutorError>(StageOutput { files })
    })
    .await;
    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(error),
        Err(error) => Err(StageExecutorError(format!(
            "the portfolio task failed: {error}"
        ))),
    }
}

/// Verify the upstream record is the succeeded strategy artifact the record names.
fn verify_upstream(
    record: &StageRecord,
    upstream: &StageRecord,
    artifact_id: &Digest,
) -> Result<(), StageExecutorError> {
    if upstream.kind != StageKind::Strategy {
        return Err(StageExecutorError(
            "the upstream stage is not a strategy stage".to_owned(),
        ));
    }
    if upstream.state != StageState::Succeeded {
        return Err(StageExecutorError(
            "the upstream strategy stage has not succeeded".to_owned(),
        ));
    }
    let artifact = upstream.artifact.as_ref().ok_or_else(|| {
        StageExecutorError("the upstream strategy stage has no artifact".to_owned())
    })?;
    if &artifact.artifact_id != artifact_id {
        return Err(StageExecutorError(
            "the upstream strategy stage publishes another artifact".to_owned(),
        ));
    }
    if upstream.provenance.data_version != record.provenance.data_version {
        return Err(StageExecutorError(
            "the upstream strategy stage belongs to another dataset".to_owned(),
        ));
    }
    if upstream.provenance.model_version != record.provenance.model_version {
        return Err(StageExecutorError(
            "the upstream strategy stage uses another model version".to_owned(),
        ));
    }
    Ok(())
}

/// Load and verify the upstream strategy stage manifest and every artifact.
async fn load_upstream_artifacts(
    objects: &Arc<dyn ObjectStore>,
    upstream: &StageRecord,
) -> Result<Vec<(String, Vec<u8>)>, StageExecutorError> {
    let manifest_key = StageManifest::key(&upstream.stage_run_id);
    let manifest_bytes = objects.get(&manifest_key).await.map_err(|error| {
        StageExecutorError(format!(
            "the upstream strategy manifest is missing: {error}"
        ))
    })?;
    let manifest = StageManifest::from_bytes(&manifest_bytes)
        .map_err(|error| StageExecutorError(error.to_string()))?;
    if manifest.stage_run_id != upstream.stage_run_id || manifest.kind != StageKind::Strategy {
        return Err(StageExecutorError(
            "the upstream strategy manifest belongs to another stage".to_owned(),
        ));
    }
    let artifact = upstream.artifact.as_ref().ok_or_else(|| {
        StageExecutorError("the upstream strategy stage has no artifact".to_owned())
    })?;
    if manifest.artifacts != artifact.files {
        return Err(StageExecutorError(
            "the upstream strategy manifest disagrees with the stage artifact".to_owned(),
        ));
    }
    let mut files = Vec::with_capacity(manifest.artifacts.len());
    for file in &manifest.artifacts {
        let bytes = objects
            .get(&StageManifest::blob_key(&file.sha256))
            .await
            .map_err(|error| {
                StageExecutorError(format!(
                    "the upstream strategy artifact {} is missing: {error}",
                    file.name
                ))
            })?;
        if bytes.len() as u64 != file.bytes || Digest::sha256(&bytes) != file.sha256 {
            return Err(StageExecutorError(format!(
                "the upstream strategy artifact {} does not match its manifest",
                file.name
            )));
        }
        files.push((file.name.clone(), bytes));
    }
    for required in DECISIONS_FILES {
        if !files.iter().any(|(name, _)| name == required) {
            return Err(StageExecutorError(format!(
                "the upstream strategy artifact has no {required}"
            )));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn stage_file(root: &Path, prefix: &str, name: &str) -> Result<StageFile, StageExecutorError> {
    let path = root.join(name);
    let bytes = std::fs::read(&path).map_err(|error| {
        StageExecutorError(format!("cannot read the {prefix} artifact {name}: {error}"))
    })?;
    Ok(StageFile {
        name: format!("{prefix}/{name}"),
        bytes,
    })
}

fn execution_error(error: ExecutionRunError) -> StageExecutorError {
    StageExecutorError(error.to_string())
}

fn portfolio_error(error: PortfolioRunError) -> StageExecutorError {
    StageExecutorError(error.to_string())
}

fn client_error(error: ClientError) -> StageExecutorError {
    StageExecutorError(error.to_string())
}

/// A private workspace holding the config, candles, and decisions bundle.
struct Workspace {
    root: PathBuf,
    run: PathBuf,
    decisions: PathBuf,
    out: PathBuf,
}

impl Workspace {
    fn create(
        files: &[(String, Vec<u8>)],
        dataset: &[u8],
        decisions: &[(String, Vec<u8>)],
    ) -> Result<Self, StageExecutorError> {
        let run_text = files
            .iter()
            .find(|(name, _)| name == "run.json")
            .map(|(_, bytes)| bytes)
            .ok_or_else(|| StageExecutorError("the config bundle has no run.json".to_owned()))?;
        let run: Value = serde_json::from_slice(run_text)
            .map_err(|error| StageExecutorError(format!("run.json is not JSON: {error}")))?;
        let data_key = run
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| StageExecutorError("run config names no data file".to_owned()))?;
        let strategy_key = run.get("strategy").and_then(Value::as_str);

        for (name, _) in files {
            safe_key(name).map_err(workspace_error)?;
        }
        safe_key(data_key).map_err(workspace_error)?;
        if let Some(strategy_key) = strategy_key {
            safe_key(strategy_key).map_err(workspace_error)?;
        }
        for (name, _) in decisions {
            safe_key(name).map_err(workspace_error)?;
        }

        let root = std::env::temp_dir().join(format!(
            "quantrule-portfolio-stage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let workspace = Self {
            run: root.join("run.json"),
            decisions: root.join("decisions"),
            out: root.join("out"),
            root,
        };
        std::fs::create_dir_all(&workspace.root).map_err(workspace_io)?;
        write_files(&workspace.root, files)?;
        write_files(&workspace.decisions, decisions)?;

        let data_text = files
            .iter()
            .find(|(name, _)| name == data_key)
            .map(|(_, bytes)| bytes)
            .ok_or_else(|| StageExecutorError(format!("the config bundle has no {data_key}")))?;
        let mut data: Value = serde_json::from_slice(data_text)
            .map_err(|error| StageExecutorError(format!("{data_key} is not JSON: {error}")))?;
        let source = data
            .get_mut("source")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| StageExecutorError(format!("{data_key} has no source object")))?;
        source.insert("path".to_owned(), Value::String("candles.csv".to_owned()));
        let data_dir = Path::new(data_key)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let candles = workspace.root.join(&data_dir).join("candles.csv");
        if let Some(parent) = candles.parent() {
            std::fs::create_dir_all(parent).map_err(workspace_io)?;
        }
        std::fs::write(&candles, source_csv(dataset).map_err(workspace_error)?)
            .map_err(workspace_io)?;
        std::fs::write(
            workspace.root.join(data_key),
            serde_json::to_vec(&data).map_err(|error| {
                StageExecutorError(format!("cannot encode {data_key}: {error}"))
            })?,
        )
        .map_err(workspace_io)?;

        if let Some(strategy_key) = strategy_key
            && !workspace.root.join(strategy_key).is_file()
        {
            return Err(StageExecutorError(format!(
                "the config bundle has no {strategy_key}"
            )));
        }
        if !workspace.run.is_file() {
            return Err(StageExecutorError(
                "the config bundle has no run.json".to_owned(),
            ));
        }
        Ok(workspace)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn write_files(root: &Path, files: &[(String, Vec<u8>)]) -> Result<(), StageExecutorError> {
    std::fs::create_dir_all(root).map_err(workspace_io)?;
    for (name, bytes) in files {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(workspace_io)?;
        }
        std::fs::write(&path, bytes).map_err(workspace_io)?;
    }
    Ok(())
}

fn workspace_error(error: crate::backtest::WorkerError) -> StageExecutorError {
    StageExecutorError(error.to_string())
}

fn workspace_io(error: std::io::Error) -> StageExecutorError {
    StageExecutorError(format!("cannot prepare the portfolio workspace: {error}"))
}
