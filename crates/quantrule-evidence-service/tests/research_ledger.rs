//! The research-ledger conformance suite.
//!
//! The shared conformance bodies run against the memory store; the Postgres
//! case is opt-in because it needs a live server. This target covers the
//! two-step pipeline link, the accepted-only digest rule, the append-only
//! registries, and the seeded committed rows.

mod research_ledger_support;

use quantrule_api::{Digest, ImprovementDecision, RunId};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::research::{
    ImprovementStore, MemoryImprovementStore, PgImprovementStore,
};

use research_ledger_support::{
    baseline, baseline_order_conformance, digest, invalidation, ledger_conformance, manifest,
    profile, registry_conformance,
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
async fn the_memory_baselines_are_newest_first() {
    let store = MemoryImprovementStore::default();
    baseline_order_conformance(&store, &profile("memory-baseline-order")).await;
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
    baseline_order_conformance(
        &store,
        &profile(&format!("pg-baseline-order-{}", std::process::id())),
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

#[tokio::test]
async fn a_research_record_is_archived_and_erased() {
    use quantrule_evidence_service::research::{ResearchArchiveOutcome, ResearchEraseOutcome};

    let store = MemoryImprovementStore::default();
    let owner = profile("retention-research");
    let id = "MODEL-2026-11-17-01";
    let manifest = manifest(id);
    let data_version = manifest.data_version.clone();
    store
        .create(owner.clone(), manifest, "2026-11-17T00:00:00Z".to_owned())
        .await
        .expect("the pre-registration");
    assert_eq!(
        store
            .erase(owner.clone(), id.to_owned())
            .await
            .expect("erase"),
        ResearchEraseOutcome::NotArchived
    );
    let ResearchArchiveOutcome::Archived(summary) = store
        .archive(
            owner.clone(),
            id.to_owned(),
            "2026-11-17T00:00:01Z".to_owned(),
        )
        .await
        .expect("archive")
    else {
        panic!("the record was not archived");
    };
    assert_eq!(summary.archived_at.as_deref(), Some("2026-11-17T00:00:01Z"));
    assert!(
        store
            .list(owner.clone(), 10)
            .await
            .expect("active list")
            .is_empty()
    );
    assert_eq!(
        store
            .list_including_archived(owner.clone(), 10)
            .await
            .expect("history list")
            .len(),
        1
    );
    let ResearchEraseOutcome::Erased(receipt) = store
        .erase(owner.clone(), id.to_owned())
        .await
        .expect("erase")
    else {
        panic!("the archived record was not erased");
    };
    assert_eq!(receipt.improvement_id, id);
    assert_eq!(receipt.data_version, data_version);
    assert!(
        store
            .get(owner, id.to_owned())
            .await
            .expect("get")
            .is_none()
    );
}

#[tokio::test]
async fn a_decided_research_record_refuses_erase() {
    use quantrule_evidence_service::research::{ResearchArchiveOutcome, ResearchEraseOutcome};

    let store = MemoryImprovementStore::default();
    let owner = profile("retention-decided");
    let id = "MODEL-2026-11-17-02";
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
            RunId(Digest::sha256(b"retention-decided-run")),
            "2026-11-17T00:01:00Z".to_owned(),
        )
        .await
        .expect("the run link");
    store
        .decide(
            owner.clone(),
            id.to_owned(),
            ImprovementDecision::Rejected,
            "the premise did not hold".to_owned(),
            "2026-11-17T00:02:00Z".to_owned(),
        )
        .await
        .expect("the decision");
    assert!(matches!(
        store
            .archive(
                owner.clone(),
                id.to_owned(),
                "2026-11-17T00:03:00Z".to_owned()
            )
            .await
            .expect("archive"),
        ResearchArchiveOutcome::Archived(_)
    ));
    assert_eq!(
        store.erase(owner, id.to_owned()).await.expect("erase"),
        ResearchEraseOutcome::Protected
    );
}
