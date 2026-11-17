//! The strategy stage executor.
//!
//! A strategy stage reads one succeeded model stage's content-addressed bundle
//! and one strategy configuration. It materializes both into a private
//! directory, runs the existing application use case, and returns only the
//! decisions bundle (`decisions.ndjson`, `summary.txt`, `provenance.json`).
//!
//! The executor does not run execution, portfolio, or a backtest. It reads no
//! fill, cash, equity, or P&L, and it cannot consult execution feedback: the
//! standalone strategy simulates its own position in the application use case.
//! That is why a strategy stage is diagnostic and never evidence.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::{Digest, ProfileId, StageKind, StageState};
use quantrule_app::stages::strategy::{StrategyRunError, StrategyRunRequest, run};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::{ClientError, DataClient, StageClient};
use quantrule_service_api::{StageManifest, StageRecord};

use crate::stage::executor::{StageExecutor, StageExecutorError, StageFile, StageOutput};

/// A counter that keeps two staging directories in one process distinct.
static NEXT: AtomicU64 = AtomicU64::new(1);

/// The strategy stage executor.
pub struct StrategyStageExecutor {
    evidence: Arc<dyn StageClient>,
    data: Arc<dyn DataClient>,
    objects: Arc<dyn ObjectStore>,
}

impl StrategyStageExecutor {
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

impl std::fmt::Debug for StrategyStageExecutor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StrategyStageExecutor")
            .finish_non_exhaustive()
    }
}

impl StageExecutor for StrategyStageExecutor {
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
    if record.kind != StageKind::Strategy {
        return Err(StageExecutorError(format!(
            "the strategy executor cannot run a {} stage",
            record.kind.as_str()
        )));
    }
    let artifact_id =
        record.input.model_artifact_id().cloned().ok_or_else(|| {
            StageExecutorError("a strategy stage needs a model artifact".to_owned())
        })?;
    let profile = ProfileId::parse(&record.owner).map_err(|error| {
        StageExecutorError(format!("the stage owner is not a profile: {error}"))
    })?;
    let upstream = evidence
        .get_stage_by_artifact(profile.clone(), StageKind::Model, artifact_id.clone())
        .await
        .map_err(client_error)?
        .ok_or_else(|| {
            StageExecutorError(format!(
                "no succeeded model stage publishes artifact {artifact_id} in this profile"
            ))
        })?;
    verify_upstream(record, &upstream, &artifact_id)?;

    let config = data
        .config_bundle(profile, record.config_id.clone())
        .await
        .map_err(client_error)?
        .ok_or_else(|| StageExecutorError(format!("config {} is unknown", record.config_id)))?;
    if !config.valid {
        return Err(StageExecutorError(
            config
                .message
                .unwrap_or_else(|| "the strategy config is invalid".to_owned()),
        ));
    }
    if config.config_hash.as_ref() != Some(&record.provenance.config_hash) {
        return Err(StageExecutorError(
            "the strategy config hash changed after submission".to_owned(),
        ));
    }
    let model_files = load_upstream_artifacts(objects, &upstream).await?;
    let config_files: Vec<(String, Vec<u8>)> = config
        .files
        .iter()
        .map(|(name, text)| (name.clone(), text.as_bytes().to_vec()))
        .collect();
    let strategy_relative = strategy_path(&config_files)?;

    let staging = Staging::new();
    let out = staging.path().join("out");
    let result = tokio::task::spawn_blocking(move || {
        write_files(&staging.path().join("model"), &model_files)?;
        write_files(&staging.path().join("config"), &config_files)?;
        let strategy = staging.path().join("config").join(&strategy_relative);
        if !strategy.is_file() {
            return Err(StageExecutorError(format!(
                "the strategy config bundle has no {}",
                strategy_relative.display()
            )));
        }
        let outcome = run(&StrategyRunRequest {
            model: &staging.path().join("model"),
            strategy: &strategy,
            out: Some(&out),
        })
        .map_err(strategy_error)?;
        let mut files = Vec::with_capacity(3);
        for name in ["decisions.ndjson", "summary.txt", "provenance.json"] {
            let path = outcome.root.join(name);
            let bytes = std::fs::read(&path).map_err(|error| {
                StageExecutorError(format!("cannot read the strategy artifact {name}: {error}"))
            })?;
            files.push(StageFile {
                name: name.to_owned(),
                bytes,
            });
        }
        Ok::<StageOutput, StageExecutorError>(StageOutput { files })
    })
    .await;
    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(error),
        Err(error) => Err(StageExecutorError(format!(
            "the strategy task failed: {error}"
        ))),
    }
}

/// Verify the upstream record is the succeeded model artifact the record names.
fn verify_upstream(
    record: &StageRecord,
    upstream: &StageRecord,
    artifact_id: &Digest,
) -> Result<(), StageExecutorError> {
    if upstream.kind != StageKind::Model {
        return Err(StageExecutorError(
            "the upstream stage is not a model stage".to_owned(),
        ));
    }
    if upstream.state != StageState::Succeeded {
        return Err(StageExecutorError(
            "the upstream model stage has not succeeded".to_owned(),
        ));
    }
    let artifact = upstream
        .artifact
        .as_ref()
        .ok_or_else(|| StageExecutorError("the upstream model stage has no artifact".to_owned()))?;
    if &artifact.artifact_id != artifact_id {
        return Err(StageExecutorError(
            "the upstream model stage publishes another artifact".to_owned(),
        ));
    }
    if upstream.provenance.data_version != record.provenance.data_version {
        return Err(StageExecutorError(
            "the upstream model stage belongs to another dataset".to_owned(),
        ));
    }
    if upstream.provenance.model_version != record.provenance.model_version {
        return Err(StageExecutorError(
            "the upstream model stage uses another model version".to_owned(),
        ));
    }
    Ok(())
}

/// Resolve the strategy file named by the run config, refusing an unsafe path.
fn strategy_path(files: &[(String, Vec<u8>)]) -> Result<PathBuf, StageExecutorError> {
    let run = files
        .iter()
        .find(|(name, _)| name == "run.json")
        .ok_or_else(|| StageExecutorError("the config bundle has no run.json".to_owned()))?;
    let value: serde_json::Value = serde_json::from_slice(&run.1)
        .map_err(|error| StageExecutorError(format!("run.json is not valid JSON: {error}")))?;
    let name = value
        .get("strategy")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            StageExecutorError("the run config does not name a strategy file".to_owned())
        })?;
    safe_segments(name)?;
    Ok(PathBuf::from(name))
}

/// Load the upstream stage manifest and every verified artifact blob.
async fn load_upstream_artifacts(
    objects: &Arc<dyn ObjectStore>,
    upstream: &StageRecord,
) -> Result<Vec<(String, Vec<u8>)>, StageExecutorError> {
    let manifest_key = StageManifest::key(&upstream.stage_run_id);
    let manifest_bytes = objects.get(&manifest_key).await.map_err(|error| {
        StageExecutorError(format!("the upstream model manifest is missing: {error}"))
    })?;
    let manifest = StageManifest::from_bytes(&manifest_bytes)
        .map_err(|error| StageExecutorError(error.to_string()))?;
    if manifest.stage_run_id != upstream.stage_run_id || manifest.kind != StageKind::Model {
        return Err(StageExecutorError(
            "the upstream model manifest belongs to another stage".to_owned(),
        ));
    }
    let artifact = upstream
        .artifact
        .as_ref()
        .ok_or_else(|| StageExecutorError("the upstream model stage has no artifact".to_owned()))?;
    if manifest.artifacts != artifact.files {
        return Err(StageExecutorError(
            "the upstream model manifest disagrees with the stage artifact".to_owned(),
        ));
    }
    let mut files = Vec::with_capacity(manifest.artifacts.len());
    for file in &manifest.artifacts {
        let bytes = objects
            .get(&StageManifest::blob_key(&file.sha256))
            .await
            .map_err(|error| {
                StageExecutorError(format!(
                    "the upstream model artifact {} is missing: {error}",
                    file.name
                ))
            })?;
        if bytes.len() as u64 != file.bytes || Digest::sha256(&bytes) != file.sha256 {
            return Err(StageExecutorError(format!(
                "the upstream model artifact {} does not match its manifest",
                file.name
            )));
        }
        files.push((file.name.clone(), bytes));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

/// Write relative files under a private root, refusing an escaping name.
fn write_files(root: &Path, files: &[(String, Vec<u8>)]) -> Result<(), StageExecutorError> {
    std::fs::create_dir_all(root).map_err(|error| {
        StageExecutorError(format!("cannot create {}: {error}", root.display()))
    })?;
    for (name, bytes) in files {
        let segments = safe_segments(name)?;
        let mut path = root.to_path_buf();
        for segment in &segments[..segments.len().saturating_sub(1)] {
            path.push(segment);
        }
        std::fs::create_dir_all(&path).map_err(|error| {
            StageExecutorError(format!("cannot create {}: {error}", path.display()))
        })?;
        let target = segments
            .iter()
            .fold(root.to_path_buf(), |path, segment| path.join(segment));
        std::fs::write(&target, bytes).map_err(|error| {
            StageExecutorError(format!("cannot write {}: {error}", target.display()))
        })?;
    }
    Ok(())
}

fn safe_segments(name: &str) -> Result<Vec<&str>, StageExecutorError> {
    if name.is_empty() || name.starts_with('/') || name.contains('\\') {
        return Err(StageExecutorError(format!(
            "the artifact name '{name}' is not a safe relative path"
        )));
    }
    let segments: Vec<&str> = name.split('/').collect();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err(StageExecutorError(format!(
            "the artifact name '{name}' is not a safe relative path"
        )));
    }
    Ok(segments)
}

fn strategy_error(error: StrategyRunError) -> StageExecutorError {
    match error {
        StrategyRunError::UnsupportedEngine { engine, reason } => {
            StageExecutorError(format!("unsupported strategy engine '{engine}': {reason}"))
        }
        other => StageExecutorError(other.to_string()),
    }
}

fn client_error(error: ClientError) -> StageExecutorError {
    StageExecutorError(error.to_string())
}

/// A private staging directory removed on drop.
struct Staging(PathBuf);

impl Staging {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "quantrule-strategy-stage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
