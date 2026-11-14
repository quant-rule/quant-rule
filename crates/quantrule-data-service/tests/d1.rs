//! D1 regression: a catalog row decodes its quality report and timestamp in the
//! selected order.
//!
//! Run against a disposable Postgres:
//!
//! ```text
//! QR_TEST_DATABASE_URL=postgres://quantrule:quantrule@127.0.0.1:55432/quantrule \
//!   cargo test -p quantrule-data-service --test d1 -- --ignored --nocapture
//! ```
//!
//! Before the fix the select put `quality::text` in the created-at position and
//! `to_char(created_at ...)` in the quality position, so this read failed with
//! `a stored quality report is malformed`. The test is ignored in the fast gate
//! because it needs a database.

use quantrule_api::{DatasetRecord, Digest, ProfileId, QualityReport};
use quantrule_data_service::DatasetCatalog;
use quantrule_data_service::migrations;
use quantrule_data_service::postgres::PgDatasetCatalog;
use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};

fn database_url() -> String {
    std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name an empty, disposable Postgres database; \
         this test is ignored in the fast gate",
    )
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at an empty, disposable Postgres"]
async fn a_quality_report_and_its_timestamp_decode_in_the_selected_order() {
    let url = database_url();
    let database = PgDatabase::connect(&url).await.expect("the pool");
    apply_migrations_in(database.pool(), "catalog", &migrations::available())
        .await
        .expect("catalog migrations");

    let profile = ProfileId::parse("d1-profile").expect("profile");
    let version = Digest::sha256(b"d1-quality-row-783");
    let quality = QualityReport {
        rows_read: 8,
        accepted: 4,
        rejected: 4,
        duplicates: 1,
        out_of_order: 1,
        invalid: 2,
        skipped: Vec::new(),
    };
    let quality_json = serde_json::to_string(&quality).expect("quality json");
    sqlx::query(
        "INSERT INTO catalog.datasets
             (profile_id, data_version, name, candles, first_bar, last_bar, bytes,
              object_key, sha256, quality, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11::timestamptz)
         ON CONFLICT (profile_id, data_version) DO UPDATE SET
             quality = EXCLUDED.quality,
             created_at = EXCLUDED.created_at",
    )
    .bind(profile.as_str())
    .bind(version.to_string())
    .bind("d1.csv")
    .bind(4_i64)
    .bind(1_704_067_200_i64)
    .bind(1_704_067_260_i64)
    .bind(100_i64)
    .bind(format!("datasets/{}", version.as_hex()))
    .bind(version.as_hex())
    .bind(quality_json)
    .bind("2026-11-14 01:02:03+00")
    .execute(database.pool())
    .await
    .expect("the catalog row");

    let catalog = PgDatasetCatalog::new(database.pool().clone());
    let record: DatasetRecord = catalog
        .get(profile.clone(), version.to_string())
        .await
        .expect("the catalog read")
        .expect("the row");
    assert_eq!(record.quality, Some(quality.clone()));
    assert_eq!(record.created_at, "2026-11-14T01:02:03Z");

    let listed = catalog
        .list(profile.clone())
        .await
        .expect("the catalog list");
    let listed = listed
        .into_iter()
        .find(|record| record.summary.data_version == version)
        .expect("the row in the list");
    assert_eq!(listed.quality, Some(quality));
    assert_eq!(listed.created_at, "2026-11-14T01:02:03Z");

    sqlx::query("DELETE FROM catalog.datasets WHERE profile_id = $1 AND data_version = $2")
        .bind(profile.as_str())
        .bind(version.to_string())
        .execute(database.pool())
        .await
        .expect("cleanup");
}
