//! The backtest worker over in-memory services.

mod common;

use common::canonical;

use quantrule_api::{Digest, ProfileId, RunState};
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, Queue, QueueError, StoreError};
use quantrule_runtime::client::{EvidenceClient, MemoryDataClient, MemoryEvidence};
use quantrule_server::backtest::{ProcessOutcome, WorkerServices, process};
use quantrule_service_api::{
    ConfigBundle, DatasetObject, NewRun, RunManifest, derive_run_id as derive,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Messages = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct MemoryQueue {
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
    fn keys(&self) -> Vec<String> {
        self.messages
            .lock()
            .expect("queue")
            .iter()
            .map(|(key, _)| key.clone())
            .collect()
    }

    fn payloads(&self, key: &str) -> Vec<Vec<u8>> {
        self.messages
            .lock()
            .expect("queue")
            .iter()
            .filter(|(name, _)| name == key)
            .map(|(_, payload)| payload.clone())
            .collect()
    }
}

struct Fixture {
    services: WorkerServices,
    runs: MemoryEvidence,
    queue: MemoryQueue,
    objects: common::MemoryObjects,
    config_id: String,
    invalid_config_id: String,
    data_version: Digest,
    config_hash: Digest,
    run_id: String,
}

fn csv() -> Vec<u8> {
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

fn bundle(root: &Path) -> ConfigBundle {
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

async fn fixture() -> Fixture {
    let profile = ProfileId::parse("default").expect("profile");
    let objects = common::MemoryObjects::default();
    let data = MemoryDataClient::default();
    let runs = MemoryEvidence::default();
    let queue = MemoryQueue::default();

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
        objects: Arc::new(objects.clone()),
        queue: Arc::new(queue.clone()),
    };
    Fixture {
        services,
        runs,
        queue,
        objects,
        config_id: valid.config_id,
        invalid_config_id: invalid.config_id,
        data_version,
        config_hash,
        run_id,
    }
}

#[derive(Clone)]
struct PutFails {
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
struct ManifestFails {
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

fn services_with_manifest_failure(fixture: &Fixture) -> WorkerServices {
    WorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        data: Arc::clone(&fixture.services.data),
        objects: Arc::new(ManifestFails {
            inner: fixture.objects.clone(),
        }),
        queue: Arc::clone(&fixture.services.queue),
    }
}

fn services_with_put_failure(fixture: &Fixture) -> WorkerServices {
    WorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        data: Arc::clone(&fixture.services.data),
        objects: Arc::new(PutFails {
            inner: fixture.objects.clone(),
        }),
        queue: Arc::clone(&fixture.services.queue),
    }
}

async fn submit_with(
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

#[tokio::test]
async fn a_queued_run_computes_artifacts_and_digest() {
    let fixture = fixture().await;
    let outcome = process(fixture.run_id.clone(), false, &fixture.services)
        .await
        .expect("process");
    assert_eq!(
        outcome,
        ProcessOutcome::Computed,
        "{:?}",
        fixture
            .runs
            .get_any(fixture.run_id.clone())
            .await
            .unwrap()
            .unwrap()
            .error
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.digest.is_some());
    assert!(!record.artifacts.is_empty());
    let evidence_stage = record
        .stages
        .iter()
        .find(|stage| stage.name == quantrule_api::StageName::Evidence)
        .expect("evidence stage");
    assert_eq!(evidence_stage.state, quantrule_api::StageStatus::Pending);
    for artifact in &record.artifacts {
        let key = RunManifest::blob_key(&artifact.sha256);
        let bytes = fixture.objects.get(&key).await.expect("artifact");
        assert_eq!(Digest::sha256(&bytes), artifact.sha256);
    }
    let manifest_key = RunManifest::key(&fixture.run_id);
    let manifest_bytes = fixture.objects.get(&manifest_key).await.expect("manifest");
    let manifest = RunManifest::from_bytes(&manifest_bytes).expect("manifest decode");
    assert_eq!(manifest.refs(), record.artifacts);
    assert_eq!(manifest.digest, record.digest.clone().expect("digest"));
    assert!(
        !fixture
            .objects
            .exists(&format!("runs/{}/results.json", fixture.run_id))
            .await
            .expect("per-name key")
    );
    let messages = fixture.queue.payloads("run.computed");
    assert_eq!(messages.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(payload, serde_json::json!({ "run_id": fixture.run_id }));
}

#[tokio::test]
async fn a_second_delivery_does_not_execute_again() {
    let fixture = fixture().await;
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("first"),
        ProcessOutcome::Computed
    );
    let first = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("second"),
        ProcessOutcome::Skipped
    );
    let second = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(first.artifacts, second.artifacts);
    assert_eq!(second.status, RunState::Running);
    assert_eq!(
        process(fixture.run_id.clone(), true, &fixture.services)
            .await
            .expect("redelivery"),
        ProcessOutcome::Skipped
    );
    let third = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(third.artifacts, first.artifacts);
    assert_eq!(third.attempts, 1);
    // A delivery after compute republishes so evidence still hears about a run
    // whose first publish was lost; it never executes it again.
    assert_eq!(fixture.queue.payloads("run.computed").len(), 3);
}

#[tokio::test]
async fn a_redelivery_executes_a_run_left_running_by_a_killed_worker() {
    let fixture = fixture().await;
    fixture
        .runs
        .claim(fixture.run_id.clone(), false)
        .await
        .expect("simulated claim");
    let outcome = process(fixture.run_id.clone(), true, &fixture.services)
        .await
        .expect("process");
    assert_eq!(outcome, ProcessOutcome::Computed);
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert_eq!(record.attempts, 2);
    assert!(record.digest.is_some());
    assert!(!record.artifacts.is_empty());
}

#[derive(Clone)]
struct StopAfter {
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

fn services_with_stop_after(fixture: &Fixture, allowed: usize) -> WorkerServices {
    WorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        data: Arc::clone(&fixture.services.data),
        objects: Arc::new(StopAfter {
            inner: fixture.objects.clone(),
            allowed,
            puts: Arc::new(Mutex::new(0)),
        }),
        queue: Arc::clone(&fixture.services.queue),
    }
}

#[tokio::test]
async fn a_stopped_write_leaves_no_manifest() {
    let fixture = fixture().await;
    let services = services_with_stop_after(&fixture, 1);
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("process"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.artifacts.is_empty());
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .any(|key| key.starts_with("artifacts/"))
    );
    let run_prefix = format!("runs/{}/", fixture.run_id);
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .all(|key| !key.starts_with(&run_prefix)),
        "a stopped write must not publish a run prefix"
    );
}

#[tokio::test]
async fn a_manifest_write_failure_leaves_no_published_set() {
    let fixture = fixture().await;
    let services = services_with_manifest_failure(&fixture);
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("first"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.artifacts.is_empty());
    assert!(
        !fixture
            .objects
            .exists(&RunManifest::key(&fixture.run_id))
            .await
            .expect("manifest")
    );
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .any(|key| key.starts_with("artifacts/"))
    );
    let run_prefix = format!("runs/{}/", fixture.run_id);
    assert!(
        fixture
            .objects
            .keys()
            .iter()
            .all(|key| !key.starts_with(&run_prefix)),
        "a killed worker must not publish a run prefix"
    );

    // Restart on the durable store: the blobs are reused and the set publishes.
    assert_eq!(
        process(fixture.run_id.clone(), true, &fixture.services)
            .await
            .expect("restart"),
        ProcessOutcome::Computed
    );
}

#[tokio::test]
async fn a_stage_failure_retries_then_dead_letters() {
    let fixture = fixture().await;
    let services = services_with_put_failure(&fixture);
    assert_eq!(
        process(fixture.run_id.clone(), false, &services)
            .await
            .expect("first"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(5)
        }
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert_eq!(record.attempts, 1);
    assert!(!fixture.queue.keys().contains(&"run.dead".to_owned()));

    assert_eq!(
        process(fixture.run_id.clone(), true, &services)
            .await
            .expect("second"),
        ProcessOutcome::Retry {
            delay: Duration::from_secs(25)
        }
    );
    assert_eq!(
        process(fixture.run_id.clone(), true, &services)
            .await
            .expect("third"),
        ProcessOutcome::Exhausted
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert_eq!(record.attempts, 3);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("artifact store"))
    );
    assert!(record.artifacts.is_empty());
    assert!(
        !fixture
            .objects
            .exists(&RunManifest::key(&fixture.run_id))
            .await
            .expect("manifest")
    );
    assert!(
        !fixture
            .objects
            .exists(&format!("runs/{}/results.json", fixture.run_id))
            .await
            .expect("per-name key")
    );
    assert!(fixture.queue.keys().contains(&"run.dead".to_owned()));
    let messages = fixture.queue.payloads("run.dead");
    assert_eq!(messages.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(payload, serde_json::json!({ "run_id": fixture.run_id }));
}

#[tokio::test]
async fn a_run_cancelled_after_claim_writes_no_bundle() {
    let fixture = fixture().await;
    fixture
        .runs
        .claim(fixture.run_id.clone(), false)
        .await
        .expect("claim");
    fixture
        .runs
        .cancel(
            &ProfileId::parse("default").expect("profile"),
            fixture.run_id.clone(),
        )
        .await
        .expect("cancel");
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Skipped
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Cancelled);
    assert!(record.artifacts.is_empty());
    assert!(
        !fixture
            .objects
            .exists(&RunManifest::key(&fixture.run_id))
            .await
            .expect("manifest")
    );
    assert!(
        !fixture
            .objects
            .exists(&format!("runs/{}/results.json", fixture.run_id))
            .await
            .expect("exists")
    );
}

#[tokio::test]
async fn a_cancelled_run_is_not_executed() {
    let fixture = fixture().await;
    fixture
        .runs
        .cancel(
            &ProfileId::parse("default").expect("profile"),
            fixture.run_id.clone(),
        )
        .await
        .expect("cancel");
    assert_eq!(
        process(fixture.run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Skipped
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id)
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Cancelled);
    assert!(record.artifacts.is_empty());
}

#[tokio::test]
async fn a_missing_dataset_fails_the_run() {
    let fixture = fixture().await;
    let run_id = submit_with(
        &fixture.runs,
        Digest::sha256(b"missing-dataset"),
        fixture.config_id.clone(),
        fixture.config_hash.clone(),
        "missing",
    )
    .await;
    assert_eq!(
        process(run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Failed
    );
    let record = fixture
        .runs
        .get_any(run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("dataset"))
    );
    let messages = fixture.queue.payloads("run.failed");
    assert_eq!(messages.len(), 1);
    let payload: serde_json::Value = serde_json::from_slice(&messages[0]).expect("payload");
    assert_eq!(payload, serde_json::json!({ "run_id": run_id }));
}

#[tokio::test]
async fn an_invalid_config_fails_the_run() {
    let fixture = fixture().await;
    let run_id = submit_with(
        &fixture.runs,
        fixture.data_version.clone(),
        fixture.invalid_config_id.clone(),
        Digest::sha256(b"invalid"),
        "invalid",
    )
    .await;
    assert_eq!(
        process(run_id.clone(), false, &fixture.services)
            .await
            .expect("process"),
        ProcessOutcome::Failed
    );
    let record = fixture
        .runs
        .get_any(run_id)
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Failed);
    assert!(record.artifacts.is_empty());
}
