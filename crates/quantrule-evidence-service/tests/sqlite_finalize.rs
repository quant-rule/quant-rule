//! The evidence finalizer over the SQLite run store.
//!
//! `finalize` reads the run through `RunStore::get_any`, verifies the manifest
//! and every artifact blob, and then calls `complete` or `fail`. These tests run
//! that whole read path against the SQLite store so the embedded profile cannot
//! finalize a run that the store cannot read or write.

mod common;

use std::sync::Arc;

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunState, StageName, StageProgress, StageStatus,
};
use quantrule_evidence_service::evidence::{
    EvidenceServices, FinalizeOutcome, RunManifest, finalize,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::runs::{NewRun, RunStore, derive};
use quantrule_evidence_service::sqlite::SqliteRunStore;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

fn computed_stages() -> Vec<StageProgress> {
    [
        (StageName::Data, StageStatus::Done),
        (StageName::Model, StageStatus::Done),
        (StageName::Backtest, StageStatus::Done),
        (StageName::Evidence, StageStatus::Pending),
    ]
    .into_iter()
    .map(|(name, state)| StageProgress {
        name,
        state,
        elapsed_ms: None,
    })
    .collect()
}

async fn store() -> Arc<SqliteRunStore> {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    Arc::new(SqliteRunStore::new(database))
}

async fn claimed(store: &SqliteRunStore) -> String {
    let data_version = Digest::sha256(b"finalize-data");
    let config_hash = Digest::sha256(b"finalize-config");
    let image_digest = Digest::sha256(b"finalize-image");
    let submitted = store
        .submit(
            ProfileId::parse("default").expect("the profile"),
            NewRun {
                run_id: derive(&data_version, &config_hash, "M1.0", &image_digest),
                config_id: "cfg_finalize".to_owned(),
                data_version,
                config_hash,
                model_version: "M1.0".to_owned(),
                image_digest,
                owner: "default".to_owned(),
                submitted_at: "2026-11-15T00:00:00Z".to_owned(),
            },
        )
        .await
        .expect("the submission");
    let run_id = submitted.record().run_id.to_string();
    let claimed = store.claim(run_id.clone(), false).await.expect("the claim");
    assert!(matches!(
        claimed,
        quantrule_evidence_service::runs::ClaimOutcome::Claimed(_)
    ));
    run_id
}

async fn put(objects: &common::MemoryObjects, name: &str, bytes: &[u8]) -> ArtifactRef {
    let sha256 = Digest::sha256(bytes);
    objects
        .put(&RunManifest::blob_key(&sha256), bytes.to_vec())
        .await
        .expect("the blob");
    ArtifactRef {
        name: name.to_owned(),
        bytes: bytes.len() as u64,
        sha256,
    }
}

async fn computed(
    store: &SqliteRunStore,
    objects: &common::MemoryObjects,
    run_id: &str,
) -> (Digest, Vec<ArtifactRef>) {
    let results = put(objects, "results.json", b"{\"profit\":1}").await;
    let trades = put(objects, "trades.csv", b"entry,exit\n").await;
    let artifacts = vec![results, trades];
    let manifest = RunManifest::from_artifacts(run_id, &artifacts);
    objects
        .put(
            &RunManifest::key(run_id),
            manifest.to_bytes().expect("the manifest bytes"),
        )
        .await
        .expect("the manifest");
    store
        .computed(
            run_id.to_owned(),
            manifest.digest.clone(),
            artifacts.clone(),
            computed_stages(),
        )
        .await
        .expect("the computed set");
    (manifest.digest, artifacts)
}

#[tokio::test]
async fn finalize_reads_the_sqlite_run_and_completes_it() {
    let store = store().await;
    let objects = common::MemoryObjects::default();
    let run_id = claimed(&store).await;
    let (digest, artifacts) = computed(&store, &objects, &run_id).await;

    let services = EvidenceServices {
        runs: Arc::clone(&store) as Arc<dyn RunStore>,
        objects: Arc::new(objects),
    };
    assert_eq!(
        finalize(run_id.clone(), &services)
            .await
            .expect("the finalization"),
        FinalizeOutcome::Succeeded
    );

    let record = store
        .get_any(run_id)
        .await
        .expect("the read")
        .expect("the run");
    assert_eq!(record.status, RunState::Succeeded);
    assert_eq!(record.digest, Some(digest));
    assert_eq!(record.artifacts, artifacts);
    assert_eq!(record.error, None);
    let evidence = record
        .stages
        .iter()
        .find(|stage| stage.name == StageName::Evidence)
        .expect("the evidence stage");
    assert_eq!(evidence.state, StageStatus::Done);
}

#[tokio::test]
async fn finalize_records_a_changed_artifact_on_the_sqlite_row() {
    let store = store().await;
    let objects = common::MemoryObjects::default();
    let run_id = claimed(&store).await;
    let (digest, artifacts) = computed(&store, &objects, &run_id).await;
    objects
        .put(
            &RunManifest::blob_key(&artifacts[0].sha256),
            b"tampered".to_vec(),
        )
        .await
        .expect("the changed blob");

    let services = EvidenceServices {
        runs: Arc::clone(&store) as Arc<dyn RunStore>,
        objects: Arc::new(objects),
    };
    assert_eq!(
        finalize(run_id.clone(), &services)
            .await
            .expect("the finalization"),
        FinalizeOutcome::Failed
    );

    let record = store
        .get_any(run_id)
        .await
        .expect("the read")
        .expect("the run");
    assert_eq!(record.status, RunState::Failed);
    assert_eq!(
        record.digest,
        Some(digest),
        "a recorded digest survives the evidence failure"
    );
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("does not match its hash")),
        "the failure does not name the changed artifact: {:?}",
        record.error
    );
    let evidence = record
        .stages
        .iter()
        .find(|stage| stage.name == StageName::Evidence)
        .expect("the evidence stage");
    assert_eq!(evidence.state, StageStatus::Failed);
}
