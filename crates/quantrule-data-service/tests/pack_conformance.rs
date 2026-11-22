//! The shared `PackRegistry` conformance suite.
//!
//! The same body runs against the in-memory registry and, when
//! `QR_TEST_DATABASE_URL` names a disposable Postgres, the Postgres registry.
//! The suite asserts the store contract, not a SQL dialect: canonical identity,
//! duplicate-label refusal, idempotent content, the guarded lifecycle, and the
//! stored canonical bytes.

use quantrule_api::{Digest, PackKind, PackState, ProfileId};
use quantrule_data_service::migrations;
use quantrule_data_service::packs::postgres::PgPacks;
use quantrule_data_service::packs::{MemoryPacks, NewPack, PackFilter, PackRegistry, parse_pack};
use quantrule_runtime::adapters::postgres::{
    PgDatabase, apply_migrations_in as apply_postgres_migrations,
};
use serde_json::{Value, json};

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

fn document(version: &str, exposure: f64) -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "portfolio",
        "version": version,
        "name": "conformance-single-position",
        "owner": "operator",
        "payload": {
            "sizing": {
                "mode": "equity_fraction",
                "fraction": 0.05,
                "quantity_step": 0.0001
            },
            "risk": {
                "per_trade_risk": 0.01,
                "max_exposure": exposure,
                "max_open_positions": 1
            },
            "drawdown": {"limit": 0.15, "action": "halt"},
            "rebalance": "none",
            "constraints": []
        }
    })
}

fn pack(version: &str, exposure: f64) -> NewPack {
    let bytes = serde_json::to_vec(&document(version, exposure)).expect("encode");
    let parsed = parse_pack(&bytes).expect("the pack is valid");
    NewPack {
        pack_id: parsed.pack_id.clone(),
        kind: parsed.kind,
        name: parsed.name,
        version: parsed.version,
        owner: parsed.owner,
        object_key: format!("packs/{}", parsed.pack_id.as_hex()),
        bytes: parsed.canonical.len() as u64,
        created_at: "2026-11-22T00:00:00Z".to_owned(),
    }
}

async fn conformance(registry: &dyn PackRegistry) {
    let owner = profile("conformance-pack");
    let other = profile("conformance-pack-other");

    assert!(
        registry
            .list(owner.clone(), PackFilter::default())
            .await
            .expect("the empty list")
            .is_empty(),
        "the profile starts with rows from an earlier run"
    );

    let first = pack("1.0.0", 0.5);
    let id = first.pack_id.to_string();
    let outcome = registry
        .register(owner.clone(), first.clone())
        .await
        .expect("register");
    assert!(outcome.created);
    assert_eq!(outcome.record.summary.state, PackState::Draft);
    assert_eq!(outcome.record.summary.kind, PackKind::Portfolio);
    assert_eq!(outcome.record.summary.bytes, first.bytes);
    assert_eq!(outcome.record.events.len(), 1);
    assert_eq!(outcome.record.sha256, first.pack_id);

    let again = registry
        .register(owner.clone(), first.clone())
        .await
        .expect("idempotent");
    assert!(!again.created);
    assert_eq!(again.record, outcome.record);

    let changed = pack("1.0.0", 0.25);
    assert_ne!(changed.pack_id, first.pack_id);
    assert!(matches!(
        registry.register(owner.clone(), changed).await,
        Err(quantrule_data_service::PackRegistryError::LabelConflict { .. })
    ));

    registry
        .register(other.clone(), first.clone())
        .await
        .expect("another profile may cite the same bytes");
    assert_eq!(
        registry
            .get(other, id.clone())
            .await
            .expect("the other profile read"),
        Some(outcome.record.clone())
    );

    assert!(matches!(
        registry
            .freeze(owner.clone(), id.clone(), "2026-11-22T00:01:00Z".to_owned())
            .await,
        Err(quantrule_data_service::PackRegistryError::NotValidated(_))
    ));
    let validated = registry
        .validate(owner.clone(), id.clone(), "2026-11-22T00:02:00Z".to_owned())
        .await
        .expect("validate");
    assert_eq!(validated.summary.state, PackState::Validated);
    assert_eq!(validated.validations.len(), 1);
    assert_eq!(validated.validations[0].validator, "pack.schema/v1");

    let frozen = registry
        .freeze(owner.clone(), id.clone(), "2026-11-22T00:03:00Z".to_owned())
        .await
        .expect("freeze");
    assert_eq!(frozen.summary.state, PackState::Frozen);
    assert_eq!(frozen.events.len(), 3);
    let repeat = registry
        .freeze(owner.clone(), id.clone(), "2026-11-22T00:04:00Z".to_owned())
        .await
        .expect("idempotent freeze");
    assert_eq!(repeat, frozen);
    assert!(matches!(
        registry
            .validate(owner.clone(), id.clone(), "2026-11-22T00:05:00Z".to_owned())
            .await,
        Err(quantrule_data_service::PackRegistryError::Frozen)
    ));

    let listed = registry
        .list(
            owner.clone(),
            PackFilter {
                kind: Some(PackKind::Portfolio),
                state: Some(PackState::Frozen),
            },
        )
        .await
        .expect("filtered list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].pack_id, first.pack_id);
    assert!(
        registry
            .list(
                owner.clone(),
                PackFilter {
                    kind: Some(PackKind::Strategy),
                    state: None,
                },
            )
            .await
            .expect("kind filter")
            .is_empty()
    );

    let retired = registry
        .retire(owner.clone(), id.clone(), "2026-11-22T00:06:00Z".to_owned())
        .await
        .expect("retire");
    assert_eq!(retired.summary.state, PackState::Retired);
    let repeat = registry
        .retire(owner.clone(), id.clone(), "2026-11-22T00:07:00Z".to_owned())
        .await
        .expect("idempotent retire");
    assert_eq!(repeat, retired);
    assert!(matches!(
        registry
            .freeze(owner.clone(), id.clone(), "2026-11-22T00:08:00Z".to_owned())
            .await,
        Err(quantrule_data_service::PackRegistryError::Retired)
    ));
    assert_eq!(
        registry
            .get(owner.clone(), id.clone())
            .await
            .expect("read after retire")
            .expect("row survives retire")
            .summary
            .state,
        PackState::Retired
    );

    // Two registrars race on one content hash: one row, one creation event.
    let race = pack("2.0.0", 0.5);
    let (left, right) = tokio::join!(
        registry.register(owner.clone(), race.clone()),
        registry.register(owner.clone(), race.clone()),
    );
    let left = left.expect("the first registrar");
    let right = right.expect("the idempotent registrar");
    assert_eq!(left.record, right.record);
    assert_ne!(
        left.created, right.created,
        "exactly one registrar must create the duplicate pack"
    );
    assert_eq!(
        registry
            .list(owner.clone(), PackFilter::default())
            .await
            .expect("the race list")
            .into_iter()
            .filter(|pack| pack.version == "2.0.0")
            .count(),
        1
    );

    // A label still names one content hash: the loser fails, and the label has
    // one row after the race.
    let first_bytes = pack("2.0.1", 0.5);
    let second_bytes = pack("2.0.1", 0.25);
    let (left, right) = tokio::join!(
        registry.register(owner.clone(), first_bytes),
        registry.register(owner.clone(), second_bytes),
    );
    let mut winners = 0;
    let mut conflicts = 0;
    for result in [left, right] {
        match result {
            Ok(_) => winners += 1,
            Err(quantrule_data_service::PackRegistryError::LabelConflict { .. }) => {
                conflicts += 1;
            }
            Err(other) => panic!("unexpected label race result: {other}"),
        }
    }
    assert_eq!((winners, conflicts), (1, 1));
    assert_eq!(
        registry
            .list(owner.clone(), PackFilter::default())
            .await
            .expect("the label list")
            .into_iter()
            .filter(|pack| pack.version == "2.0.1")
            .count(),
        1
    );
}

#[tokio::test]
async fn the_memory_registry_conforms() {
    conformance(&MemoryPacks::default()).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at an empty, disposable Postgres"]
async fn the_postgres_registry_conforms() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name an empty, disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_postgres_migrations(database.pool(), "catalog", &migrations::available())
        .await
        .expect("the catalog migrations");
    let pack_files: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.tables
          WHERE table_schema = 'catalog' AND table_name = 'pack_files'",
    )
    .fetch_one(database.pool())
    .await
    .expect("the pack table count");
    assert_eq!(
        pack_files.0, 0,
        "the catalog must not carry a redundant pack byte table"
    );
    let byte_columns: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM information_schema.columns
          WHERE table_schema = 'catalog'
            AND table_name LIKE 'pack%'
            AND data_type = 'bytea'",
    )
    .fetch_one(database.pool())
    .await
    .expect("the pack byte column count");
    assert_eq!(
        byte_columns.0, 0,
        "the catalog must not store a pack byte payload"
    );
    for statement in [
        "DELETE FROM catalog.pack_lifecycle_events
          WHERE profile_id IN ('conformance-pack', 'conformance-pack-other')",
        "DELETE FROM catalog.pack_validation_records
          WHERE profile_id IN ('conformance-pack', 'conformance-pack-other')",
        "DELETE FROM catalog.packs
          WHERE profile_id IN ('conformance-pack', 'conformance-pack-other')",
    ] {
        sqlx::query(statement)
            .execute(database.pool())
            .await
            .expect("the conformance cleanup");
    }
    let registry = PgPacks::new(database.pool().clone());
    conformance(&registry).await;
    let missing = registry
        .get(
            profile("conformance-pack"),
            Digest::sha256(b"missing").to_string(),
        )
        .await
        .expect("the missing read");
    assert!(missing.is_none());
}
