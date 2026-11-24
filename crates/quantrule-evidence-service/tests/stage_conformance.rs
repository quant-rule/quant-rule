//! The shared `StageStore` conformance suite.
//!
//! The same body runs against the memory store and, when
//! `QR_TEST_DATABASE_URL` names a disposable Postgres, the Postgres store. The suite asserts the store contract, not a SQL dialect: content
//! addressing, idempotent creation, profile scoping, listing order, claim and
//! attempt semantics, complete/fail/cancel transitions, and artifact ordering.

use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, StageArtifact, StageInput, StageKind, StageProvenance,
    StageState,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::stage::{
    MemoryStages, NewStageRun, PgStageStore, StageCancelOutcome, StageClaimOutcome,
    StageCreateOutcome, StageHeartbeatOutcome, StageStore,
};

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
            .list(alpha.clone(), None, None, 50)
            .await
            .expect("alpha list"),
        vec![record.summary()]
    );
    assert!(
        store
            .list(gamma.clone(), None, None, 50)
            .await
            .expect("gamma list")
            .is_empty()
    );
    assert_eq!(
        store
            .list(alpha.clone(), None, Some(StageState::Queued), 50)
            .await
            .expect("the queued list"),
        vec![record.summary()]
    );
    assert!(
        store
            .list(alpha.clone(), None, Some(StageState::Succeeded), 50)
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
    let listed = store
        .list(alpha.clone(), None, None, 50)
        .await
        .expect("the list");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].stage_run_id, second.record().stage_run_id);
    assert_eq!(listed[1].stage_run_id, record.stage_run_id);
    let kind_filtered = store
        .list(alpha.clone(), Some(StageKind::Strategy), None, 50)
        .await
        .expect("the kind filter");
    assert_eq!(kind_filtered.len(), 1);
    assert_eq!(kind_filtered[0].stage_run_id, second.record().stage_run_id);
    assert_eq!(
        store
            .list(alpha.clone(), Some(StageKind::Portfolio), None, 50)
            .await
            .expect("the empty kind filter")
            .len(),
        0
    );
    assert_eq!(
        store
            .list(alpha.clone(), None, None, 1)
            .await
            .expect("the limit")
            .len(),
        1,
        "the list limit was ignored"
    );

    let StageClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), "conformance-worker".to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("a queued stage was not claimed");
    };
    assert_eq!(claimed.state, StageState::Running);
    assert_eq!(claimed.attempts, 1);
    assert_eq!(claimed.lease_term, 1);

    // A live lease is never stolen, even by a second worker or a broker
    // redelivery.
    let StageClaimOutcome::Running(running) = store
        .claim(id.clone(), "other-worker".to_owned(), None)
        .await
        .expect("live")
    else {
        panic!("a live delivery was reclaimed");
    };
    assert_eq!(
        running.attempts, 1,
        "a live delivery changed the attempt count"
    );
    let StageHeartbeatOutcome::Renewed(renewed) = store
        .renew(id.clone(), "conformance-worker".to_owned(), 1)
        .await
        .expect("renew")
    else {
        panic!("a live lease was not renewed");
    };
    assert_eq!(renewed.lease_term, 1);
    let StageHeartbeatOutcome::Stale = store
        .renew(id.clone(), "other-worker".to_owned(), 1)
        .await
        .expect("stale renew")
    else {
        panic!("another holder renewed the lease");
    };

    // The same worker's own continuation is a new attempt under a new term.
    let StageClaimOutcome::Claimed(reclaimed) = store
        .claim(id.clone(), "conformance-worker".to_owned(), Some(1))
        .await
        .expect("the first continuation")
    else {
        panic!("the worker's own continuation was refused");
    };
    assert_eq!(reclaimed.attempts, 2);
    assert_eq!(reclaimed.lease_term, 2);
    let StageClaimOutcome::Claimed(reclaimed) = store
        .claim(id.clone(), "conformance-worker".to_owned(), Some(2))
        .await
        .expect("the second continuation")
    else {
        panic!("the second continuation was refused");
    };
    assert_eq!(reclaimed.attempts, 3);
    assert_eq!(reclaimed.lease_term, 3);
    // The third attempt is still live, so a duplicate delivery is refused
    // without stealing it; the expired-lease exhaustion path is covered by the
    // memory store's expiry test.
    let StageClaimOutcome::Running(spent) = store
        .claim(id.clone(), "conformance-worker".to_owned(), Some(3))
        .await
        .expect("the live spent budget")
    else {
        panic!("a live stage was reclaimed after the retry budget");
    };
    assert_eq!(spent.attempts, 3);

    let artifact = artifacts();
    let completed = store
        .complete(id.clone(), 3, artifact.clone())
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
    let found = store
        .find_by_artifact(
            alpha.clone(),
            StageKind::Model,
            artifact.artifact_id.clone(),
        )
        .await
        .expect("the artifact lookup")
        .expect("the succeeded model stage");
    assert_eq!(found.stage_run_id.to_string(), id);
    assert_eq!(
        store
            .find_by_artifact(
                alpha.clone(),
                StageKind::Strategy,
                artifact.artifact_id.clone(),
            )
            .await
            .expect("the wrong-kind lookup"),
        None
    );
    assert_eq!(
        store
            .find_by_artifact(
                gamma.clone(),
                StageKind::Model,
                artifact.artifact_id.clone(),
            )
            .await
            .expect("the other profile lookup"),
        None
    );
    assert_eq!(
        store
            .find_by_artifact(
                alpha.clone(),
                StageKind::Model,
                Digest::sha256(b"missing-artifact"),
            )
            .await
            .expect("the missing artifact lookup"),
        None
    );

    let StageClaimOutcome::Terminal(terminal) = store
        .claim(id.clone(), "conformance-worker".to_owned(), None)
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
            3,
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
        .fail(id.clone(), 3, "after success".to_owned())
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
        .claim(second_id.clone(), "conformance-worker".to_owned(), None)
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
    let StageClaimOutcome::Claimed(third_claimed) = store
        .claim(third_id.clone(), "conformance-worker".to_owned(), None)
        .await
        .expect("the third claim")
    else {
        panic!("the third stage was not claimed");
    };
    let failed = store
        .fail(
            third_id.clone(),
            third_claimed.lease_term,
            "the portfolio stage failed".to_owned(),
        )
        .await
        .expect("the failure");
    assert_eq!(failed.state, StageState::Failed);
    assert_eq!(failed.error.as_deref(), Some("the portfolio stage failed"));
    assert_eq!(failed.artifact, None);
    let unchanged = store
        .complete(third_id.clone(), third_claimed.lease_term, artifacts())
        .await
        .expect("the late completion");
    assert_eq!(unchanged.state, StageState::Failed);
    assert_eq!(unchanged.artifact, None);
    assert_eq!(
        store
            .find_by_artifact(
                alpha.clone(),
                StageKind::Portfolio,
                artifact.artifact_id.clone(),
            )
            .await
            .expect("the failed-stage lookup"),
        None,
        "a failed terminal stage is not an artifact source"
    );

    assert!(matches!(
        store
            .cancel(alpha.clone(), "missing".to_owned())
            .await
            .expect("the missing cancel"),
        StageCancelOutcome::Missing
    ));
    assert!(matches!(
        store
            .claim("missing".to_owned(), "conformance-worker".to_owned(), None)
            .await
            .expect("the missing claim"),
        StageClaimOutcome::Missing
    ));
    assert!(
        store
            .complete("missing".to_owned(), 1, artifacts())
            .await
            .is_err(),
        "a completion of an unknown stage succeeded"
    );
    assert!(
        store
            .fail("missing".to_owned(), 1, "missing".to_owned())
            .await
            .is_err(),
        "a failure of an unknown stage succeeded"
    );

    let fourth = new_run(token, "fourth", StageKind::Model, "2026-11-17T00:00:03Z");
    let fourth = store
        .create(alpha.clone(), fourth)
        .await
        .expect("the fourth stage");
    let fourth_id = fourth.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(fourth_claimed) = store
        .claim(fourth_id.clone(), "conformance-worker".to_owned(), None)
        .await
        .expect("the fourth claim")
    else {
        panic!("the fourth stage was not claimed");
    };
    let unique = StageArtifact::from_files(vec![ArtifactRef {
        name: "unique.json".to_owned(),
        bytes: 6,
        sha256: digest(format!("unique-{token}")),
    }])
    .expect("the unique artifact");
    store
        .complete(fourth_id.clone(), fourth_claimed.lease_term, unique.clone())
        .await
        .expect("the fourth completion");
    let unique_hash = unique.files[0].sha256.clone();

    assert!(
        store
            .delete(beta.clone(), fourth_id.clone())
            .await
            .expect("the other profile delete")
            .is_none(),
        "another profile deleted a stage it did not submit"
    );
    let deletion = store
        .delete(alpha.clone(), fourth_id.clone())
        .await
        .expect("the delete")
        .expect("the deleted stage");
    assert_eq!(deletion.stage_run_id.to_string(), fourth_id);
    assert_eq!(deletion.artifact_hashes, vec![unique_hash.clone()]);
    assert_eq!(
        store
            .get(alpha.clone(), fourth_id)
            .await
            .expect("the deleted get"),
        None
    );
    assert!(
        !store
            .artifact_referenced(unique_hash)
            .await
            .expect("the artifact reference"),
        "a deleted stage still references its artifact"
    );
    assert!(
        store
            .artifact_referenced(artifacts().files[0].sha256.clone())
            .await
            .expect("the shared artifact reference"),
        "the delete removed another stage's artifact reference"
    );
}

#[tokio::test]
async fn the_memory_stage_store_conforms() {
    run_conformance(&MemoryStages::default(), &token()).await;
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
