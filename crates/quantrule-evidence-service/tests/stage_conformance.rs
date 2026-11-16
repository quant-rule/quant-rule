//! The shared `StageStore` conformance suite.
//!
//! The same body runs against the memory store and the SQLite store, and —
//! when `QR_TEST_DATABASE_URL` names a disposable Postgres — the Postgres
//! store. The suite asserts the store contract, not a SQL dialect: content
//! addressing, idempotent creation, profile scoping, listing order, claim and
//! attempt semantics, complete/fail/cancel transitions, and artifact ordering.
//!
//! A separate SQLite case proves the evidence boundary: a stage row created in
//! the `stage` schema is invisible to the `runs` store, so a client cannot read
//! a diagnostic stage as evidence.

use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, StageArtifact, StageInput, StageKind, StageProvenance,
    StageState,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::runs::RunStore;
use quantrule_evidence_service::sqlite::SqliteRunStore;
use quantrule_evidence_service::stage::{
    MemoryStages, NewStageRun, PgStageStore, SqliteStageStore, StageCancelOutcome,
    StageClaimOutcome, StageCreateOutcome, StageStore,
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
    ProfileId::parse(&format!("stage-{name}-{token}")).expect("a conformance profile id")
}

fn digest(seed: String) -> Digest {
    Digest::sha256(seed.as_bytes())
}

fn new_run(token: &str, name: &str, kind: StageKind, submitted_at: &str) -> NewStageRun {
    let data_version = digest(format!("data-{token}-{name}"));
    let provenance = StageProvenance {
        data_version: data_version.clone(),
        config_hash: digest(format!("config-{token}-{name}")),
        model_version: "M1.0".to_owned(),
        image_digest: digest(format!("image-{token}-{name}")),
    };
    let input = match kind {
        StageKind::Model => StageInput::DataVersion { data_version },
        StageKind::Strategy => StageInput::ModelArtifact {
            model_artifact_id: digest(format!("model-{token}-{name}")),
        },
        StageKind::Portfolio => StageInput::DecisionsArtifact {
            decisions_artifact_id: digest(format!("decisions-{token}-{name}")),
        },
    };
    NewStageRun::new(
        kind,
        "cfg_conformance".to_owned(),
        input,
        provenance,
        "conformance".to_owned(),
        submitted_at.to_owned(),
    )
}

fn artifacts() -> StageArtifact {
    StageArtifact::from_files(vec![
        ArtifactRef {
            name: "summary.txt".to_owned(),
            bytes: 12,
            sha256: Digest::sha256(b"conformance-summary"),
        },
        ArtifactRef {
            name: "decisions.ndjson".to_owned(),
            bytes: 32,
            sha256: Digest::sha256(b"conformance-decisions"),
        },
    ])
    .expect("the conformance artifact")
}

async fn run_conformance(store: &dyn StageStore, token: &str) {
    let alpha = profile("alpha", token);
    let beta = profile("beta", token);
    let gamma = profile("gamma", token);
    let first = new_run(token, "first", StageKind::Model, "2026-11-17T00:00:00Z");

    let created = store
        .create(alpha.clone(), first.clone())
        .await
        .expect("the first submission");
    assert!(created.created(), "the first stage was not created");
    let record = created.record().clone();
    assert_eq!(record.state, StageState::Queued);
    assert_eq!(record.attempts, 0);
    assert_eq!(record.artifact, None);
    assert_eq!(record.error, None);
    assert_eq!(record.kind, StageKind::Model);
    assert_eq!(record.owner, "conformance");
    assert_eq!(record.provenance.model_version, "M1.0");
    assert!(!record.summary().evidence);
    assert!(!record.detail().evidence);
    let id = record.stage_run_id.to_string();

    let existing = store
        .create(alpha.clone(), first.clone())
        .await
        .expect("the repeated submission");
    assert!(!existing.created(), "a repeated submission created a stage");
    assert_eq!(existing.record(), &record);

    let shared = store
        .create(beta.clone(), first)
        .await
        .expect("the shared submission");
    assert!(
        matches!(shared, StageCreateOutcome::Existing(_)),
        "a second profile created a new stage"
    );
    assert_eq!(shared.record().stage_run_id, record.stage_run_id);

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
            .expect("the missing stage"),
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
    assert!(
        store
            .list(gamma.clone(), None, 50)
            .await
            .expect("gamma list")
            .is_empty()
    );
    assert_eq!(
        store
            .list(alpha.clone(), Some(StageState::Queued), 50)
            .await
            .expect("the queued list"),
        vec![record.summary()]
    );
    assert!(
        store
            .list(alpha.clone(), Some(StageState::Succeeded), 50)
            .await
            .expect("the succeeded list")
            .is_empty()
    );

    let second = new_run(token, "second", StageKind::Strategy, "2026-11-17T00:00:01Z");
    let second = store
        .create(alpha.clone(), second)
        .await
        .expect("the second stage");
    assert!(second.created());
    assert_eq!(second.record().kind, StageKind::Strategy);
    let second_id = second.record().stage_run_id.to_string();
    let listed = store.list(alpha.clone(), None, 50).await.expect("the list");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].stage_run_id, second.record().stage_run_id);
    assert_eq!(listed[1].stage_run_id, record.stage_run_id);
    assert_eq!(
        store
            .list(alpha.clone(), None, 1)
            .await
            .expect("the limit")
            .len(),
        1,
        "the list limit was ignored"
    );

    let StageClaimOutcome::Claimed(claimed) = store.claim(id.clone(), false).await.expect("claim")
    else {
        panic!("a queued stage was not claimed");
    };
    assert_eq!(claimed.state, StageState::Running);
    assert_eq!(claimed.attempts, 1);

    let StageClaimOutcome::Running(running) = store.claim(id.clone(), false).await.expect("live")
    else {
        panic!("a live delivery was reclaimed");
    };
    assert_eq!(
        running.attempts, 1,
        "a live delivery changed the attempt count"
    );

    let StageClaimOutcome::Claimed(reclaimed) = store
        .claim(id.clone(), true)
        .await
        .expect("the first redelivery")
    else {
        panic!("a redelivered stage was not reclaimed");
    };
    assert_eq!(reclaimed.attempts, 2);
    let StageClaimOutcome::Claimed(reclaimed) = store
        .claim(id.clone(), true)
        .await
        .expect("the second redelivery")
    else {
        panic!("the second redelivery was not reclaimed");
    };
    assert_eq!(reclaimed.attempts, 3);
    let StageClaimOutcome::Exhausted(spent) = store
        .claim(id.clone(), true)
        .await
        .expect("the spent budget")
    else {
        panic!("the retry budget was not exhausted");
    };
    assert_eq!(spent.attempts, 3);

    let artifact = artifacts();
    let completed = store
        .complete(id.clone(), artifact.clone())
        .await
        .expect("the completion");
    assert_eq!(completed.state, StageState::Succeeded);
    assert_eq!(completed.attempts, 3);
    assert_eq!(completed.error, None);
    assert_eq!(completed.artifact, Some(artifact.clone()));
    assert_eq!(
        completed
            .artifact
            .as_ref()
            .expect("the artifact")
            .files
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["decisions.ndjson", "summary.txt"],
        "the artifact files are not ordered by name"
    );
    assert_eq!(
        completed.summary().artifact_id,
        Some(artifact.artifact_id.clone())
    );

    let StageClaimOutcome::Terminal(terminal) = store
        .claim(id.clone(), false)
        .await
        .expect("terminal claim")
    else {
        panic!("a succeeded stage was claimed");
    };
    assert_eq!(terminal.state, StageState::Succeeded);
    let StageCancelOutcome::Conflict(conflict) = store
        .cancel(alpha.clone(), id.clone())
        .await
        .expect("the terminal cancel")
    else {
        panic!("a succeeded stage was cancelled");
    };
    assert_eq!(conflict.state, StageState::Succeeded);

    let unchanged = store
        .complete(
            id.clone(),
            StageArtifact::from_files(vec![ArtifactRef {
                name: "other.json".to_owned(),
                bytes: 1,
                sha256: Digest::sha256(b"other"),
            }])
            .expect("the other artifact"),
        )
        .await
        .expect("the late completion");
    assert_eq!(unchanged.state, StageState::Succeeded);
    assert_eq!(unchanged.artifact, Some(artifact.clone()));
    let unchanged = store
        .fail(id.clone(), "after success".to_owned())
        .await
        .expect("the late failure");
    assert_eq!(unchanged.state, StageState::Succeeded);
    assert_eq!(unchanged.error, None);

    let StageCancelOutcome::Cancelled(cancelled) = store
        .cancel(alpha.clone(), second_id.clone())
        .await
        .expect("the cancel")
    else {
        panic!("a queued stage was not cancelled");
    };
    assert_eq!(cancelled.state, StageState::Cancelled);
    let StageCancelOutcome::Conflict(_) = store
        .cancel(alpha.clone(), second_id.clone())
        .await
        .expect("the second cancel")
    else {
        panic!("a cancelled stage was cancelled again");
    };
    let StageClaimOutcome::Terminal(terminal) = store
        .claim(second_id.clone(), false)
        .await
        .expect("the cancelled claim")
    else {
        panic!("a cancelled stage was claimed");
    };
    assert_eq!(terminal.state, StageState::Cancelled);
    let StageCancelOutcome::Missing = store
        .cancel(beta.clone(), second_id.clone())
        .await
        .expect("the other profile cancel")
    else {
        panic!("another profile cancelled a stage it did not submit");
    };
    assert_eq!(
        store
            .get(beta.clone(), second_id.clone())
            .await
            .expect("the other profile get"),
        None
    );

    let third = new_run(token, "third", StageKind::Portfolio, "2026-11-17T00:00:02Z");
    let third = store
        .create(alpha.clone(), third)
        .await
        .expect("the third stage");
    let third_id = third.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(_) = store
        .claim(third_id.clone(), false)
        .await
        .expect("the third claim")
    else {
        panic!("the third stage was not claimed");
    };
    let failed = store
        .fail(third_id.clone(), "the portfolio stage failed".to_owned())
        .await
        .expect("the failure");
    assert_eq!(failed.state, StageState::Failed);
    assert_eq!(failed.error.as_deref(), Some("the portfolio stage failed"));
    assert_eq!(failed.artifact, None);
    let unchanged = store
        .complete(third_id.clone(), artifacts())
        .await
        .expect("the late completion");
    assert_eq!(unchanged.state, StageState::Failed);
    assert_eq!(unchanged.artifact, None);

    assert!(matches!(
        store
            .cancel(alpha.clone(), "missing".to_owned())
            .await
            .expect("the missing cancel"),
        StageCancelOutcome::Missing
    ));
    assert!(matches!(
        store
            .claim("missing".to_owned(), false)
            .await
            .expect("the missing claim"),
        StageClaimOutcome::Missing
    ));
    assert!(
        store
            .complete("missing".to_owned(), artifacts())
            .await
            .is_err(),
        "a completion of an unknown stage succeeded"
    );
    assert!(
        store
            .fail("missing".to_owned(), "missing".to_owned())
            .await
            .is_err(),
        "a failure of an unknown stage succeeded"
    );
}

async fn baseline(database: &SqliteDatabase) -> SqliteStageStore {
    apply_migrations_in(database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence migrations");
    SqliteStageStore::new(database.clone())
}

#[tokio::test]
async fn the_memory_stage_store_conforms() {
    run_conformance(&MemoryStages::default(), &token()).await;
}

#[tokio::test]
async fn the_sqlite_stage_store_conforms() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let store = baseline(&database).await;
    run_conformance(&store, &token()).await;
}

#[tokio::test]
async fn a_stage_row_is_not_readable_as_a_run() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let stage_store = baseline(&database).await;
    let run_store = SqliteRunStore::new(database);
    let token = token();
    let profile = profile("alpha", &token);
    let created = stage_store
        .create(
            profile.clone(),
            new_run(&token, "boundary", StageKind::Model, "2026-11-17T00:00:00Z"),
        )
        .await
        .expect("the stage");
    let id = created.record().stage_run_id.to_string();
    assert!(
        stage_store
            .get_any(id.clone())
            .await
            .expect("the stage read")
            .is_some(),
        "the stage row is missing"
    );
    assert_eq!(
        run_store
            .get(profile.clone(), id.clone())
            .await
            .expect("run get"),
        None,
        "the run store read a stage row"
    );
    assert_eq!(
        run_store.get_any(id).await.expect("run get_any"),
        None,
        "the run store read a stage row without a profile"
    );
    assert!(
        run_store
            .list(profile, None, 50)
            .await
            .expect("run list")
            .is_empty(),
        "the run list contained a stage row"
    );
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_stage_store_conforms() {
    use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in as apply};

    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply(database.pool(), "runs", &migrations::available())
        .await
        .expect("the evidence migrations");
    run_conformance(&PgStageStore::new(database.pool().clone()), &token()).await;
}
