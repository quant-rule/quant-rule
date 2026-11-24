//! Shared fixtures and fakes for the diagnostic stage worker tests.

use std::sync::{Arc, Mutex};

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, StageArtifact, StageInput, StageKind, StageProvenance,
};
use quantrule_backtest_service::stage::{
    StageDispatch, StageExecutor, StageExecutorError, StageFile, StageOutput, StageWorkerServices,
};
use quantrule_runtime::adapters::memory::MemoryObjects;
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, Queue, QueueError, StoreError};
use quantrule_runtime::client::MemoryStageClient;
use quantrule_service_api::{NewStageRun, StageManifest, StageRecord};

#[allow(unused_imports)]
pub use quantrule_api::{StageRunDetail, StageRunSummary, StageState};
#[allow(unused_imports)]
pub use quantrule_runtime::client::ClientError;
#[allow(unused_imports)]
pub use quantrule_service_api::{STAGE_MANIFEST_SCHEMA, StageClaimOutcome};

type Messages = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

/// A queue fake that records every publish.
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

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

fn profile() -> ProfileId {
    ProfileId::parse("default").expect("profile")
}

/// The stage submitted by the fixture.
pub fn new_stage(seed: &[u8]) -> NewStageRun {
    let data_version = digest(seed);
    NewStageRun::new(
        StageKind::Model,
        "cfg-1".to_owned(),
        StageInput::DataVersion {
            data_version: data_version.clone(),
        },
        StageProvenance {
            data_version,
            config_hash: digest(b"config"),
            model_version: "M1.3".to_owned(),
            image_digest: digest(b"image"),
        },
        "default".to_owned(),
        "2026-11-17T00:00:00Z".to_owned(),
    )
}

pub struct Fixture {
    pub evidence: MemoryStageClient,
    pub objects: MemoryObjects,
    pub queue: MemoryQueue,
    pub services: StageWorkerServices,
    pub stage_run_id: String,
}

impl Fixture {
    pub async fn new(seed: &[u8]) -> Self {
        let evidence = MemoryStageClient::default();
        let objects = MemoryObjects::default();
        let queue = MemoryQueue::default();
        let record = evidence
            .submit(&profile(), new_stage(seed))
            .await
            .expect("submit the stage");
        let services = StageWorkerServices {
            evidence: Arc::new(evidence.clone()),
            objects: Arc::new(objects.clone()),
            queue: Arc::new(queue.clone()),
            holder: "stage-worker-test".to_owned(),
        };
        Self {
            evidence,
            objects,
            queue,
            services,
            stage_run_id: record.stage_run_id.to_string(),
        }
    }
}

/// A deterministic successful executor.
pub struct Success(pub Vec<StageFile>);

impl StageExecutor for Success {
    fn execute(&self, _record: &StageRecord) -> BoxFuture<Result<StageOutput, StageExecutorError>> {
        let files = self.0.clone();
        Box::pin(async move { Ok(StageOutput { files }) })
    }
}

/// An executor that fails before producing a file.
pub struct Failure(pub &'static str);

impl StageExecutor for Failure {
    fn execute(&self, _record: &StageRecord) -> BoxFuture<Result<StageOutput, StageExecutorError>> {
        let message = self.0.to_owned();
        Box::pin(async move { Err(StageExecutorError(message)) })
    }
}

/// An executor that cancels the stage before it returns.
pub struct CancelDuring {
    pub evidence: MemoryStageClient,
    pub stage_run_id: String,
}

impl StageExecutor for CancelDuring {
    fn execute(&self, _record: &StageRecord) -> BoxFuture<Result<StageOutput, StageExecutorError>> {
        let evidence = self.evidence.clone();
        let stage_run_id = self.stage_run_id.clone();
        Box::pin(async move {
            evidence
                .cancel(&profile(), stage_run_id)
                .await
                .map_err(|error| StageExecutorError(error.to_string()))?;
            Ok(StageOutput::one("results.json", b"late".to_vec()))
        })
    }
}

/// The dispatcher for one fake executor.
pub fn dispatch(executor: Arc<dyn StageExecutor>) -> StageDispatch {
    StageDispatch::uniform(executor)
}

/// A successful output.
pub fn output() -> StageOutput {
    StageOutput {
        files: vec![
            StageFile {
                name: "summary.txt".to_owned(),
                bytes: b"summary".to_vec(),
            },
            StageFile {
                name: "results.json".to_owned(),
                bytes: b"{\"coverage\":1}".to_vec(),
            },
        ],
    }
}

/// The artifact identity the success output derives.
pub fn expected_artifact() -> StageArtifact {
    let files = output()
        .files
        .into_iter()
        .map(|file| ArtifactRef {
            name: file.name,
            bytes: file.bytes.len() as u64,
            sha256: Digest::sha256(&file.bytes),
        })
        .collect();
    StageArtifact::from_files(files).expect("artifact")
}

/// An object store whose manifest put fails after the blobs were accepted.
#[derive(Clone)]
pub struct ManifestFails {
    pub inner: MemoryObjects,
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

    fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
        self.inner.delete(key)
    }
}

/// The services whose object store refuses the manifest.
pub fn services_with_manifest_failure(fixture: &Fixture) -> StageWorkerServices {
    StageWorkerServices {
        evidence: Arc::clone(&fixture.services.evidence),
        objects: Arc::new(ManifestFails {
            inner: fixture.objects.clone(),
        }),
        queue: Arc::clone(&fixture.services.queue),
        holder: "stage-worker-test".to_owned(),
    }
}

pub fn manifest_key(record: &StageRecord) -> String {
    StageManifest::key(&record.stage_run_id)
}
