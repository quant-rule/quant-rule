//! The migration advisory lock.
//!
//! Run against an empty, disposable Postgres:
//!
//! ```text
//! QR_TEST_DATABASE_URL=postgres://quantrule:quantrule@127.0.0.1:55432/quantrule \
//!   cargo test -p quantrule-runtime --test migration_lock \
//!   two_service_migrators_do_not_deadlock -- --ignored --nocapture
//! ```
//!
//! The test is ignored in the fast gate because it needs a database. The
//! database must contain no rows in `public.schema_migrations`: the runner is
//! shared and this test's small migration list would be refused against a
//! database carrying the service's list.

use std::time::Duration;

use quantrule_runtime::adapters::postgres::{MIGRATION_LOCK, PgDatabase, apply_migrations};
use quantrule_runtime::migrations::Migration;

/// One probe migration. The sleep widens the window in which the two migrators
/// overlap, so a lock that outlives its session blocks the second instead of
/// racing past it.
const FIRST: Migration = Migration {
    version: 9_001,
    name: "lock-probe-one",
    sql: "CREATE TABLE qr_lock_probe_one (id integer PRIMARY KEY);
          SELECT pg_sleep(0.2);",
};

/// The second probe migration.
const SECOND: Migration = Migration {
    version: 9_002,
    name: "lock-probe-two",
    sql: "CREATE TABLE qr_lock_probe_two (id integer PRIMARY KEY);
          SELECT pg_sleep(0.2);",
};

fn database_url() -> String {
    std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name an empty, disposable Postgres database; \
         this test is ignored in the fast gate",
    )
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at an empty, disposable Postgres"]
async fn two_service_migrators_do_not_deadlock() {
    let url = database_url();
    let first = PgDatabase::connect(&url).await.expect("the first pool");
    let second = PgDatabase::connect(&url).await.expect("the second pool");
    let available = [FIRST, SECOND];

    let outcome = tokio::time::timeout(Duration::from_secs(10), async {
        tokio::join!(
            apply_migrations(first.pool(), &available),
            apply_migrations(second.pool(), &available),
        )
    })
    .await
    .expect("two migrators deadlocked: the advisory lock outlived its session");

    let first_applied = outcome.0.expect("the first migrator");
    let second_applied = outcome.1.expect("the second migrator");
    assert_eq!(
        first_applied + second_applied,
        available.len(),
        "one migrator applies the set, the other finds it recorded"
    );
    assert!(
        first_applied == available.len() || second_applied == available.len(),
        "the lock serialises the whole set, not one migration at a time"
    );

    let ledger: (i64,) =
        sqlx::query_as("SELECT count(*) FROM public.schema_migrations WHERE version IN ($1, $2)")
            .bind(FIRST.version)
            .bind(SECOND.version)
            .fetch_one(first.pool())
            .await
            .expect("the ledger");
    assert_eq!(ledger.0, 2, "one ledger row per applied migration");

    // A third session can take the same lock immediately, so neither migrator
    // left it held by an idle pooled connection.
    let mut probe = first.pool().acquire().await.expect("a probe connection");
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(MIGRATION_LOCK)
        .fetch_one(&mut *probe)
        .await
        .expect("the lock probe");
    assert!(
        acquired,
        "the advisory lock is still held after both migrators finished"
    );
    let released: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_LOCK)
        .fetch_one(&mut *probe)
        .await
        .expect("the unlock probe");
    assert!(released, "the probe did not hold the lock it just took");
}
