//! The shared `RunStore` and `HypothesisStore` conformance suites.
//!
//! The same body runs against the SQLite store and — when
//! `QR_TEST_DATABASE_URL` names a disposable Postgres — the Postgres store. The
//! fast tier runs the SQLite case; the Postgres case is opt-in because it needs
//! a live server. The suite asserts the store contract, not a SQL dialect:
//! submission identity, profile scoping, listing, claim and attempt semantics,
//! computed/complete/fail/cancel transitions, sealed-hypothesis round-trip, and
//! profile-scoped research memory.
//!
//! The suite uses a process-local token in every profile, run identity, and
//! hypothesis premise so two invocations against the same Postgres database do
//! not collide. A sealed hypothesis is immutable in both dialects, so the suite
//! never tries to delete one.

use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunState, StageName, StageProgress, StageStatus,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::postgres::{PgHypothesisStore, PgRunStore};
use quantrule_evidence_service::research::HypothesisStore;
use quantrule_evidence_service::runs::{
    CancelOutcome, ClaimOutcome, NewRun, RunStore, SubmitOutcome, derive,
};
use quantrule_evidence_service::sqlite::{SqliteHypothesisStore, SqliteRunStore};
use quantrule_research::{CostModel, Hypothesis, Partition, SealedHypothesis, Split};
use quantrule_runtime::adapters::postgres::{
    PgDatabase, apply_migrations_in as apply_postgres_migrations,
};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

/// A value unique to one invocation within this process.
fn token() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn profile(name: &str, token: &str) -> ProfileId {
    ProfileId::parse(&format!("conf-{name}-{token}")).expect("a conformance profile id")
}

fn new_run(token: &str, name: &str, submitted_at: &str) -> NewRun {
    let data_version = Digest::sha256(format!("data-{token}-{name}").as_bytes());
    let config_hash = Digest::sha256(format!("config-{token}-{name}").as_bytes());
    let image_digest = Digest::sha256(format!("image-{token}-{name}").as_bytes());
    NewRun {
        run_id: derive(&data_version, &config_hash, "M1.0", &image_digest),
        config_id: "cfg_conformance".to_owned(),
        data_version,
        config_hash,
        model_version: "M1.0".to_owned(),
        image_digest,
        owner: "conformance".to_owned(),
        submitted_at: submitted_at.to_owned(),
    }
}

fn stages(evidence: StageStatus, model: StageStatus) -> Vec<StageProgress> {
    [
        (StageName::Data, StageStatus::Done),
        (StageName::Model, model),
        (StageName::Backtest, StageStatus::Pending),
        (StageName::Evidence, evidence),
    ]
    .into_iter()
    .map(|(name, state)| StageProgress {
        name,
        state,
        elapsed_ms: None,
    })
    .collect()
}

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

fn done_stages() -> Vec<StageProgress> {
    computed_stages()
        .into_iter()
        .map(|stage| StageProgress {
            state: StageStatus::Done,
            ..stage
        })
        .collect()
}

fn failed_stages() -> Vec<StageProgress> {
    stages(StageStatus::Pending, StageStatus::Failed)
}

fn artifacts() -> Vec<ArtifactRef> {
    vec![
        ArtifactRef {
            name: "execution/fills.csv".to_owned(),
            bytes: 12,
            sha256: Digest::sha256(b"conformance-fills"),
        },
        ArtifactRef {
            name: "results.json".to_owned(),
            bytes: 32,
            sha256: Digest::sha256(b"conformance-results"),
        },
    ]
}

async fn run_conformance(store: &dyn RunStore, token: &str) {
    let alpha = profile("alpha", token);
    let beta = profile("beta", token);
    let gamma = profile("gamma", token);
    let first = new_run(token, "first", "2026-11-15T00:00:00Z");

    let created = store
        .submit(alpha.clone(), first.clone())
        .await
        .expect("the first submission");
    assert!(created.created(), "the first submission was not created");
    let record = created.record().clone();
    assert_eq!(record.status, RunState::Queued);
    assert_eq!(record.attempts, 0);
    assert_eq!(record.stages.len(), 4);
    assert!(
        record
            .stages
            .iter()
            .all(|stage| stage.state == StageStatus::Pending),
        "a queued run starts with every stage pending"
    );
    assert_eq!(record.owner, "conformance");
    assert_eq!(record.model_version, "M1.0");
    assert_eq!(record.digest, None);
    assert_eq!(record.error, None);
    let id = record.run_id.to_string();

    let existing = store
        .submit(alpha.clone(), first.clone())
        .await
        .expect("the repeated submission");
    assert!(!existing.created(), "a repeated submission created a run");
    assert_eq!(existing.record(), &record);

    let shared = store
        .submit(beta.clone(), first)
        .await
        .expect("the shared submission");
    assert!(
        matches!(shared, SubmitOutcome::Existing(_)),
        "a second profile created a new run"
    );
    assert_eq!(shared.record().run_id, record.run_id);

    assert_eq!(
        store
            .get(alpha.clone(), id.clone())
            .await
            .expect("alpha get"),
        Some(record.clone())
    );
    assert_eq!(
        store.get(beta.clone(), id.clone()).await.expect("beta get"),
        Some(record.clone())
    );
    assert_eq!(
        store
            .get(gamma.clone(), id.clone())
            .await
            .expect("gamma get"),
        None
    );
    assert_eq!(
        store
            .get(alpha.clone(), "missing".to_owned())
            .await
            .expect("the missing run"),
        None
    );
    assert_eq!(
        store.get_any(id.clone()).await.expect("the worker read"),
        Some(record.clone())
    );
    assert_eq!(
        store
            .get_any("missing".to_owned())
            .await
            .expect("the missing worker read"),
        None
    );

    assert_eq!(
        store
            .list(alpha.clone(), None, 50)
            .await
            .expect("alpha list"),
        vec![record.summary()]
    );
    assert_eq!(
        store.list(beta.clone(), None, 50).await.expect("beta list"),
        vec![record.summary()]
    );
    assert!(
        store
            .list(gamma.clone(), None, 50)
            .await
            .expect("gamma list")
            .is_empty()
    );
    assert_eq!(
        store
            .list(alpha.clone(), Some(RunState::Queued), 50)
            .await
            .expect("queued list"),
        vec![record.summary()]
    );
    assert!(
        store
            .list(alpha.clone(), Some(RunState::Succeeded), 50)
            .await
            .expect("succeeded list")
            .is_empty()
    );

    let second = new_run(token, "second", "2026-11-15T00:00:01Z");
    let second = store
        .submit(alpha.clone(), second)
        .await
        .expect("the second run");
    assert!(second.created());
    let second_id = second.record().run_id.to_string();
    let listed = store.list(alpha.clone(), None, 50).await.expect("the list");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].run_id, second.record().run_id);
    assert_eq!(listed[0].submitted_at, "2026-11-15T00:00:01Z");
    assert_eq!(listed[1].run_id, record.run_id);
    assert_eq!(
        store
            .list(alpha.clone(), None, 1)
            .await
            .expect("the limit")
            .len(),
        1,
        "the list limit was ignored"
    );

    let ClaimOutcome::Claimed(claimed) = store.claim(id.clone(), false).await.expect("claim")
    else {
        panic!("a queued run was not claimed");
    };
    assert_eq!(claimed.status, RunState::Running);
    assert_eq!(claimed.attempts, 1);

    let ClaimOutcome::Running(running) = store.claim(id.clone(), false).await.expect("live claim")
    else {
        panic!("a live delivery was reclaimed");
    };
    assert_eq!(
        running.attempts, 1,
        "a live delivery changed the attempt count"
    );

    let ClaimOutcome::Claimed(reclaimed) = store
        .claim(id.clone(), true)
        .await
        .expect("the first redelivery")
    else {
        panic!("a redelivered run was not reclaimed");
    };
    assert_eq!(reclaimed.attempts, 2);
    let ClaimOutcome::Claimed(reclaimed) = store
        .claim(id.clone(), true)
        .await
        .expect("the second redelivery")
    else {
        panic!("the second redelivery was not reclaimed");
    };
    assert_eq!(reclaimed.attempts, 3);
    let ClaimOutcome::Exhausted(spent) = store
        .claim(id.clone(), true)
        .await
        .expect("the spent budget")
    else {
        panic!("the retry budget was not exhausted");
    };
    assert_eq!(spent.attempts, 3);

    let digest = Digest::sha256(format!("digest-{token}").as_bytes());
    let artifact_list = artifacts();
    let computed = store
        .computed(
            id.clone(),
            digest.clone(),
            artifact_list.clone(),
            computed_stages(),
        )
        .await
        .expect("the computed write");
    assert_eq!(computed.status, RunState::Running);
    assert_eq!(computed.digest, Some(digest.clone()));
    assert_eq!(computed.artifacts, artifact_list);
    assert_eq!(computed.stages, computed_stages());

    let ClaimOutcome::Running(computed_live) = store
        .claim(id.clone(), true)
        .await
        .expect("the computed claim")
    else {
        panic!("a computed run was reclaimed by the worker");
    };
    assert_eq!(computed_live.digest, Some(digest.clone()));

    let completed = store
        .complete(
            id.clone(),
            digest.clone(),
            artifact_list.clone(),
            done_stages(),
        )
        .await
        .expect("the completion");
    assert_eq!(completed.status, RunState::Succeeded);
    assert_eq!(completed.digest, Some(digest.clone()));
    assert_eq!(completed.error, None);
    assert_eq!(completed.artifacts, artifact_list);
    assert!(
        completed
            .stages
            .iter()
            .all(|stage| stage.state == StageStatus::Done),
        "a completed run left a stage unfinished"
    );

    let ClaimOutcome::Terminal(terminal) = store
        .claim(id.clone(), false)
        .await
        .expect("the terminal claim")
    else {
        panic!("a succeeded run was claimed");
    };
    assert_eq!(terminal.status, RunState::Succeeded);
    let CancelOutcome::Conflict(conflict) = store
        .cancel(alpha.clone(), id.clone())
        .await
        .expect("the terminal cancel")
    else {
        panic!("a succeeded run was cancelled");
    };
    assert_eq!(conflict.status, RunState::Succeeded);

    let unchanged = store
        .complete(
            id.clone(),
            Digest::sha256(b"a different digest"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("the late completion");
    assert_eq!(unchanged.status, RunState::Succeeded);
    assert_eq!(unchanged.digest, Some(digest));
    let unchanged = store
        .fail(id.clone(), "after success".to_owned(), Vec::new())
        .await
        .expect("the late failure");
    assert_eq!(unchanged.status, RunState::Succeeded);
    assert_eq!(unchanged.error, None);

    let CancelOutcome::Cancelled(cancelled) = store
        .cancel(alpha.clone(), second_id.clone())
        .await
        .expect("the cancel")
    else {
        panic!("a queued run was not cancelled");
    };
    assert_eq!(cancelled.status, RunState::Cancelled);
    let CancelOutcome::Conflict(_) = store
        .cancel(alpha.clone(), second_id.clone())
        .await
        .expect("the second cancel")
    else {
        panic!("a cancelled run was cancelled again");
    };
    let ClaimOutcome::Terminal(terminal) = store
        .claim(second_id.clone(), false)
        .await
        .expect("the cancelled claim")
    else {
        panic!("a cancelled run was claimed");
    };
    assert_eq!(terminal.status, RunState::Cancelled);
    let CancelOutcome::Missing = store
        .cancel(beta.clone(), second_id.clone())
        .await
        .expect("the other profile cancel")
    else {
        panic!("another profile cancelled a run it did not submit");
    };
    assert_eq!(
        store
            .get(beta.clone(), second_id.clone())
            .await
            .expect("the other profile get"),
        None
    );

    let third = new_run(token, "third", "2026-11-15T00:00:02Z");
    let third = store
        .submit(alpha.clone(), third)
        .await
        .expect("the third run");
    let third_id = third.record().run_id.to_string();
    let ClaimOutcome::Claimed(_) = store
        .claim(third_id.clone(), false)
        .await
        .expect("the third claim")
    else {
        panic!("the third run was not claimed");
    };
    let failed = store
        .fail(
            third_id.clone(),
            "the model stage failed".to_owned(),
            failed_stages(),
        )
        .await
        .expect("the failure");
    assert_eq!(failed.status, RunState::Failed);
    assert_eq!(failed.error.as_deref(), Some("the model stage failed"));
    assert_eq!(failed.digest, None);
    assert_eq!(failed.stages, failed_stages());
    let unchanged = store
        .complete(
            third_id.clone(),
            Digest::sha256(b"late"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("the late completion");
    assert_eq!(unchanged.status, RunState::Failed);
    assert_eq!(unchanged.digest, None);

    assert!(matches!(
        store
            .cancel(alpha.clone(), "missing".to_owned())
            .await
            .expect("the missing cancel"),
        CancelOutcome::Missing
    ));
    assert!(matches!(
        store
            .claim("missing".to_owned(), false)
            .await
            .expect("the missing claim"),
        ClaimOutcome::Missing
    ));
    assert!(
        store
            .computed(
                "missing".to_owned(),
                Digest::sha256(b"missing"),
                Vec::new(),
                Vec::new()
            )
            .await
            .is_err(),
        "a computed write to an unknown run succeeded"
    );
    assert!(
        store
            .fail("missing".to_owned(), "missing".to_owned(), Vec::new())
            .await
            .is_err(),
        "a failure write to an unknown run succeeded"
    );
}

fn sealed(token: &str) -> SealedHypothesis {
    let split = Split::new(
        Partition::new("dev", 100, 200).expect("the development partition"),
        Partition::new("val", 200, 300).expect("the validation partition"),
    )
    .expect("the split");
    Hypothesis::new(
        format!("a retest holds {token}"),
        "strategy/retest.toml",
        "sha256:abc",
        CostModel::new(1.0, 0.0, 0.0).expect("the costs"),
        split,
        "expectancy above zero",
    )
    .expect("the hypothesis")
    .seal("2026-11-15T00:00:00Z")
    .expect("the seal")
}

async fn hypothesis_conformance(store: &dyn HypothesisStore, token: &str) {
    let alpha = profile("alpha", token);
    let beta = profile("beta", token);
    let sealed = sealed(token);
    let fingerprint = sealed.fingerprint().to_owned();

    assert_eq!(
        store
            .load(alpha.clone(), fingerprint.clone())
            .await
            .expect("the absent read"),
        None
    );
    store
        .insert(alpha.clone(), sealed.clone())
        .await
        .expect("the insert");
    assert_eq!(
        store
            .load(alpha.clone(), fingerprint.clone())
            .await
            .expect("the read"),
        Some(sealed.clone())
    );
    assert_eq!(
        store
            .load(beta.clone(), fingerprint.clone())
            .await
            .expect("the other profile read"),
        None
    );
    assert_eq!(
        store
            .load(alpha.clone(), "missing".to_owned())
            .await
            .expect("the unknown fingerprint"),
        None
    );

    store
        .insert(beta.clone(), sealed.clone())
        .await
        .expect("the other profile insert");
    assert_eq!(
        store
            .load(beta, fingerprint)
            .await
            .expect("the other profile read"),
        Some(sealed)
    );
}

#[tokio::test]
async fn the_sqlite_run_store_conforms() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    run_conformance(&SqliteRunStore::new(database), &token()).await;
}

#[tokio::test]
async fn the_sqlite_hypothesis_store_conforms() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    hypothesis_conformance(&SqliteHypothesisStore::new(database), &token()).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_stores_conform() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_postgres_migrations(database.pool(), "runs", &migrations::available())
        .await
        .expect("the evidence migrations");
    let token = token();
    run_conformance(&PgRunStore::new(database.pool().clone()), &token).await;
    hypothesis_conformance(&PgHypothesisStore::new(database.pool().clone()), &token).await;
}
