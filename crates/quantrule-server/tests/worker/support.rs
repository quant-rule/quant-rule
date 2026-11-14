//! Shared fixtures and fakes for the backtest worker tests.

#[path = "../common/mod.rs"]
pub mod common;

use common::canonical;

// A single shared vocabulary for every worker test module.
#[allow(unused_imports)]
pub use quantrule_api::{ArtifactRef, Digest, ProfileId, RunState};
#[allow(unused_imports)]
pub use quantrule_runtime::adapters::{
    Backend, BoxFuture, ObjectStore, Queue, QueueError, StoreError,
};
#[allow(unused_imports)]
pub use quantrule_runtime::client::{
    ClientError, DataClient, EvidenceClient, MemoryDataClient, MemoryEvidence, MemoryModel,
    ModelClient, Service,
};
#[allow(unused_imports)]
pub use quantrule_server::backtest::{ProcessOutcome, WorkerServices, process};
#[allow(unused_imports)]
pub use quantrule_service_api::{
    ConfigBundle, DatasetObject, ModelBundleManifest, NewRun, RunManifest, RunRecord,
    derive_run_id as derive,
};
pub use std::collections::BTreeMap;
pub use std::path::Path;
pub use std::sync::{Arc, Mutex};
pub use std::time::Duration;

type Messages = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone, Default)]
pub struct MemoryQueue {
    messages: Messages,
}

impl Queue for MemoryQueue {
    fn backend(&self) -> Backend {
        Backend::InProcess
    }

    fn publish(&self, key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        let messages = Arc::clone(&self.messages);
        let key = key.to_owned();
        Box::pin(async move {
            messages
                .lock()
                .map_err(|_| QueueError("the queue lock is poisoned".to_owned()))?
                .push((key, payload));
            Ok(())
        })
    }
}

impl MemoryQueue {
    pub fn keys(&self) -> Vec<String> {
        self.messages
            .lock()
            .expect("queue")
            .iter()
            .map(|(key, _)| key.clone())
            .collect()
    }

    pub fn payloads(&self, key: &str) -> Vec<Vec<u8>> {
        self.messages
            .lock()
            .expect("queue")
            .iter()
            .filter(|(name, _)| name == key)
            .map(|(_, payload)| payload.clone())
            .collect()
    }
}

pub fn csv() -> Vec<u8> {
    let mut text = String::new();
    for offset in 0..250 {
        let hour = offset / 60;
        let minute = offset % 60;
        let open = 100.0 + offset as f64 * 0.01;
        text.push_str(&format!(
            "2024-01-01 {hour:02}:{minute:02}:00+00:00,{open},{},{},{},10\n",
            open + 0.5,
            open - 0.5,
            open + 0.25
        ));
    }
    canonical(&text)
}

pub fn bundle(root: &Path) -> ConfigBundle {
    let mut files = BTreeMap::new();
    for name in [
        "run.json",
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
        "strategy.json",
    ] {
        let path = root.join(name);
        if path.is_file() {
            files.insert(
                name.to_owned(),
                std::fs::read_to_string(path).expect("config file"),
            );
        }
    }
    ConfigBundle {
        config_id: "cfg_valid_v4".to_owned(),
        files,
        valid: true,
        message: None,
        config_hash: Some(Digest::sha256(b"valid-v4")),
    }
}

pub struct Fixture {
    pub services: WorkerServices,
    pub runs: MemoryEvidence,
    pub queue: MemoryQueue,
    pub objects: common::MemoryObjects,
    pub model: MemoryModel,
    pub config_id: String,
    pub invalid_config_id: String,
    pub data_version: Digest,
    pub config_hash: Digest,
    pub run_id: String,
    pub baseline: Vec<(String, Vec<u8>)>,
    pub baseline_digest: Digest,
    pub manifest: ModelBundleManifest,
}

static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// A private test directory that removes itself.
pub struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        Self(std::env::temp_dir().join(format!(
            "quantrule-worker-test-{label}-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every file under a bundle root, sorted by name.
pub fn read_bundle(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, directory: &Path, files: &mut Vec<(String, Vec<u8>)>) {
        let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(directory)
            .expect("the bundle directory")
            .map(|entry| entry.expect("a bundle entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(root, &path, files);
            } else if path.is_file() {
                let name = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((name, std::fs::read(&path).expect("the artifact bytes")));
            }
        }
    }
    let mut files = Vec::new();
    walk(root, root, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

pub fn refs(files: &[(String, Vec<u8>)]) -> Vec<ArtifactRef> {
    files
        .iter()
        .map(|(name, bytes)| ArtifactRef {
            name: name.clone(),
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(bytes),
        })
        .collect()
}

/// The coupled `backtest run` baseline over a materialized config and dataset.
pub fn coupled_baseline(config: &ConfigBundle, dataset: &[u8]) -> (Vec<(String, Vec<u8>)>, Digest) {
    let workspace = quantrule_server::backtest::Workspace::create(config, dataset)
        .expect("the baseline workspace");
    let out = TempDir::new("baseline");
    let outcome = quantrule_app::service::AppService::run_backtest(
        &quantrule_app::stages::backtest::BacktestRequest {
            config: &workspace.run_config(),
            strategy: &workspace.strategy(),
            out: Some(out.path()),
        },
    )
    .expect("the coupled baseline");
    let files = read_bundle(&outcome.root);
    let digest = quantrule_service_api::canonical_digest(&refs(&files));
    (files, digest)
}

/// Build the model bundle the model service would publish, from the same
/// materialized config and candles, and register it with the fake.
pub async fn model_bundle(
    config: &ConfigBundle,
    dataset: &[u8],
    data_version: &Digest,
    profile: &ProfileId,
    objects: &common::MemoryObjects,
    model: &MemoryModel,
) -> (Vec<(String, Vec<u8>)>, ModelBundleManifest) {
    let workspace = quantrule_server::backtest::Workspace::create(config, dataset)
        .expect("the model workspace");
    let run_config =
        quantrule_app::config::RunConfig::load(&workspace.run_config()).expect("run config");
    let out = TempDir::new("model-bundle");
    let run = quantrule_app::pipeline::run_model(&run_config).expect("the model run");
    let built =
        quantrule_app::artifact::model_bundle::model_bundle_from(&run_config, out.path(), &run)
            .expect("the model bundle");
    let mut artifacts = Vec::with_capacity(built.names.len());
    for name in &built.names {
        let bytes = std::fs::read(built.path.join(name)).expect("the model artifact");
        objects
            .put(&format!("model/bundle/{name}"), bytes.clone())
            .await
            .expect("the model object");
        artifacts.push((name.clone(), bytes));
    }
    artifacts.sort_by(|left, right| left.0.cmp(&right.0));
    let manifest = ModelBundleManifest {
        schema: quantrule_service_api::MODEL_BUNDLE_SCHEMA.to_owned(),
        data_version: data_version.clone(),
        config_id: config.config_id.clone(),
        config_hash: config.config_hash.clone().expect("the config hash"),
        model_version: "M1.0".to_owned(),
        ladder: vec![
            "1m".to_owned(),
            "5m".to_owned(),
            "15m".to_owned(),
            "1h".to_owned(),
        ],
        manifest_key: "model/bundle/manifest.json".to_owned(),
        artifacts: refs(&artifacts),
    };
    model.add_bundle(profile, manifest.clone());
    (artifacts, manifest)
}

pub async fn fixture() -> Fixture {
    let profile = ProfileId::parse("default").expect("profile");
    let objects = common::MemoryObjects::default();
    let data = MemoryDataClient::default();
    let runs = MemoryEvidence::default();
    let queue = MemoryQueue::default();
    let model = MemoryModel::new();

    let dataset = csv();
    let data_version = Digest::sha256(&dataset);
    let key = format!("datasets/{}", data_version.as_hex());
    objects.put(&key, dataset.clone()).await.expect("put");
    data.add_dataset(
        &profile,
        DatasetObject {
            data_version: data_version.clone(),
            object_key: key,
            sha256: data_version.clone(),
            candles: 250,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_200 + 249 * 60,
            bytes: dataset.len() as u64,
        },
    );

    let valid =
        bundle(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid-v4"));
    let invalid = ConfigBundle {
        config_id: "cfg_invalid".to_owned(),
        files: BTreeMap::from([(
            "run.json".to_owned(),
            std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/config/invalid/run.json"),
            )
            .expect("invalid run"),
        )]),
        valid: false,
        message: Some("the bundle has no data.json".to_owned()),
        config_hash: None,
    };
    data.add_config(&profile, valid.clone());
    data.add_config(&profile, invalid.clone());

    let (baseline, baseline_digest) = coupled_baseline(&valid, &dataset);
    let (_, manifest) =
        model_bundle(&valid, &dataset, &data_version, &profile, &objects, &model).await;

    let config_hash = valid.config_hash.clone().expect("hash");
    let image = Digest::sha256(b"image");
    let run_id = derive(&data_version, &config_hash, "M1.0", &image);
    let new_run = NewRun {
        run_id,
        config_id: valid.config_id.clone(),
        data_version: data_version.clone(),
        config_hash: config_hash.clone(),
        model_version: "M1.0".to_owned(),
        image_digest: image,
        owner: "default".to_owned(),
        submitted_at: "2026-11-13T00:00:00Z".to_owned(),
    };
    let submitted = runs.submit(&profile, new_run).await.expect("submit");
    let run_id = submitted.run_id.to_string();

    let services = WorkerServices {
        evidence: Arc::new(runs.clone()),
        data: Arc::new(data),
        model: Arc::new(model.clone()),
        objects: Arc::new(objects.clone()),
        queue: Arc::new(queue.clone()),
    };
    Fixture {
        services,
        runs,
        queue,
        objects,
        model,
        config_id: valid.config_id,
        invalid_config_id: invalid.config_id,
        data_version,
        config_hash,
        run_id,
        baseline,
        baseline_digest,
        manifest,
    }
}

#[derive(Clone)]
pub struct PutFails {
    inner: common::MemoryObjects,
}

impl ObjectStore for PutFails {
    fn backend(&self) -> Backend {
        Backend::Filesystem
    }

    fn put(&self, _key: &str, _bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        Box::pin(async { Err(StoreError("the artifact store is full".to_owned())) })
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        self.inner.get(key)
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        self.inner.exists(key)
    }
}

#[derive(Clone)]
pub struct ManifestFails {
    inner: common::MemoryObjects,
}

impl ObjectStore for ManifestFails {
    fn backend(&self) -> Backend {
        Backend::Filesystem
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        if key.ends_with("/manifest.json") {
            return Box::pin(async {
                Err(StoreError(
                    "the worker was killed before publish".to_owned(),
                ))
            });
        }
        self.inner.put(key, bytes)
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        self.inner.get(key)
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        self.inner.exists(key)
    }
}

pub fn services_with_manifest_failure(fixture: &Fixture) -> WorkerServices {
    WorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        data: Arc::clone(&fixture.services.data),
        model: Arc::clone(&fixture.services.model),
        objects: Arc::new(ManifestFails {
            inner: fixture.objects.clone(),
        }),
        queue: Arc::clone(&fixture.services.queue),
    }
}

pub fn services_with_put_failure(fixture: &Fixture) -> WorkerServices {
    WorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        data: Arc::clone(&fixture.services.data),
        model: Arc::clone(&fixture.services.model),
        objects: Arc::new(PutFails {
            inner: fixture.objects.clone(),
        }),
        queue: Arc::clone(&fixture.services.queue),
    }
}

pub async fn submit_with(
    runs: &MemoryEvidence,
    data_version: Digest,
    config_id: String,
    config_hash: Digest,
    suffix: &str,
) -> String {
    let profile = ProfileId::parse("default").expect("profile");
    let image = Digest::sha256(suffix.as_bytes());
    let run_id = derive(&data_version, &config_hash, "M1.0", &image);
    runs.submit(
        &profile,
        NewRun {
            run_id,
            config_id,
            data_version,
            config_hash,
            model_version: "M1.0".to_owned(),
            image_digest: image,
            owner: "default".to_owned(),
            submitted_at: "2026-11-13T00:00:00Z".to_owned(),
        },
    )
    .await
    .expect("submit")
    .run_id
    .to_string()
}

#[derive(Clone)]
pub struct StopAfter {
    inner: common::MemoryObjects,
    allowed: usize,
    puts: Arc<Mutex<usize>>,
}

impl ObjectStore for StopAfter {
    fn backend(&self) -> Backend {
        Backend::Filesystem
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        let inner = self.inner.clone();
        let puts = Arc::clone(&self.puts);
        let allowed = self.allowed;
        let key = key.to_owned();
        Box::pin(async move {
            let accepted = {
                let mut count = puts
                    .lock()
                    .map_err(|_| StoreError("the counter lock is poisoned".to_owned()))?;
                if *count < allowed {
                    *count += 1;
                    true
                } else {
                    false
                }
            };
            if !accepted {
                return Err(StoreError("the worker was stopped".to_owned()));
            }
            inner.put(&key, bytes).await
        })
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        self.inner.get(key)
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        self.inner.exists(key)
    }
}

pub fn services_with_stop_after(fixture: &Fixture, allowed: usize) -> WorkerServices {
    WorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        data: Arc::clone(&fixture.services.data),
        model: Arc::clone(&fixture.services.model),
        objects: Arc::new(StopAfter {
            inner: fixture.objects.clone(),
            allowed,
            puts: Arc::new(Mutex::new(0)),
        }),
        queue: Arc::clone(&fixture.services.queue),
    }
}

#[derive(Clone)]
pub struct FailingData {
    pub inner: MemoryDataClient,
    pub fail_config: bool,
    pub fail_dataset: bool,
}

impl DataClient for FailingData {
    fn config_bundle(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<quantrule_service_api::ConfigBundle>, ClientError>> {
        if self.fail_config {
            return Box::pin(async {
                Err(ClientError::Unreachable {
                    service: Service::Data,
                    message: "data is down".to_owned(),
                })
            });
        }
        self.inner.config_bundle(profile, config_id)
    }

    fn dataset_object(
        &self,
        profile: ProfileId,
        data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>> {
        if self.fail_dataset {
            return Box::pin(async {
                Err(ClientError::Unreachable {
                    service: Service::Data,
                    message: "data is down".to_owned(),
                })
            });
        }
        self.inner.dataset_object(profile, data_version)
    }

    fn dataset_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        self.inner.dataset_count(profile)
    }
}

/// A model client that answers one fixed transport or status error.
#[derive(Clone)]
pub struct FailingModel {
    pub error: ClientError,
}

/// A model client that returns one fixed manifest for any request.
#[derive(Clone)]
pub struct FixedModel {
    pub manifest: ModelBundleManifest,
}

impl ModelClient for FixedModel {
    fn bundle(
        &self,
        _profile: ProfileId,
        _data_version: Digest,
        _config_id: String,
    ) -> BoxFuture<Result<ModelBundleManifest, ClientError>> {
        let manifest = self.manifest.clone();
        Box::pin(async move { Ok(manifest) })
    }
}

impl ModelClient for FailingModel {
    fn bundle(
        &self,
        _profile: ProfileId,
        _data_version: Digest,
        _config_id: String,
    ) -> BoxFuture<Result<ModelBundleManifest, ClientError>> {
        let error = self.error.clone();
        Box::pin(async move { Err(error) })
    }
}

/// An evidence client whose chosen call is down.
#[derive(Clone)]
pub struct EvidenceDown {
    pub inner: MemoryEvidence,
    pub fail_claim: bool,
    pub fail_computed: bool,
}

impl EvidenceClient for EvidenceDown {
    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<quantrule_service_api::RunClaimOutcome, ClientError>> {
        if self.fail_claim {
            return Box::pin(async {
                Err(ClientError::Unreachable {
                    service: Service::Evidence,
                    message: "evidence is down".to_owned(),
                })
            });
        }
        self.inner.claim(run_id, redelivered)
    }

    fn get_run(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>> {
        self.inner.get_run(run_id)
    }

    fn computed(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        if self.fail_computed {
            return Box::pin(async {
                Err(ClientError::Unreachable {
                    service: Service::Evidence,
                    message: "evidence is down".to_owned(),
                })
            });
        }
        self.inner.computed(run_id, digest, artifacts, stages)
    }

    fn failed(
        &self,
        run_id: String,
        error: String,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        self.inner.failed(run_id, error, stages)
    }
}

/// An evidence client that reports a cancellation on the Nth status check.
#[derive(Clone)]
pub struct BoundaryEvidence {
    pub inner: MemoryEvidence,
    pub cancel_on: usize,
    pub checks: Arc<Mutex<usize>>,
}

impl EvidenceClient for BoundaryEvidence {
    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<quantrule_service_api::RunClaimOutcome, ClientError>> {
        self.inner.claim(run_id, redelivered)
    }

    fn get_run(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, ClientError>> {
        let inner = self.inner.clone();
        let checks = Arc::clone(&self.checks);
        let cancel_on = self.cancel_on;
        Box::pin(async move {
            let count = {
                let mut checks = checks.lock().expect("the check counter");
                *checks += 1;
                *checks
            };
            let Some(mut record) = inner.get_run(run_id).await? else {
                return Ok(None);
            };
            if count == cancel_on {
                record.status = RunState::Cancelled;
            }
            Ok(Some(record))
        })
    }

    fn computed(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        self.inner.computed(run_id, digest, artifacts, stages)
    }

    fn failed(
        &self,
        run_id: String,
        error: String,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<RunRecord, ClientError>> {
        self.inner.failed(run_id, error, stages)
    }
}

pub fn services_with(
    fixture: &Fixture,
    evidence: Arc<dyn EvidenceClient>,
    data: Arc<dyn DataClient>,
    model: Arc<dyn ModelClient>,
) -> WorkerServices {
    WorkerServices {
        evidence,
        data,
        model,
        objects: Arc::clone(&fixture.services.objects),
        queue: Arc::clone(&fixture.services.queue),
    }
}
