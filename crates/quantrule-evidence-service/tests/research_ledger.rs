//! The research-ledger conformance suite.
//!
//! The shared conformance bodies run against the memory and SQLite stores; the
//! Postgres case is opt-in because it needs a live server. This target covers the
//! two-step pipeline link, the accepted-only digest rule, the append-only
//! registries, and the seeded committed rows.

mod research_ledger_support;

use quantrule_api::{ImprovementDecision, ImprovementTier, RunId};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::research::{
    ImprovementStore, MemoryImprovementStore, PgImprovementStore, SqliteImprovementStore,
};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

use research_ledger_support::{
    baseline, digest, invalidation, ledger_conformance, manifest, profile, registry_conformance,
};

#[tokio::test]
async fn the_memory_ledger_conforms() {
    let store = MemoryImprovementStore::default();
    ledger_conformance(&store, &profile("memory-ledger")).await;
}

#[tokio::test]
async fn the_memory_registries_conform() {
    let store = MemoryImprovementStore::default();
    registry_conformance(&store, &profile("memory-registry")).await;
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
async fn the_sqlite_registries_conform() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    registry_conformance(
        &SqliteImprovementStore::new(database),
        &profile("sqlite-registry"),
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
async fn the_sqlite_ledger_requires_a_digest_only_for_acceptance() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let store = SqliteImprovementStore::new(database.clone());
    let owner = profile("acceptance-digest");
    let rejected_id = "MODEL-2026-11-17-01";
    let accepted_id = "MODEL-2026-11-17-02";
    let raw_id = "MODEL-2026-11-17-03";
    for id in [rejected_id, accepted_id, raw_id] {
        store
            .create(
                owner.clone(),
                manifest(id),
                "2026-11-17T00:00:00Z".to_owned(),
            )
            .await
            .expect("the pre-registration");
        store
            .link_run(
                owner.clone(),
                id.to_owned(),
                RunId(digest(id)),
                "2026-11-17T00:05:00Z".to_owned(),
            )
            .await
            .expect("the pipeline link");
    }

    let missing = store
        .decide(
            owner.clone(),
            rejected_id.to_owned(),
            ImprovementDecision::Accepted,
            "accepted without a digest".to_owned(),
            "2026-11-17T00:10:00Z".to_owned(),
        )
        .await
        .expect_err("accepted without a digest");
    assert!(
        missing.to_string().contains("canonical digest"),
        "{missing}"
    );

    let rejected = store
        .decide(
            owner.clone(),
            rejected_id.to_owned(),
            ImprovementDecision::Rejected,
            "rejected without a digest".to_owned(),
            "2026-11-17T00:10:00Z".to_owned(),
        )
        .await
        .expect("a rejected decision without a digest");
    assert_eq!(rejected.canonical_digest, None);

    store
        .link_digest(
            owner.clone(),
            accepted_id.to_owned(),
            digest("the accepted digest"),
        )
        .await
        .expect("the accepted digest");
    let accepted = store
        .decide(
            owner.clone(),
            accepted_id.to_owned(),
            ImprovementDecision::Accepted,
            "accepted after the digest".to_owned(),
            "2026-11-17T00:10:00Z".to_owned(),
        )
        .await
        .expect("an accepted decision after the digest");
    assert!(accepted.canonical_digest.is_some());

    let owner_id = owner.as_str().to_owned();
    let raw_accept = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs
                    SET state = 'decided', decision = 'accepted',
                        decision_note = 'accepted without a digest',
                        decided_at = '2026-11-17T00:10:00Z'
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![owner_id, "MODEL-2026-11-17-03"],
            )
        })
        .await;
    assert!(
        raw_accept.is_err(),
        "the schema refuses accepted without a digest"
    );
}

#[tokio::test]
async fn the_sqlite_ledger_records_the_link_and_digest_once() {
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
        .link_run(
            owner.clone(),
            "MODEL-2026-11-17-01".to_owned(),
            RunId(digest("the first run")),
            "2026-11-17T00:05:00Z".to_owned(),
        )
        .await
        .expect("the pipeline link");

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

    let submitted_at_owner = owner.as_str().to_owned();
    let resubmit = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs
                    SET submitted_at = '2026-11-17T00:07:00Z'
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![submitted_at_owner, "MODEL-2026-11-17-01"],
            )
        })
        .await;
    assert!(resubmit.is_err(), "the submission time is immutable");

    store
        .link_digest(
            owner.clone(),
            "MODEL-2026-11-17-01".to_owned(),
            digest("the first result"),
        )
        .await
        .expect("the canonical digest");

    let digest_owner = owner.as_str().to_owned();
    let change_digest = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs
                    SET canonical_digest = ?3
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![
                    digest_owner,
                    "MODEL-2026-11-17-01",
                    digest("the second result").to_string()
                ],
            )
        })
        .await;
    assert!(change_digest.is_err(), "the canonical digest is immutable");
}

#[tokio::test]
async fn the_sqlite_registries_are_append_only() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let store = SqliteImprovementStore::new(database.clone());
    let owner = profile("sqlite-append-only");
    let baseline = baseline(owner.as_str());
    store
        .add_baseline(owner.clone(), baseline.clone())
        .await
        .expect("the baseline append");
    let invalidation = invalidation(owner.as_str());
    store
        .add_invalidation(owner.clone(), invalidation.clone())
        .await
        .expect("the invalidation append");

    let baseline_owner = owner.as_str().to_owned();
    let baseline_id = baseline.baseline_id.clone();
    let baseline_update = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__baselines SET scope = 'changed'
                  WHERE profile_id = ?1 AND baseline_id = ?2",
                rusqlite::params![baseline_owner, baseline_id],
            )
        })
        .await;
    assert!(baseline_update.is_err(), "a baseline is append-only");

    let baseline_owner = owner.as_str().to_owned();
    let baseline_id = baseline.baseline_id.clone();
    let baseline_delete = database
        .call(move |connection| {
            connection.execute(
                "DELETE FROM research__baselines
                  WHERE profile_id = ?1 AND baseline_id = ?2",
                rusqlite::params![baseline_owner, baseline_id],
            )
        })
        .await;
    assert!(baseline_delete.is_err(), "a baseline cannot be deleted");

    let invalidation_owner = owner.as_str().to_owned();
    let invalidation_id = invalidation.invalidation_id.clone();
    let invalidation_update = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__invalidations SET reason = 'changed'
                  WHERE profile_id = ?1 AND invalidation_id = ?2",
                rusqlite::params![invalidation_owner, invalidation_id],
            )
        })
        .await;
    assert!(
        invalidation_update.is_err(),
        "an invalidation is append-only"
    );

    let invalidation_owner = owner.as_str().to_owned();
    let invalidation_id = invalidation.invalidation_id.clone();
    let invalidation_delete = database
        .call(move |connection| {
            connection.execute(
                "DELETE FROM research__invalidations
                  WHERE profile_id = ?1 AND invalidation_id = ?2",
                rusqlite::params![invalidation_owner, invalidation_id],
            )
        })
        .await;
    assert!(
        invalidation_delete.is_err(),
        "an invalidation cannot be deleted"
    );

    assert_eq!(
        store.baselines(owner.clone()).await.expect("the baselines"),
        vec![baseline]
    );
    assert_eq!(
        store.invalidations(owner).await.expect("the invalidations"),
        vec![invalidation]
    );
}

#[tokio::test]
async fn the_sqlite_ledger_refuses_a_direct_created_to_decided_update() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
    let store = SqliteImprovementStore::new(database.clone());
    let owner = profile("skip-submit");
    store
        .create(
            owner.clone(),
            manifest("MODEL-2026-11-17-01"),
            "2026-11-17T00:00:00Z".to_owned(),
        )
        .await
        .expect("the pre-registration");

    let owner_id = owner.as_str().to_owned();
    let update = database
        .call(move |connection| {
            connection.execute(
                "UPDATE research__improvement_runs
                    SET pipeline_run_id = ?3,
                        canonical_digest = ?4,
                        submitted_at = '2026-11-17T00:05:00Z',
                        state = 'decided',
                        decision = 'accepted',
                        decision_note = 'skipped submit',
                        decided_at = '2026-11-17T00:10:00Z'
                  WHERE profile_id = ?1 AND improvement_id = ?2",
                rusqlite::params![
                    owner_id,
                    "MODEL-2026-11-17-01",
                    RunId(digest("the run")).to_string(),
                    digest("the digest").to_string()
                ],
            )
        })
        .await;
    assert!(update.is_err(), "created must move to submitted first");
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
    registry_conformance(
        &store,
        &profile(&format!("pg-registry-{}", std::process::id())),
    )
    .await;

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

    let change_digest = sqlx::query(
        "UPDATE research.improvement_runs SET canonical_digest = $2
          WHERE profile_id = $1 AND improvement_id = 'MODEL-2026-11-17-01'",
    )
    .bind(owner.as_str())
    .bind(digest("another result").to_string())
    .execute(database.pool())
    .await;
    assert!(change_digest.is_err(), "a canonical digest is immutable");

    let registry_owner = profile(&format!("pg-append-{}", std::process::id()));
    let baseline = baseline(registry_owner.as_str());
    let invalidation = invalidation(registry_owner.as_str());
    store
        .add_baseline(registry_owner.clone(), baseline.clone())
        .await
        .expect("the baseline append");
    store
        .add_invalidation(registry_owner.clone(), invalidation.clone())
        .await
        .expect("the invalidation append");

    let baseline_update = sqlx::query(
        "UPDATE research.baselines SET scope = 'changed'
          WHERE profile_id = $1 AND baseline_id = $2",
    )
    .bind(registry_owner.as_str())
    .bind(&baseline.baseline_id)
    .execute(database.pool())
    .await;
    assert!(baseline_update.is_err(), "a baseline is append-only");

    let invalidation_delete = sqlx::query(
        "DELETE FROM research.invalidations
          WHERE profile_id = $1 AND invalidation_id = $2",
    )
    .bind(registry_owner.as_str())
    .bind(&invalidation.invalidation_id)
    .execute(database.pool())
    .await;
    assert!(
        invalidation_delete.is_err(),
        "an invalidation cannot be deleted"
    );

    let skip_submit_owner = profile(&format!("pg-skip-{}", std::process::id()));
    store
        .create(
            skip_submit_owner.clone(),
            manifest("MODEL-2026-11-17-01"),
            "2026-11-17T00:00:00Z".to_owned(),
        )
        .await
        .expect("the skip-submit pre-registration");
    let skip_submit = sqlx::query(
        "UPDATE research.improvement_runs
            SET pipeline_run_id = $2, canonical_digest = $3,
                submitted_at = $4::timestamptz, state = 'decided',
                decision = 'accepted', decision_note = 'skipped submit',
                decided_at = $5::timestamptz
          WHERE profile_id = $1 AND improvement_id = 'MODEL-2026-11-17-01'",
    )
    .bind(skip_submit_owner.as_str())
    .bind(RunId(digest("the run")).to_string())
    .bind(digest("the digest").to_string())
    .bind("2026-11-17T00:05:00Z")
    .bind("2026-11-17T00:10:00Z")
    .execute(database.pool())
    .await;
    assert!(skip_submit.is_err(), "created must move to submitted first");
}
