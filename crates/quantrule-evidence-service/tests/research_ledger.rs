//! The research-ledger conformance suite.
//!
//! The same body runs against the memory store and the SQLite store; the
//! Postgres case is opt-in because it needs a live server. The suite asserts the
//! store contract: profile scoping, idempotent create, an immutable manifest, a
//! single pipeline link, a single human decision, and the seeded baseline and
//! invalidation rows.

use quantrule_api::{
    Digest, HoldoutAccess, IMPROVEMENT_RUN_SCHEMA, ImprovementDecision, ImprovementManifest,
    ImprovementPartition, ImprovementRunState, ImprovementTier, InvalidationRecord, ProfileId,
    RunId,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::research::{
    ImprovementStore, MemoryImprovementStore, PgImprovementStore, SqliteImprovementStore,
    seeded_baselines,
};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("a test profile id")
}

fn manifest(improvement_id: &str) -> ImprovementManifest {
    let mut green_criteria = serde_json::Map::new();
    green_criteria.insert("profit_factor".to_owned(), serde_json::json!(1.1));
    ImprovementManifest {
        schema: IMPROVEMENT_RUN_SCHEMA.to_owned(),
        improvement_id: improvement_id.to_owned(),
        tier: ImprovementTier::Model,
        hypothesis: "the compression expansion holds on the development partition".to_owned(),
        created_utc: "2026-11-17T00:00:00Z".to_owned(),
        changed: ImprovementTier::Model,
        frozen: vec![ImprovementTier::Strategy, ImprovementTier::Portfolio],
        partition: ImprovementPartition::ModelDev,
        data_version: Digest::sha256(b"the test dataset"),
        config_id: "cfg_test".to_owned(),
        criteria_id: None,
        green_criteria,
        holdout_access: HoldoutAccess::None,
        baseline: None,
        code_commit: "c236539f".to_owned(),
        seed: 7,
        command: Some("run submit".to_owned()),
        analytics: Some(true),
    }
}

fn digest(seed: &str) -> Digest {
    Digest::sha256(seed.as_bytes())
}

async fn ledger_conformance(store: &dyn ImprovementStore, owner: &ProfileId) {
    let other = profile(&format!("{}-other", owner.as_str()));
    let record = manifest("MODEL-2026-11-17-01");

    let created = store
        .create(
            owner.clone(),
            record.clone(),
            "2026-11-17T00:00:00Z".to_owned(),
        )
        .await
        .expect("the pre-registration");
    assert!(created.created());
    assert_eq!(created.record().state, ImprovementRunState::Created);

    let repeated = store
        .create(
            owner.clone(),
            record.clone(),
            "2026-11-17T00:00:01Z".to_owned(),
        )
        .await
        .expect("the idempotent create");
    assert!(!repeated.created());
    assert_eq!(repeated.record().created_at, "2026-11-17T00:00:00Z");

    let mut changed = record.clone();
    changed.hypothesis = "a different hypothesis under the same id".to_owned();
    assert!(
        store
            .create(owner.clone(), changed, "2026-11-17T00:00:02Z".to_owned())
            .await
            .is_err(),
        "an existing improvement id must not be rewritten"
    );

    assert_eq!(
        store
            .get(other.clone(), "MODEL-2026-11-17-01".to_owned())
            .await
            .expect("the other profile read"),
        None
    );
    assert!(
        store
            .list(other.clone(), 10)
            .await
            .expect("the other profile list")
            .is_empty()
    );

    assert!(
        store
            .decide(
                owner.clone(),
                "MODEL-2026-11-17-01".to_owned(),
                ImprovementDecision::Accepted,
                "not yet submitted".to_owned(),
                "2026-11-17T00:10:00Z".to_owned(),
            )
            .await
            .is_err(),
        "a decision needs a submitted pipeline run"
    );

    let run_id = RunId(digest("the pipeline run"));
    let canonical = digest("the canonical result");
    let submitted = store
        .link_pipeline_run(
            owner.clone(),
            "MODEL-2026-11-17-01".to_owned(),
            run_id.clone(),
            canonical.clone(),
            "2026-11-17T00:05:00Z".to_owned(),
        )
        .await
        .expect("the pipeline link");
    assert_eq!(submitted.state, ImprovementRunState::Submitted);
    assert_eq!(submitted.pipeline_run_id, Some(run_id));
    assert_eq!(submitted.canonical_digest, Some(canonical));

    assert!(
        store
            .link_pipeline_run(
                owner.clone(),
                "MODEL-2026-11-17-01".to_owned(),
                RunId(digest("another run")),
                digest("another result"),
                "2026-11-17T00:06:00Z".to_owned(),
            )
            .await
            .is_err(),
        "a pipeline link is recorded once"
    );

    assert!(
        store
            .decide(
                owner.clone(),
                "MODEL-2026-11-17-01".to_owned(),
                ImprovementDecision::Accepted,
                "   ".to_owned(),
                "2026-11-17T00:10:00Z".to_owned(),
            )
            .await
            .is_err(),
        "a decision needs a note"
    );

    let decided = store
        .decide(
            owner.clone(),
            "MODEL-2026-11-17-01".to_owned(),
            ImprovementDecision::Accepted,
            "the pre-registered criteria passed".to_owned(),
            "2026-11-17T00:10:00Z".to_owned(),
        )
        .await
        .expect("the human decision");
    assert_eq!(decided.state, ImprovementRunState::Decided);
    assert_eq!(decided.decision, Some(ImprovementDecision::Accepted));
    assert_eq!(
        decided.decision_note.as_deref(),
        Some("the pre-registered criteria passed")
    );

    assert!(
        store
            .decide(
                owner.clone(),
                "MODEL-2026-11-17-01".to_owned(),
                ImprovementDecision::Rejected,
                "a second decision".to_owned(),
                "2026-11-17T00:11:00Z".to_owned(),
            )
            .await
            .is_err(),
        "a decision is recorded once"
    );

    let listed = store.list(owner.clone(), 10).await.expect("the list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].state, ImprovementRunState::Decided);
    assert_eq!(listed[0].decision, Some(ImprovementDecision::Accepted));
}

#[tokio::test]
async fn the_memory_ledger_conforms() {
    let store = MemoryImprovementStore::default();
    ledger_conformance(&store, &profile("memory-ledger")).await;
}

#[tokio::test]
async fn the_memory_baselines_are_profile_scoped_and_append_only() {
    let store = MemoryImprovementStore::default();
    let seeded = seeded_baselines();
    let first = seeded[0].clone();
    for baseline in seeded {
        store.add_baseline(profile("baseline-owner"), baseline);
    }
    let mut replacement = first.clone();
    replacement.scope = "a rewritten scope".to_owned();
    store.add_baseline(profile("baseline-owner"), replacement);
    let rows = store
        .baselines(profile("baseline-owner"))
        .await
        .expect("the baselines");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].accepted_by, "Human");
    assert_eq!(
        rows.iter().find(|row| row.baseline_id == first.baseline_id),
        Some(&first),
        "the first baseline row wins"
    );
    assert!(
        store
            .baselines(profile("another-owner"))
            .await
            .expect("the other baselines")
            .is_empty()
    );
}

#[tokio::test]
async fn the_memory_invalidations_are_append_only() {
    let store = MemoryImprovementStore::default();
    let owner = profile("invalidation-owner");
    let first = InvalidationRecord {
        invalidation_id: "MODEL-2026-11-10-01".to_owned(),
        date: "2026-11-10".to_owned(),
        commit: "worktree".to_owned(),
        removed: "MODEL-2026-11-10-01".to_owned(),
        reason: "the first reason".to_owned(),
        superseded_by: None,
        created_at: "2026-11-10T00:00:00Z".to_owned(),
    };
    let mut replacement = first.clone();
    replacement.reason = "a rewritten reason".to_owned();
    store.add_invalidation(owner.clone(), first.clone());
    store.add_invalidation(owner.clone(), replacement);
    assert_eq!(
        store.invalidations(owner).await.expect("the invalidations"),
        vec![first]
    );
}

#[tokio::test]
async fn the_sqlite_ledger_conforms() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    ledger_conformance(
        &SqliteImprovementStore::new(database),
        &profile("sqlite-ledger"),
    )
    .await;
}

#[tokio::test]
async fn the_sqlite_ledger_reads_the_seeded_registries() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let store = SqliteImprovementStore::new(database);

    let baselines = store
        .baselines(profile("default"))
        .await
        .expect("baselines");
    assert_eq!(baselines.len(), 3);
    assert_eq!(
        baselines
            .iter()
            .map(|record| record.baseline_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "MODEL-2026-11-04-04..05",
            "MODEL-2026-11-04-06..07",
            "MODEL-2026-11-04-10..11"
        ]
    );
    assert!(
        baselines
            .iter()
            .all(|record| record.tier == ImprovementTier::Model
                && record.accepted_by == "Human"
                && record.accepted_on == "2026-11-04")
    );

    let invalidations = store
        .invalidations(profile("default"))
        .await
        .expect("invalidations");
    assert_eq!(invalidations.len(), 8);
    assert!(
        invalidations
            .iter()
            .any(|record| record.invalidation_id.ends_with("SR10.md"))
    );
    assert!(
        invalidations
            .iter()
            .any(|record| record.invalidation_id.ends_with("SR11.md"))
    );

    assert!(
        store
            .baselines(profile("another-profile"))
            .await
            .expect("the other baselines")
            .is_empty()
    );
    assert!(
        store
            .invalidations(profile("another-profile"))
            .await
            .expect("the other invalidations")
            .is_empty()
    );
}

#[tokio::test]
async fn the_sqlite_ledger_is_append_only() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let store = SqliteImprovementStore::new(database.clone());
    let owner = profile("append-only");
    store
        .create(
            owner.clone(),
            manifest("MODEL-2026-11-17-01"),
            "2026-11-17T00:00:00Z".to_owned(),
        )
        .await
        .expect("the pre-registration");

    let manifest_owner = owner.as_str().to_owned();
    let manifest_update = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs SET manifest = '{}'
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![manifest_owner, "MODEL-2026-11-17-01"],
            )
        })
        .await;
    assert!(manifest_update.is_err(), "a manifest is immutable");

    let identity_owner = owner.as_str().to_owned();
    let identity_update = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs
                    SET improvement_id = 'MODEL-2026-11-17-02'
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![identity_owner, "MODEL-2026-11-17-01"],
            )
        })
        .await;
    assert!(
        identity_update.is_err(),
        "the pre-registration identity is immutable"
    );

    let created_at_owner = owner.as_str().to_owned();
    let created_at_update = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs
                    SET created_at = '2026-11-18T00:00:00Z'
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![created_at_owner, "MODEL-2026-11-17-01"],
            )
        })
        .await;
    assert!(
        created_at_update.is_err(),
        "the pre-registration time is immutable"
    );

    let baseline_update = database
        .call(|connection| {
            connection.execute(
                "UPDATE research__baselines SET scope = 'changed'
                  WHERE profile_id = 'default'
                    AND baseline_id = 'MODEL-2026-11-04-04..05'",
                [],
            )
        })
        .await;
    assert!(baseline_update.is_err(), "a baseline is append-only");

    let baseline_delete = database
        .call(|connection| {
            connection.execute(
                "DELETE FROM research__baselines
                  WHERE profile_id = 'default'
                    AND baseline_id = 'MODEL-2026-11-04-04..05'",
                [],
            )
        })
        .await;
    assert!(baseline_delete.is_err(), "a baseline cannot be deleted");

    let invalidation_update = database
        .call(|connection| {
            connection.execute(
                "UPDATE research__invalidations SET reason = 'changed'
                  WHERE profile_id = 'default'
                    AND invalidation_id = 'docs/improvement/2026-11-10/SR10.md'",
                [],
            )
        })
        .await;
    assert!(
        invalidation_update.is_err(),
        "an invalidation is append-only"
    );

    let invalidation_delete = database
        .call(|connection| {
            connection.execute(
                "DELETE FROM research__invalidations
                  WHERE profile_id = 'default'
                    AND invalidation_id = 'docs/improvement/2026-11-10/SR10.md'",
                [],
            )
        })
        .await;
    assert!(
        invalidation_delete.is_err(),
        "an invalidation cannot be deleted"
    );
}

#[tokio::test]
async fn the_sqlite_ledger_records_a_link_once() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let store = SqliteImprovementStore::new(database.clone());
    let owner = profile("link-once");
    store
        .create(
            owner.clone(),
            manifest("MODEL-2026-11-17-01"),
            "2026-11-17T00:00:00Z".to_owned(),
        )
        .await
        .expect("the pre-registration");
    store
        .link_pipeline_run(
            owner.clone(),
            "MODEL-2026-11-17-01".to_owned(),
            RunId(digest("the first run")),
            digest("the first result"),
            "2026-11-17T00:05:00Z".to_owned(),
        )
        .await
        .expect("the pipeline link");

    let owner_id = owner.as_str().to_owned();
    let relink = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs
                    SET pipeline_run_id = ?3
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![
                    owner_id,
                    "MODEL-2026-11-17-01",
                    RunId(digest("the second run")).to_string()
                ],
            )
        })
        .await;
    assert!(relink.is_err(), "a pipeline link is recorded once");
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_ledger_conforms() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = quantrule_runtime::adapters::postgres::PgDatabase::connect(&url)
        .await
        .expect("the Postgres pool");
    quantrule_runtime::adapters::postgres::apply_migrations_in(
        database.pool(),
        "runs",
        &migrations::available(),
    )
    .await
    .expect("the evidence migrations");
    let store = PgImprovementStore::new(database.pool().clone());
    let owner = profile(&format!("pg-ledger-{}", std::process::id()));
    ledger_conformance(&store, &owner).await;

    let baselines = store
        .baselines(profile("default"))
        .await
        .expect("baselines");
    assert_eq!(baselines.len(), 3);
    let invalidations = store
        .invalidations(profile("default"))
        .await
        .expect("invalidations");
    assert_eq!(invalidations.len(), 8);

    let manifest_update = sqlx::query(
        "UPDATE research.improvement_runs SET manifest = '{}'::jsonb
          WHERE profile_id = $1 AND improvement_id = 'MODEL-2026-11-17-01'",
    )
    .bind(owner.as_str())
    .execute(database.pool())
    .await;
    assert!(manifest_update.is_err(), "a manifest is immutable");

    let relink = sqlx::query(
        "UPDATE research.improvement_runs SET pipeline_run_id = $2
          WHERE profile_id = $1 AND improvement_id = 'MODEL-2026-11-17-01'",
    )
    .bind(owner.as_str())
    .bind(RunId(digest("another run")).to_string())
    .execute(database.pool())
    .await;
    assert!(relink.is_err(), "a pipeline link is recorded once");

    let baseline_update = sqlx::query(
        "UPDATE research.baselines SET scope = 'changed'
          WHERE profile_id = 'default'
            AND baseline_id = 'MODEL-2026-11-04-04..05'",
    )
    .execute(database.pool())
    .await;
    assert!(baseline_update.is_err(), "a baseline is append-only");

    let invalidation_delete = sqlx::query(
        "DELETE FROM research.invalidations
          WHERE profile_id = 'default'
            AND invalidation_id = 'docs/improvement/2026-11-10/SR10.md'",
    )
    .execute(database.pool())
    .await;
    assert!(
        invalidation_delete.is_err(),
        "an invalidation cannot be deleted"
    );
}
