//! The shared `ArtifactCatalog` conformance suite.
//!
//! The reference and retention rules are the contract: a referenced artifact is
//! never collected, an unreferenced one is collected only at or before the
//! cutoff, every collection writes a tombstone, and a shared object is reported
//! so the caller does not delete the other profile's bytes.

use quantrule_api::{ArtifactKind, ArtifactReference, DatasetReferenceKind, Digest, ProfileId};
use quantrule_data_service::artifacts::store::{ArtifactCatalog, NewArtifact};
use quantrule_data_service::artifacts::{MemoryArtifacts, postgres::PgArtifacts};
use quantrule_data_service::migrations;
use quantrule_runtime::adapters::postgres::{
    PgDatabase, apply_migrations_in as apply_postgres_migrations,
};

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

fn artifact(seed: &[u8], created_at: &str) -> NewArtifact {
    let artifact_id = Digest::sha256(seed);
    NewArtifact {
        artifact_id: artifact_id.clone(),
        kind: ArtifactKind::Signal,
        object_key: format!("artifacts/{}", artifact_id.as_hex()),
        bytes: 128,
        created_at: created_at.to_owned(),
    }
}

fn reference(id: &str) -> ArtifactReference {
    ArtifactReference {
        reference_id: id.to_owned(),
        kind: DatasetReferenceKind::Run,
        created_at: "2026-11-22T00:00:00Z".to_owned(),
    }
}

async fn conformance(catalog: &dyn ArtifactCatalog) {
    let owner = profile("conformance-artifact");
    let other = profile("conformance-artifact-other");
    assert!(
        catalog
            .list(owner.clone(), ArtifactKind::Signal)
            .await
            .expect("the empty list")
            .is_empty()
    );

    let retained = artifact(b"retained", "2026-01-01T00:00:00Z");
    let doomed = artifact(b"doomed", "2026-01-01T00:00:00Z");
    let fresh = artifact(b"fresh", "2026-11-20T00:00:00Z");
    for item in [&retained, &doomed, &fresh] {
        catalog
            .register(owner.clone(), item.clone())
            .await
            .expect("register");
    }
    let again = catalog
        .register(owner.clone(), retained.clone())
        .await
        .expect("idempotent register");
    assert_eq!(again.summary.artifact_id, retained.artifact_id);
    assert_eq!(
        catalog
            .get(
                owner.clone(),
                ArtifactKind::Signal,
                doomed.artifact_id.to_string()
            )
            .await
            .expect("get")
            .expect("present")
            .summary
            .bytes,
        128
    );
    assert_eq!(
        catalog
            .list(owner.clone(), ArtifactKind::Signal)
            .await
            .expect("list")
            .len(),
        3
    );

    for _ in 0..2 {
        assert_eq!(
            catalog
                .add_reference(
                    owner.clone(),
                    ArtifactKind::Signal,
                    retained.artifact_id.to_string(),
                    reference("run:one"),
                )
                .await
                .expect("reference"),
            1
        );
    }
    assert_eq!(
        catalog
            .references(
                owner.clone(),
                ArtifactKind::Signal,
                retained.artifact_id.to_string()
            )
            .await
            .expect("references")
            .len(),
        1
    );

    let cutoff = "2026-11-01T00:00:00Z".to_owned();
    let collected_at = "2026-11-22T00:00:00Z".to_owned();
    let collected = catalog
        .collect(
            owner.clone(),
            ArtifactKind::Signal,
            cutoff.clone(),
            collected_at.clone(),
        )
        .await
        .expect("collect");
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].artifact_id, doomed.artifact_id);
    assert!(!collected[0].object_shared);
    assert!(
        catalog
            .get(
                owner.clone(),
                ArtifactKind::Signal,
                retained.artifact_id.to_string()
            )
            .await
            .expect("retained")
            .is_some(),
        "a referenced artifact was collected"
    );
    assert!(
        catalog
            .get(
                owner.clone(),
                ArtifactKind::Signal,
                fresh.artifact_id.to_string()
            )
            .await
            .expect("fresh")
            .is_some(),
        "an artifact inside the window was collected"
    );

    assert_eq!(
        catalog
            .release_reference(
                owner.clone(),
                ArtifactKind::Signal,
                retained.artifact_id.to_string(),
                "run:one".to_owned(),
            )
            .await
            .expect("release"),
        0
    );
    let collected = catalog
        .collect(
            owner.clone(),
            ArtifactKind::Signal,
            cutoff.clone(),
            collected_at.clone(),
        )
        .await
        .expect("collect after release");
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].artifact_id, retained.artifact_id);

    // A shared object is reported, never silently claimed by one profile.
    let shared = artifact(b"shared", "2026-01-01T00:00:00Z");
    catalog
        .register(owner.clone(), shared.clone())
        .await
        .expect("owner");
    catalog
        .register(other, shared.clone())
        .await
        .expect("other");
    let collected = catalog
        .collect(owner.clone(), ArtifactKind::Signal, cutoff, collected_at)
        .await
        .expect("collect shared");
    assert_eq!(collected.len(), 1);
    assert!(collected[0].object_shared);
}

#[tokio::test]
async fn the_memory_artifact_catalog_conforms() {
    conformance(&MemoryArtifacts::default()).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at an empty, disposable Postgres"]
async fn the_postgres_artifact_catalog_conforms() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name an empty, disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_postgres_migrations(database.pool(), "catalog", &migrations::available())
        .await
        .expect("the catalog migrations");
    for statement in [
        "DELETE FROM catalog.artifact_references
          WHERE profile_id IN ('conformance-artifact', 'conformance-artifact-other')",
        "DELETE FROM catalog.artifacts
          WHERE profile_id IN ('conformance-artifact', 'conformance-artifact-other')",
        "DELETE FROM catalog.retention_events
          WHERE kind = 'artifact'
            AND profile_id IN ('conformance-artifact', 'conformance-artifact-other')",
    ] {
        sqlx::query(statement)
            .execute(database.pool())
            .await
            .expect("the conformance cleanup");
    }
    let pool = database.pool().clone();
    conformance(&PgArtifacts::new(pool.clone())).await;
    let tombstones: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM catalog.retention_events
          WHERE kind = 'artifact' AND profile_id = 'conformance-artifact'
            AND action = 'erase' AND reason = 'retention.collect'",
    )
    .fetch_one(&pool)
    .await
    .expect("the tombstone count");
    assert_eq!(tombstones.0, 3, "every collection keeps one tombstone");
}
