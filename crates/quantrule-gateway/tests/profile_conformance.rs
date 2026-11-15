//! The shared `ProfileStore` conformance suite.
//!
//! The same body runs against the in-memory store, the SQLite store, and — when
//! `QR_TEST_DATABASE_URL` names a disposable Postgres — the Postgres store. The
//! fast tier runs the first two; the Postgres case is opt-in because it needs a
//! live server. The suite asserts the store contract, not a SQL dialect: create,
//! duplicate, get, missing, and one ordered list.

use std::collections::BTreeMap;

use quantrule_api::ProfileId;
use quantrule_gateway::adapters::postgres::PgProfileStore;
use quantrule_gateway::adapters::sqlite::SqliteProfileStore;
use quantrule_gateway::profile::{MemoryProfiles, ProfileError, ProfileRecord, ProfileStore};
use quantrule_runtime::adapters::postgres::{
    PgDatabase, apply_migrations_in as apply_postgres_migrations,
};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

fn record(id: &str) -> ProfileRecord {
    ProfileRecord {
        profile_id: ProfileId::parse(id).expect("the profile id"),
        name: id.to_owned(),
        created_at: "2026-11-15T00:00:00Z".to_owned(),
    }
}

async fn conformance(store: &dyn ProfileStore) {
    let alpha = record("conformance-alpha");
    let beta = record("conformance-beta");
    let missing = ProfileId::parse("conformance-missing").expect("the missing id");

    let mut expected: BTreeMap<String, ProfileRecord> = store
        .list()
        .await
        .expect("the initial list")
        .into_iter()
        .map(|profile| (profile.profile_id.as_str().to_owned(), profile))
        .collect();

    store.create(alpha.clone()).await.expect("alpha creates");
    assert_eq!(
        store.create(alpha.clone()).await,
        Err(ProfileError::AlreadyExists),
        "a duplicate id was accepted"
    );
    assert_eq!(
        store
            .get(alpha.profile_id.clone())
            .await
            .expect("alpha reads"),
        Some(alpha.clone())
    );
    assert_eq!(
        store.get(missing).await.expect("the missing id reads"),
        None
    );
    expected.insert(alpha.profile_id.as_str().to_owned(), alpha.clone());
    assert_eq!(
        store.list().await.expect("the list after alpha"),
        expected.values().cloned().collect::<Vec<_>>()
    );

    store.create(beta.clone()).await.expect("beta creates");
    expected.insert(beta.profile_id.as_str().to_owned(), beta);
    assert_eq!(
        store.list().await.expect("the list after beta"),
        expected.values().cloned().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn the_memory_store_conforms() {
    conformance(&MemoryProfiles::default()).await;
}

#[tokio::test]
async fn the_sqlite_store_conforms() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(
        &database,
        "gateway",
        &quantrule_gateway::migrations::sqlite(),
    )
    .await
    .expect("the gateway baseline");
    let store = SqliteProfileStore::new(database);
    conformance(&store).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_store_conforms() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_postgres_migrations(
        database.pool(),
        "gateway",
        &quantrule_gateway::migrations::gateway(),
    )
    .await
    .expect("the gateway migrations");
    sqlx::query(
        "DELETE FROM gateway.profiles
          WHERE profile_id IN ('conformance-alpha', 'conformance-beta')",
    )
    .execute(database.pool())
    .await
    .expect("the conformance cleanup");
    conformance(&PgProfileStore::new(database.pool().clone())).await;
}
