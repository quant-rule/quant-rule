//! Stage retention: expiry removes only diagnostic rows and stage objects.

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunId, StageArtifact, StageInput, StageKind, StageProvenance,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::operations::OperationsStatus;
use quantrule_evidence_service::runs::{MemoryRuns, NewRun, RunStore};
use quantrule_evidence_service::sqlite::SqliteRunStore;
use quantrule_evidence_service::stage::{
    MemoryStages, NewStageRun, SqliteStageStore, StageDeletion, StageStore, record_computed,
    retention,
};
use quantrule_runtime::adapters::memory::MemoryObjects;
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};
use quantrule_runtime::clock;
use quantrule_service_api::StageManifest;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

fn digest(seed: &[u8]) -> Digest {
    Digest::sha256(seed)
}

fn stage(seed: &[u8], submitted_at: &str) -> NewStageRun {
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
        submitted_at.to_owned(),
    )
}

fn artifact(seed: &[u8]) -> StageArtifact {
    StageArtifact::from_files(vec![ArtifactRef {
        name: "results.json".to_owned(),
        bytes: seed.len() as u64,
        sha256: digest(seed),
    }])
    .expect("artifact")
}

fn run(seed: &[u8], submitted_at: &str) -> NewRun {
    NewRun {
        run_id: RunId(digest(seed)),
        config_id: "cfg-1".to_owned(),
        data_version: digest(b"data"),
        config_hash: digest(b"config"),
        model_version: "M1.3".to_owned(),
        image_digest: digest(b"image"),
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: submitted_at.to_owned(),
    }
}

async fn write_stage_objects(objects: &MemoryObjects, record: &quantrule_service_api::StageRecord) {
    let artifact = artifact(b"stage-bytes");
    let manifest = StageManifest::new(
        record.stage_run_id.clone(),
        record.kind,
        record.config_id.clone(),
        &record.provenance,
        record.input.clone(),
        artifact.files.clone(),
    )
    .expect("manifest");
    objects
        .put(
            &StageManifest::key(&record.stage_run_id),
            manifest.to_bytes().expect("bytes"),
        )
        .await
        .expect("manifest");
    objects
        .put(
            &StageManifest::blob_key(&artifact.files[0].sha256),
            b"stage-bytes".to_vec(),
        )
        .await
        .expect("blob");
}

#[tokio::test]
async fn an_expired_stage_disappears_and_a_run_remains() {
    let stages = MemoryStages::default();
    let runs = MemoryRuns::default();
    let created = stages
        .create(profile("default"), stage(b"old", "2020-01-01T00:00:00Z"))
        .await
        .expect("stage");
    let record = created.record().clone();
    let run = runs
        .submit(profile("default"), run(b"old-run", "2020-01-01T00:00:00Z"))
        .await
        .expect("run");
    let objects = MemoryObjects::default();
    write_stage_objects(&objects, &record).await;
    let run_key = format!("artifacts/{}", digest(b"run-bytes").as_hex());
    objects
        .put(&run_key, b"run-bytes".to_vec())
        .await
        .expect("run artifact");

    let report = retention::purge_expired(&stages, &objects, "2025-01-01T00:00:00Z")
        .await
        .expect("purge");
    assert_eq!(report.stages, 1);
    assert!(report.objects >= 1);
    assert!(
        stages
            .get_any(record.stage_run_id.to_string())
            .await
            .expect("stage")
            .is_none(),
        "the expired stage remains"
    );
    assert!(
        runs.get(profile("default"), run.record().run_id.to_string())
            .await
            .expect("run")
            .is_some(),
        "retention deleted a run"
    );
    assert!(
        objects.exists(&run_key).await.expect("run key"),
        "retention deleted a run object"
    );
    assert!(
        !objects
            .exists(&StageManifest::key(&record.stage_run_id))
            .await
            .expect("manifest key"),
        "the stage manifest remains"
    );
}

#[tokio::test]
async fn a_shared_stage_blob_survives_until_the_last_reference_is_gone() {
    let stages = MemoryStages::default();
    let objects = MemoryObjects::default();
    let first = stages
        .create(profile("default"), stage(b"one", "2026-11-17T00:00:00Z"))
        .await
        .expect("first")
        .record()
        .clone();
    let second = stages
        .create(profile("default"), stage(b"two", "2026-11-17T00:00:01Z"))
        .await
        .expect("second")
        .record()
        .clone();
    let artifact = artifact(b"shared");
    for record in [&first, &second] {
        let manifest = StageManifest::new(
            record.stage_run_id.clone(),
            record.kind,
            record.config_id.clone(),
            &record.provenance,
            record.input.clone(),
            artifact.files.clone(),
        )
        .expect("manifest");
        objects
            .put(
                &StageManifest::key(&record.stage_run_id),
                manifest.to_bytes().expect("bytes"),
            )
            .await
            .expect("manifest");
    }
    objects
        .put(
            &StageManifest::blob_key(&artifact.files[0].sha256),
            b"shared".to_vec(),
        )
        .await
        .expect("blob");
    stages
        .claim(first.stage_run_id.to_string(), false)
        .await
        .expect("claim first");
    stages
        .claim(second.stage_run_id.to_string(), false)
        .await
        .expect("claim second");
    stages
        .complete(first.stage_run_id.to_string(), artifact.clone())
        .await
        .expect("complete first");
    stages
        .complete(second.stage_run_id.to_string(), artifact.clone())
        .await
        .expect("complete second");

    let deletion = StageDeletion {
        stage_run_id: first.stage_run_id.clone(),
        artifact_hashes: vec![artifact.files[0].sha256.clone()],
    };
    let removed = retention::remove_deleted(&stages, &objects, &deletion)
        .await
        .expect("remove first");
    assert_eq!(removed, 1, "only the manifest should be removed");
    assert!(
        objects
            .exists(&StageManifest::blob_key(&artifact.files[0].sha256))
            .await
            .expect("blob"),
        "a shared blob was deleted"
    );
}

#[tokio::test]
async fn the_sqlite_stage_tables_expire_without_touching_a_run() {
    let database = SqliteDatabase::open_in_memory().expect("database");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("migrations");
    let stages = SqliteStageStore::new(database.clone());
    let runs = SqliteRunStore::new(database);
    let created = stages
        .create(profile("default"), stage(b"old", "2020-01-01T00:00:00Z"))
        .await
        .expect("stage");
    let run = runs
        .submit(profile("default"), run(b"old-run", "2020-01-01T00:00:00Z"))
        .await
        .expect("run");
    stages
        .claim(created.record().stage_run_id.to_string(), false)
        .await
        .expect("claim");
    stages
        .complete(
            created.record().stage_run_id.to_string(),
            artifact(b"sqlite"),
        )
        .await
        .expect("complete");

    let report =
        retention::purge_expired(&stages, &MemoryObjects::default(), "2025-01-01T00:00:00Z")
            .await
            .expect("purge");
    assert_eq!(report.stages, 1);
    assert!(
        stages
            .get_any(created.record().stage_run_id.to_string())
            .await
            .expect("stage")
            .is_none()
    );
    assert!(
        runs.get(profile("default"), run.record().run_id.to_string())
            .await
            .expect("run")
            .is_some(),
        "the SQLite retention deleted a run"
    );
}

#[tokio::test]
async fn run_cleanup_reports_the_counts_and_utc_instant() {
    let stages = MemoryStages::default();
    let objects = MemoryObjects::default();
    let created = stages
        .create(profile("default"), stage(b"old", "2020-01-01T00:00:00Z"))
        .await
        .expect("stage");
    write_stage_objects(&objects, created.record()).await;
    let now = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
    let report = retention::run_cleanup(&stages, &objects, now, 30).await;
    assert_eq!(report.completed_utc, clock::rfc3339(1_800_000_000));
    assert_eq!(report.stages_removed, 1);
    assert!(report.objects_removed >= 1);
    assert_eq!(report.error, None);
}

#[tokio::test]
async fn a_failed_cleanup_reports_the_error_and_zero_counts() {
    let stages = MemoryStages::default();
    let objects = MemoryObjects::default();
    let before = UNIX_EPOCH - Duration::from_secs(1);
    let report = retention::run_cleanup(&stages, &objects, before, 30).await;
    assert_eq!(report.completed_utc, "1970-01-01T00:00:00Z");
    assert_eq!(report.stages_removed, 0);
    assert_eq!(report.objects_removed, 0);
    assert!(report.error.is_some(), "a clock failure is reported");
}

#[tokio::test]
async fn an_unexpired_stage_is_kept() {
    let stages = MemoryStages::default();
    let objects = MemoryObjects::default();
    let created = stages
        .create(profile("default"), stage(b"recent", "2026-11-17T00:00:00Z"))
        .await
        .expect("stage");
    let report = retention::purge_expired(&stages, &objects, "2026-11-01T00:00:00Z")
        .await
        .expect("purge");
    assert_eq!(report.stages, 0);
    assert!(
        stages
            .get_any(created.record().stage_run_id.to_string())
            .await
            .expect("stage")
            .is_some()
    );
}

#[tokio::test(start_paused = true)]
async fn the_hourly_sweep_removes_a_stage_without_a_restart() {
    let stages: Arc<dyn StageStore> = Arc::new(MemoryStages::default());
    let storage = Arc::new(MemoryObjects::default());
    let objects: Arc<dyn ObjectStore> = storage.clone();
    let operations = OperationsStatus::new(30, true);
    let created = stages
        .create(profile("default"), stage(b"old", "2020-01-01T00:00:00Z"))
        .await
        .expect("stage");
    write_stage_objects(&storage, created.record()).await;

    let task = tokio::spawn(retention::sweep(
        Arc::clone(&stages),
        Arc::clone(&objects),
        Arc::clone(&operations),
        Arc::new(tokio::sync::Mutex::new(())),
        30,
        retention::SWEEP_INTERVAL,
    ));

    // The first tick is immediate; wait for the startup pass to record.
    for _ in 0..10 {
        tokio::task::yield_now().await;
        if operations.report().last_cleanup.is_some() {
            break;
        }
    }
    assert!(
        operations.report().last_cleanup.is_some(),
        "the startup sweep did not run"
    );
    assert!(
        stages
            .get_any(created.record().stage_run_id.to_string())
            .await
            .expect("stage")
            .is_none()
    );

    // A stage written after the startup pass is removed by the next tick,
    // without restarting the service.
    let second = stages
        .create(profile("default"), stage(b"later", "2020-01-01T00:00:00Z"))
        .await
        .expect("stage");
    write_stage_objects(&storage, second.record()).await;
    tokio::time::advance(retention::SWEEP_INTERVAL).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
        if stages
            .get_any(second.record().stage_run_id.to_string())
            .await
            .expect("stage")
            .is_none()
        {
            break;
        }
    }
    assert!(
        stages
            .get_any(second.record().stage_run_id.to_string())
            .await
            .expect("stage")
            .is_none(),
        "the next hourly sweep did not run"
    );
    task.abort();
}

/// A store that pauses the first blob read, so a concurrent delete can be
/// started while the record path holds the evidence gate.
struct BlockingObjects {
    inner: Arc<MemoryObjects>,
    blob_key: String,
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

impl ObjectStore for BlockingObjects {
    fn backend(&self) -> Backend {
        self.inner.backend()
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        self.inner.put(key, bytes)
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        let inner = Arc::clone(&self.inner);
        let blob = self.blob_key.clone();
        let entered = Arc::clone(&self.entered);
        let release = Arc::clone(&self.release);
        let key = key.to_owned();
        Box::pin(async move {
            if key == blob {
                entered.notify_one();
                release.notified().await;
            }
            inner.get(&key).await
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        self.inner.exists(key)
    }

    fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
        self.inner.delete(key)
    }
}

#[tokio::test]
async fn a_record_in_progress_keeps_the_blob_from_a_concurrent_delete() {
    let stages = Arc::new(MemoryStages::default());
    let inner = Arc::new(MemoryObjects::default());
    let artifact = artifact(b"stage-bytes");
    let blob_key = StageManifest::blob_key(&artifact.files[0].sha256);

    // The new stage is running and its manifest and blob are already written.
    let new = stages
        .create(profile("default"), stage(b"new", "2020-01-01T00:00:00Z"))
        .await
        .expect("new stage");
    let new_id = new.record().stage_run_id.to_string();
    stages
        .claim(new_id.clone(), false)
        .await
        .expect("claim new");
    write_stage_objects(&inner, new.record()).await;

    // The expired stage owns the same blob and is deleted through the store.
    let old = stages
        .create(profile("default"), stage(b"old", "2020-01-01T00:00:00Z"))
        .await
        .expect("old stage");
    let old_id = old.record().stage_run_id.to_string();
    stages
        .claim(old_id.clone(), false)
        .await
        .expect("claim old");
    stages
        .complete(old_id.clone(), artifact.clone())
        .await
        .expect("complete old");
    let deletion = stages
        .delete(profile("default"), old_id)
        .await
        .expect("delete old")
        .expect("the old stage");

    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let objects = Arc::new(BlockingObjects {
        inner: Arc::clone(&inner),
        blob_key: blob_key.clone(),
        entered: Arc::clone(&entered),
        release: Arc::clone(&release),
    });
    let gate = Arc::new(tokio::sync::Mutex::new(()));

    let record_gate = Arc::clone(&gate);
    let record_stages = Arc::clone(&stages);
    let record_objects = Arc::clone(&objects);
    let record_new = new_id.clone();
    let record = tokio::spawn(async move {
        let _guard = record_gate.lock().await;
        record_computed(
            record_stages.as_ref(),
            record_objects.as_ref(),
            &record_new,
            artifact,
        )
        .await
    });

    tokio::time::timeout(Duration::from_secs(1), entered.notified())
        .await
        .expect("the record reaches the blob read");

    let delete_gate = Arc::clone(&gate);
    let delete_stages = Arc::clone(&stages);
    let delete_objects = Arc::clone(&objects);
    let delete = tokio::spawn(async move {
        let _guard = delete_gate.lock().await;
        retention::remove_deleted(delete_stages.as_ref(), delete_objects.as_ref(), &deletion).await
    });

    tokio::task::yield_now().await;
    assert!(
        !delete.is_finished(),
        "the delete must wait for the record's gate"
    );
    release.notify_one();
    record
        .await
        .expect("record task")
        .expect("the record completes");
    delete
        .await
        .expect("delete task")
        .expect("the delete completes");
    assert!(
        inner.exists(&blob_key).await.expect("blob exists"),
        "the new stage's blob was deleted"
    );
    let stored = stages
        .get_any(new_id)
        .await
        .expect("get new")
        .expect("new stage");
    assert_eq!(stored.state, quantrule_api::StageState::Succeeded);
}
