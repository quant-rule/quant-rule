//! The SQLite run transaction's cross-connection behaviour.
//!
//! The embedded profile opens the same SQLite file from more than one process,
//! so a claim is not protected by one async mutex. Each transition runs inside
//! `BEGIN IMMEDIATE` and re-reads the run row; these tests open two independent
//! connections to one file and prove that:
//!
//! - two live deliveries of a queued run claim it exactly once;
//! - a consumer killed after a claim leaves a redeliverable running row that
//!   the next connection reclaims against the same attempt budget;
//! - the attempt count and the terminal state survive closing and reopening the
//!   file.
//!
//! The test is deterministic: it asserts a set of outcomes and the stored row,
//! never the order in which the two blocking tasks happened to finish.

use std::path::{Path, PathBuf};

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunState, StageName, StageProgress, StageStatus,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::runs::{ClaimOutcome, NewRun, RunStore, derive};
use quantrule_evidence_service::sqlite::SqliteRunStore;
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

fn directory(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&directory);
    directory
}

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("the profile")
}

fn new_run(seed: &[u8]) -> NewRun {
    let data_version = Digest::sha256(seed);
    let config_hash = Digest::sha256(b"transaction-config");
    let image_digest = Digest::sha256(b"transaction-image");
    NewRun {
        run_id: derive(&data_version, &config_hash, "M1.0", &image_digest),
        config_id: "cfg_transaction".to_owned(),
        data_version,
        config_hash,
        model_version: "M1.0".to_owned(),
        image_digest,
        model_artifact_id: None,
        owner: "transaction".to_owned(),
        submitted_at: "2026-11-15T00:00:00Z".to_owned(),
    }
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

#[tokio::test]
async fn two_live_deliveries_claim_a_queued_run_once() {
    let directory = directory("evidence-claim-race");
    let path = directory.join("quantrule.sqlite");
    let first = SqliteDatabase::open(&path).expect("the first connection");
    apply_migrations_in(&first, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let second = SqliteDatabase::open(&path).expect("the second connection");
    let left = SqliteRunStore::new(first);
    let right = SqliteRunStore::new(second);

    let submitted = left
        .submit(profile("default"), new_run(b"claim-race"))
        .await
        .expect("the submission");
    let id = submitted.record().run_id.to_string();

    let (left_outcome, right_outcome) = tokio::join!(
        left.claim(id.clone(), false),
        right.claim(id.clone(), false),
    );
    let outcomes = [
        left_outcome.expect("the left claim"),
        right_outcome.expect("the right claim"),
    ];
    let claimed = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ClaimOutcome::Claimed(_)))
        .count();
    let running = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ClaimOutcome::Running(_)))
        .count();
    assert_eq!(
        (claimed, running),
        (1, 1),
        "two live deliveries did not resolve to one claim and one live holder: {outcomes:?}"
    );

    let record = left.get_any(id).await.expect("the read").expect("the run");
    assert_eq!(record.status, RunState::Running);
    assert_eq!(record.attempts, 1, "the claim was counted twice");

    let _ = std::fs::remove_dir_all(&directory);
}

#[tokio::test]
async fn a_killed_consumer_is_reclaimed_and_the_state_survives_reopen() {
    let directory = directory("evidence-reclaim");
    let path = directory.join("quantrule.sqlite");
    let first = SqliteDatabase::open(&path).expect("the first connection");
    apply_migrations_in(&first, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let second = SqliteDatabase::open(&path).expect("the second connection");
    let owner = SqliteRunStore::new(first.clone());
    let survivor = SqliteRunStore::new(second.clone());

    let submitted = owner
        .submit(profile("default"), new_run(b"reclaim"))
        .await
        .expect("the submission");
    let id = submitted.record().run_id.to_string();
    let ClaimOutcome::Claimed(claimed) = owner.claim(id.clone(), false).await.expect("the claim")
    else {
        panic!("the queued run was not claimed");
    };
    assert_eq!(claimed.attempts, 1);

    // The owner "dies" here: it never calls computed or complete. A broker
    // redelivery is the only proof the holder is gone, so the survivor may
    // reclaim the same uncomputed running row.
    let ClaimOutcome::Claimed(reclaimed) =
        survivor.claim(id.clone(), true).await.expect("the reclaim")
    else {
        panic!("a killed consumer's run was not reclaimed");
    };
    assert_eq!(reclaimed.attempts, 2);
    assert_eq!(reclaimed.status, RunState::Running);
    assert_eq!(reclaimed.digest, None);

    let digest = Digest::sha256(b"reclaim-digest");
    let artifacts = vec![ArtifactRef {
        name: "results.json".to_owned(),
        bytes: 32,
        sha256: Digest::sha256(b"reclaim-results"),
    }];
    let computed = survivor
        .computed(id.clone(), digest.clone(), artifacts.clone(), done_stages())
        .await
        .expect("the computed write");
    assert_eq!(computed.digest, Some(digest.clone()));

    let ClaimOutcome::Running(computed_live) = survivor
        .claim(id.clone(), true)
        .await
        .expect("the computed claim")
    else {
        panic!("a computed run was reclaimed");
    };
    assert_eq!(computed_live.attempts, 2);
    assert_eq!(computed_live.digest, Some(digest.clone()));

    let completed = survivor
        .complete(id.clone(), digest.clone(), artifacts, done_stages())
        .await
        .expect("the completion");
    assert_eq!(completed.status, RunState::Succeeded);
    assert_eq!(completed.attempts, 2);

    drop(survivor);
    drop(owner);
    drop(second);
    drop(first);

    let reopened = SqliteDatabase::open(&path).expect("the file reopens");
    let store = SqliteRunStore::new(reopened);
    let record = store
        .get_any(id)
        .await
        .expect("the reopened read")
        .expect("the reopened run");
    assert_eq!(record.status, RunState::Succeeded);
    assert_eq!(record.attempts, 2);
    assert_eq!(record.digest, Some(digest));
    assert_eq!(record.stages, done_stages());

    let _ = std::fs::remove_dir_all(&directory);
}
