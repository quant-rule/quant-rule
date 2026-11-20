//! The shared `DatasetCatalog` and `ConfigCatalog` conformance suite.
//!
//! The same body runs against the in-memory stores, the SQLite stores, and —
//! when `QR_TEST_DATABASE_URL` names a disposable Postgres — the Postgres
//! stores. The fast tier runs the first two; the Postgres case is opt-in
//! because it needs a live server. The suite asserts the store contract, not a
//! SQL dialect: round-trip, duplicate, profile scope, shared content, ordering,
//! and the stored quality/files documents.

use std::collections::BTreeMap;

use quantrule_api::{
    ConfigOrigin, DatasetRecord, DatasetReference, DatasetReferenceKind, DatasetSummary, Digest,
    ProfileId, QualityReport, SkippedRow,
};
use quantrule_data_service::migrations;
use quantrule_data_service::postgres::{PgConfigStore, PgDatasetCatalog};
use quantrule_data_service::sqlite::{SqliteConfigStore, SqliteDatasetCatalog};
use quantrule_data_service::{
    CatalogError, ConfigCatalog, ConfigRecord, DatasetCatalog, MemoryCatalog, MemoryConfigs,
};
use quantrule_runtime::adapters::postgres::{
    PgDatabase, apply_migrations_in as apply_postgres_migrations,
};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("the profile id")
}

fn summary(seed: &[u8], name: &str) -> DatasetSummary {
    let version = Digest::sha256(seed);
    DatasetSummary {
        data_version: version,
        name: name.to_owned(),
        candles: 10,
        first_bar: 1_704_067_200,
        last_bar: 1_704_067_260,
        bytes: 128,
        archived_at: None,
    }
}

fn dataset(seed: &[u8], name: &str, quality: Option<QualityReport>) -> DatasetRecord {
    let summary = summary(seed, name);
    DatasetRecord {
        object_key: format!("datasets/{}", summary.data_version.as_hex()),
        sha256: summary.data_version.clone(),
        summary,
        quality,
        created_at: "2026-11-15T09:00:00Z".to_owned(),
    }
}

fn quality() -> QualityReport {
    QualityReport {
        rows_read: 8,
        accepted: 4,
        rejected: 4,
        duplicates: 1,
        out_of_order: 1,
        invalid: 2,
        skipped: vec![SkippedRow {
            line: 3,
            reason: "invalid".to_owned(),
        }],
    }
}

async fn dataset_conformance(catalog: &dyn DatasetCatalog) {
    let owner = profile("conformance-catalog");
    let other = profile("conformance-catalog-other");
    let empty = profile("conformance-catalog-empty");

    assert_eq!(
        catalog.count(owner.clone()).await.expect("the count"),
        0,
        "the profile starts with rows from an earlier run"
    );
    assert!(
        catalog
            .list(owner.clone())
            .await
            .expect("the empty list")
            .is_empty()
    );

    let first = dataset(b"conformance-catalog-a", "a.csv", None);
    catalog
        .insert(owner.clone(), first.clone())
        .await
        .expect("the first insert");
    assert_eq!(catalog.count(owner.clone()).await.expect("the count"), 1);
    assert_eq!(
        catalog
            .get(owner.clone(), first.summary.data_version.to_string())
            .await
            .expect("the read"),
        Some(first.clone())
    );
    assert_eq!(
        catalog.insert(owner.clone(), first.clone()).await,
        Err(CatalogError::AlreadyExists),
        "a duplicate version was accepted"
    );

    // The row is profile-scoped; the object key is the same content address.
    catalog
        .insert(other.clone(), first.clone())
        .await
        .expect("the second profile insert");
    assert_eq!(catalog.count(other.clone()).await.expect("the count"), 1);
    assert_eq!(catalog.count(owner.clone()).await.expect("the count"), 1);

    let second = dataset(b"conformance-catalog-b", "b.csv", Some(quality()));
    catalog
        .insert(owner.clone(), second.clone())
        .await
        .expect("the second insert");

    let mut ordered = vec![first.clone(), second];
    ordered.sort_by_key(|record| record.summary.data_version.clone());
    let listed = catalog.list(owner.clone()).await.expect("the ordered list");
    assert_eq!(listed, ordered, "the list is not ordered by version");
    assert_eq!(
        listed
            .iter()
            .find(|record| record.quality.is_some())
            .and_then(|record| record.quality.clone()),
        Some(quality()),
        "the quality document did not round trip"
    );

    assert_eq!(
        catalog
            .get(
                owner.clone(),
                Digest::sha256(b"conformance-catalog-missing").to_string()
            )
            .await
            .expect("the missing read"),
        None
    );
    assert!(
        catalog
            .list(empty.clone())
            .await
            .expect("the empty profile list")
            .is_empty()
    );

    // Runtime history: archive hides the row, a reference blocks erase, and the
    // shared object keeps the other profile's row readable.
    let version = first.summary.data_version.to_string();
    assert_eq!(
        catalog.erase(owner.clone(), version.clone()).await,
        Err(CatalogError::NotArchived),
        "an active dataset was erased"
    );
    let archived = catalog
        .archive(
            owner.clone(),
            version.clone(),
            "2026-11-20T00:00:00Z".to_owned(),
        )
        .await
        .expect("the archive");
    assert_eq!(
        archived.summary.archived_at.as_deref(),
        Some("2026-11-20T00:00:00Z")
    );
    assert_eq!(
        catalog
            .archive(
                owner.clone(),
                version.clone(),
                "2026-11-20T00:00:01Z".to_owned()
            )
            .await,
        Err(CatalogError::Archived)
    );
    let active = catalog.list(owner.clone()).await.expect("the active list");
    assert!(
        !active
            .iter()
            .any(|record| record.summary.data_version.to_string() == version),
        "an archived dataset was listed by default"
    );
    assert_eq!(
        catalog
            .list_including_archived(owner.clone())
            .await
            .expect("the history list")
            .len(),
        2
    );
    assert_eq!(
        catalog
            .count(owner.clone())
            .await
            .expect("the active count"),
        1
    );

    let reference = DatasetReference {
        reference_id: "run:conformance-1".to_owned(),
        kind: DatasetReferenceKind::Run,
        created_at: "2026-11-20T00:00:00Z".to_owned(),
    };
    assert_eq!(
        catalog
            .add_reference(owner.clone(), version.clone(), reference.clone())
            .await
            .expect("the reference"),
        1
    );
    assert_eq!(
        catalog
            .add_reference(owner.clone(), version.clone(), reference)
            .await
            .expect("the idempotent reference"),
        1
    );
    let references = catalog
        .references(owner.clone(), version.clone())
        .await
        .expect("the reference list");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].kind, DatasetReferenceKind::Run);
    assert_eq!(
        catalog.erase(owner.clone(), version.clone()).await,
        Err(CatalogError::Referenced(1)),
        "a referenced dataset was erased"
    );
    assert_eq!(
        catalog
            .release_reference(
                owner.clone(),
                version.clone(),
                "run:conformance-1".to_owned()
            )
            .await
            .expect("the release"),
        0
    );
    let erased = catalog
        .erase(owner.clone(), version.clone())
        .await
        .expect("the erase");
    assert!(
        erased.object_shared,
        "the other profile still points at the same object"
    );
    assert_eq!(
        catalog
            .get(owner.clone(), version.clone())
            .await
            .expect("the erased read"),
        None
    );
    assert!(
        catalog
            .get(other.clone(), version.clone())
            .await
            .expect("the shared read")
            .is_some(),
        "erasing one profile removed another profile's row"
    );

    // A unique dataset takes its object with it.
    let unique = dataset(b"conformance-catalog-unique", "unique.csv", None);
    let unique_version = unique.summary.data_version.to_string();
    catalog
        .insert(owner.clone(), unique)
        .await
        .expect("the unique insert");
    catalog
        .archive(
            owner.clone(),
            unique_version.clone(),
            "2026-11-20T00:00:00Z".to_owned(),
        )
        .await
        .expect("the unique archive");
    let erased = catalog
        .erase(owner.clone(), unique_version)
        .await
        .expect("the unique erase");
    assert!(!erased.object_shared);

    assert_eq!(
        catalog
            .archive(
                owner,
                Digest::sha256(b"conformance-catalog-missing").to_string(),
                "2026-11-20T00:00:00Z".to_owned()
            )
            .await,
        Err(CatalogError::NotFound)
    );
}

fn config(id: &str, origin: ConfigOrigin, valid: bool) -> ConfigRecord {
    ConfigRecord {
        config_id: id.to_owned(),
        name: format!("{id}/run.json"),
        origin,
        valid,
        message: (!valid).then(|| "missing-data.json".to_owned()),
        config_hash: valid.then(|| Digest::sha256(id.as_bytes())),
        body: "{\"run\":\"conformance\"}".to_owned(),
        files: BTreeMap::from([
            (
                "run.json".to_owned(),
                "{\"run\":\"conformance\"}".to_owned(),
            ),
            (
                "data.json".to_owned(),
                "{\"candles\":\"fixture\"}".to_owned(),
            ),
        ]),
        created_at: "2026-11-15T09:00:00Z".to_owned(),
    }
}

async fn config_conformance(store: &dyn ConfigCatalog) {
    let owner = profile("conformance-config");
    let other = profile("conformance-config-other");
    let empty = profile("conformance-config-empty");

    assert!(
        store
            .list(owner.clone())
            .await
            .expect("the empty list")
            .is_empty()
    );

    let valid = config("conformance_cfg_a", ConfigOrigin::Repository, true);
    store
        .upsert(owner.clone(), valid.clone())
        .await
        .expect("the first upsert");
    assert_eq!(
        store
            .get(owner.clone(), valid.config_id.clone())
            .await
            .expect("the read"),
        Some(valid.clone()),
        "the config, its bundle, and its first instant did not round trip"
    );
    let listed = store.list(owner.clone()).await.expect("the list");
    assert_eq!(listed, vec![valid.clone()]);
    assert_eq!(listed[0].summary(), valid.summary());

    // An upsert refreshes the row; the bundle is keyed by content id and is
    // shared across profiles.
    let mut refreshed = valid.clone();
    refreshed.name = "refreshed/run.json".to_owned();
    refreshed.origin = ConfigOrigin::Upload;
    refreshed.valid = false;
    refreshed.message = Some("model.json is missing".to_owned());
    refreshed.config_hash = None;
    refreshed.body = "{\"run\":\"refreshed\"}".to_owned();
    store
        .upsert(owner.clone(), refreshed.clone())
        .await
        .expect("the refresh");
    let loaded = store
        .get(owner.clone(), valid.config_id.clone())
        .await
        .expect("the refreshed read")
        .expect("the refreshed row");
    assert_eq!(loaded.config_id, refreshed.config_id);
    assert_eq!(loaded.name, refreshed.name);
    assert_eq!(loaded.origin, ConfigOrigin::Upload);
    assert!(!loaded.valid);
    assert_eq!(loaded.message, refreshed.message);
    assert_eq!(loaded.config_hash, None);
    assert_eq!(loaded.body, refreshed.body);
    assert_eq!(loaded.files, refreshed.files);

    // The same config id in another profile is a second row over one bundle.
    let mut shared = valid.clone();
    shared.created_at = "2026-11-16T00:00:00Z".to_owned();
    store
        .upsert(other.clone(), shared.clone())
        .await
        .expect("the second profile upsert");
    assert_eq!(
        store
            .get(other.clone(), shared.config_id.clone())
            .await
            .expect("the second profile read"),
        Some(shared.clone())
    );
    let loaded = store
        .get(owner.clone(), valid.config_id.clone())
        .await
        .expect("the owner read")
        .expect("the owner row");
    assert_eq!(loaded.files, valid.files, "the bundle is shared by id");

    let second = config("conformance_cfg_b", ConfigOrigin::Repository, true);
    store
        .upsert(owner.clone(), second.clone())
        .await
        .expect("the second config");
    let ids: Vec<String> = store
        .list(owner.clone())
        .await
        .expect("the ordered list")
        .into_iter()
        .map(|record| record.config_id)
        .collect();
    assert_eq!(ids, vec!["conformance_cfg_a", "conformance_cfg_b"]);

    assert_eq!(
        store
            .get(owner.clone(), "conformance_cfg_missing".to_owned())
            .await
            .expect("the missing read"),
        None
    );
    assert!(
        store
            .list(empty)
            .await
            .expect("the empty profile list")
            .is_empty()
    );
    assert_eq!(
        store
            .get(other, valid.config_id)
            .await
            .expect("the other profile read")
            .map(|record| record.config_id),
        Some("conformance_cfg_a".to_owned())
    );
}

#[tokio::test]
async fn the_memory_stores_conform() {
    dataset_conformance(&MemoryCatalog::default()).await;
    config_conformance(&MemoryConfigs::default()).await;
}

async fn sqlite_database() -> SqliteDatabase {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "catalog", &migrations::sqlite())
        .await
        .expect("the catalog baseline");
    database
}

#[tokio::test]
async fn the_sqlite_stores_conform() {
    let database = sqlite_database().await;
    dataset_conformance(&SqliteDatasetCatalog::new(database.clone())).await;
    config_conformance(&SqliteConfigStore::new(database)).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at an empty, disposable Postgres"]
async fn the_postgres_stores_conform() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name an empty, disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_postgres_migrations(database.pool(), "catalog", &migrations::available())
        .await
        .expect("the catalog migrations");

    for statement in [
        "DELETE FROM catalog.datasets
          WHERE profile_id IN ('conformance-catalog', 'conformance-catalog-other')",
        "DELETE FROM catalog.configs
          WHERE profile_id IN ('conformance-config', 'conformance-config-other')",
        "DELETE FROM catalog.config_bundles
          WHERE config_id LIKE 'conformance_cfg_%'",
    ] {
        sqlx::query(statement)
            .execute(database.pool())
            .await
            .expect("the conformance cleanup");
    }

    let pool = database.pool().clone();
    dataset_conformance(&PgDatasetCatalog::new(pool.clone())).await;
    config_conformance(&PgConfigStore::new(pool)).await;
}
