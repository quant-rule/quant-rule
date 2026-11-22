//! The Postgres pack registry.
//!
//! The rows live in the `catalog` schema: `packs` is the profile-scoped
//! metadata and identity row, and the validation and lifecycle tables are
//! append-only history. The canonical bytes are the object store's single
//! authority; this store keeps the object key, hash and size. A pack row and
//! its creation event commit together; every later transition locks the row and
//! appends its event in the same transaction, so a crash leaves either the old
//! state or the new one, never a state without its event.

use quantrule_api::{
    Digest, PackKind, PackLifecycleEvent, PackRecord, PackState, PackSummary, PackValidationRecord,
    ProfileId,
};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::store::{NewPack, PackFilter, PackRegistry, PackRegistryError, RegisterOutcome};

const VALIDATOR: &str = "pack.schema/v1";

/// A pack registry backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgPacks {
    pool: PgPool,
}

impl PgPacks {
    /// Build the registry over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type SummaryRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

type ValidationRow = (String, bool, Option<String>, String);
type EventRow = (Option<String>, String, String, Option<String>, String);

fn summary_of(row: &SummaryRow) -> Result<PackSummary, PackRegistryError> {
    let kind = PackKind::from_code(&row.1).ok_or_else(|| {
        PackRegistryError::Store(format!("a stored pack has an unknown kind: {}", row.1))
    })?;
    let state = PackState::from_code(&row.5).ok_or_else(|| {
        PackRegistryError::Store(format!("a stored pack has an unknown state: {}", row.5))
    })?;
    let pack_id = Digest::parse(&row.0).map_err(|error| {
        PackRegistryError::Store(format!("a stored pack id is malformed: {error}"))
    })?;
    let bytes = u64::try_from(row.6)
        .map_err(|_| PackRegistryError::Store("a stored pack size is negative".to_owned()))?;
    Ok(PackSummary {
        pack_id,
        kind,
        name: row.2.clone(),
        version: row.3.clone(),
        owner: row.4.clone(),
        state,
        bytes,
        created_at: row.7.clone(),
        validated_at: row.8.clone(),
        frozen_at: row.9.clone(),
        retired_at: row.10.clone(),
    })
}

const RECORD_SELECT: &str = "SELECT p.pack_id, p.kind, p.name, p.version, p.owner, p.state,
            p.bytes,
            to_char(p.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(p.validated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(p.frozen_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(p.retired_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
       FROM catalog.packs p
      WHERE p.profile_id = $1 AND p.pack_id = $2";

const LIST_SELECT: &str = "SELECT p.pack_id, p.kind, p.name, p.version, p.owner, p.state,
            p.bytes,
            to_char(p.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(p.validated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(p.frozen_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(p.retired_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
       FROM catalog.packs p
      WHERE p.profile_id = $1
        AND ($2::text IS NULL OR p.kind = $2)
        AND ($3::text IS NULL OR p.state = $3)
      ORDER BY p.name, p.version, p.pack_id";

async fn load_record(
    pool: &PgPool,
    profile: &ProfileId,
    pack_id: &str,
) -> Result<Option<PackRecord>, PackRegistryError> {
    let row: Option<SummaryRow> = sqlx::query_as(RECORD_SELECT)
        .bind(profile.as_str())
        .bind(pack_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| PackRegistryError::Store(error.to_string()))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let summary = summary_of(&row)?;
    let object_key: (String,) = sqlx::query_as(
        "SELECT object_key FROM catalog.packs WHERE profile_id = $1 AND pack_id = $2",
    )
    .bind(profile.as_str())
    .bind(pack_id)
    .fetch_one(pool)
    .await
    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
    let validations: Vec<ValidationRow> = sqlx::query_as(
        "SELECT validator, valid, message,
                to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
           FROM catalog.pack_validation_records
          WHERE profile_id = $1 AND pack_id = $2 ORDER BY record_id",
    )
    .bind(profile.as_str())
    .bind(pack_id)
    .fetch_all(pool)
    .await
    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
    let events: Vec<EventRow> = sqlx::query_as(
        "SELECT from_state, to_state, action, reason,
                to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
           FROM catalog.pack_lifecycle_events
          WHERE profile_id = $1 AND pack_id = $2 ORDER BY event_id",
    )
    .bind(profile.as_str())
    .bind(pack_id)
    .fetch_all(pool)
    .await
    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
    Ok(Some(PackRecord {
        summary: summary.clone(),
        object_key: object_key.0,
        sha256: summary.pack_id,
        validations: validations
            .into_iter()
            .map(
                |(validator, valid, message, created_at)| PackValidationRecord {
                    validator,
                    valid,
                    message,
                    created_at,
                },
            )
            .collect(),
        events: events
            .into_iter()
            .map(|(from, to, action, reason, created_at)| {
                Ok(PackLifecycleEvent {
                    from: from
                        .as_deref()
                        .map(|code| {
                            PackState::from_code(code).ok_or_else(|| {
                                PackRegistryError::Store(format!(
                                    "a stored lifecycle event has an unknown from state: {code}"
                                ))
                            })
                        })
                        .transpose()?,
                    to: PackState::from_code(&to).ok_or_else(|| {
                        PackRegistryError::Store(format!(
                            "a stored lifecycle event has an unknown to state: {to}"
                        ))
                    })?,
                    action,
                    reason,
                    created_at,
                })
            })
            .collect::<Result<Vec<_>, PackRegistryError>>()?,
    }))
}

impl PackRegistry for PgPacks {
    fn register(
        &self,
        profile: ProfileId,
        pack: NewPack,
    ) -> BoxFuture<Result<RegisterOutcome, PackRegistryError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let pack_id = pack.pack_id.to_string();
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let label: Option<(String,)> = sqlx::query_as(
                "SELECT pack_id FROM catalog.packs
                  WHERE profile_id = $1 AND name = $2 AND version = $3",
            )
            .bind(profile.as_str())
            .bind(&pack.name)
            .bind(&pack.version)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            if let Some((existing,)) = &label
                && existing != &pack_id
            {
                return Err(PackRegistryError::LabelConflict {
                    existing: Digest::parse(existing).map_err(|error| {
                        PackRegistryError::Store(format!("a stored pack id is malformed: {error}"))
                    })?,
                    name: pack.name,
                    version: pack.version,
                });
            }
            let inserted = match sqlx::query(
                "INSERT INTO catalog.packs
                     (profile_id, pack_id, kind, name, version, owner, state, object_key,
                      sha256, bytes, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, 'draft', $7, $8, $9, $10::timestamptz)
                 ON CONFLICT (profile_id, pack_id) DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(&pack_id)
            .bind(pack.kind.code())
            .bind(&pack.name)
            .bind(&pack.version)
            .bind(&pack.owner)
            .bind(&pack.object_key)
            .bind(&pack_id)
            .bind(i64::try_from(pack.bytes).unwrap_or(i64::MAX))
            .bind(&pack.created_at)
            .execute(&mut *transaction)
            .await
            {
                Ok(inserted) => inserted,
                Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23505") => {
                    // A concurrent registrar took the label after the
                    // transaction's check. The aborted transaction cannot be
                    // queried again, so roll it back and read the winner.
                    transaction
                        .rollback()
                        .await
                        .map_err(|error| PackRegistryError::Store(error.to_string()))?;
                    let existing: Option<(String,)> = sqlx::query_as(
                        "SELECT pack_id FROM catalog.packs
                          WHERE profile_id = $1 AND name = $2 AND version = $3",
                    )
                    .bind(profile.as_str())
                    .bind(&pack.name)
                    .bind(&pack.version)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
                    return match existing {
                        Some((existing,)) if existing != pack_id => {
                            Err(PackRegistryError::LabelConflict {
                                existing: Digest::parse(&existing).map_err(|error| {
                                    PackRegistryError::Store(format!(
                                        "a stored pack id is malformed: {error}"
                                    ))
                                })?,
                                name: pack.name,
                                version: pack.version,
                            })
                        }
                        _ => Err(PackRegistryError::Store(error.to_string())),
                    };
                }
                Err(error) => return Err(PackRegistryError::Store(error.to_string())),
            };
            let created = inserted.rows_affected() == 1;
            if created {
                sqlx::query(
                    "INSERT INTO catalog.pack_lifecycle_events
                         (profile_id, pack_id, from_state, to_state, action, created_at)
                     VALUES ($1, $2, NULL, 'draft', 'create', $3::timestamptz)",
                )
                .bind(profile.as_str())
                .bind(&pack_id)
                .bind(&pack.created_at)
                .execute(&mut *transaction)
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            }
            transaction
                .commit()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let record = load_record(&pool, &profile, &pack_id)
                .await?
                .ok_or(PackRegistryError::NotFound)?;
            Ok(RegisterOutcome { record, created })
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        pack_id: String,
    ) -> BoxFuture<Result<Option<PackRecord>, PackRegistryError>> {
        let pool = self.pool.clone();
        Box::pin(async move { load_record(&pool, &profile, &pack_id).await })
    }

    fn list(
        &self,
        profile: ProfileId,
        filter: PackFilter,
    ) -> BoxFuture<Result<Vec<PackSummary>, PackRegistryError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<SummaryRow> = sqlx::query_as(LIST_SELECT)
                .bind(profile.as_str())
                .bind(filter.kind.map(PackKind::code))
                .bind(filter.state.map(PackState::code))
                .fetch_all(&pool)
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            rows.iter().map(summary_of).collect()
        })
    }

    fn validate(
        &self,
        profile: ProfileId,
        pack_id: String,
        validated_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let state: Option<(String,)> = sqlx::query_as(
                "SELECT state FROM catalog.packs
                  WHERE profile_id = $1 AND pack_id = $2 FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(&pack_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let state = state.ok_or(PackRegistryError::NotFound)?.0;
            let state = PackState::from_code(&state).ok_or_else(|| {
                PackRegistryError::Store(format!("a stored pack has an unknown state: {state}"))
            })?;
            match state {
                PackState::Frozen => return Err(PackRegistryError::Frozen),
                PackState::Retired => return Err(PackRegistryError::Retired),
                PackState::Draft | PackState::Validated => {}
            }
            sqlx::query(
                "INSERT INTO catalog.pack_validation_records
                     (profile_id, pack_id, validator, valid, message, created_at)
                 VALUES ($1, $2, $3, true, NULL, $4::timestamptz)",
            )
            .bind(profile.as_str())
            .bind(&pack_id)
            .bind(VALIDATOR)
            .bind(&validated_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            if state == PackState::Draft {
                sqlx::query(
                    "UPDATE catalog.packs SET state = 'validated', validated_at = $3::timestamptz
                      WHERE profile_id = $1 AND pack_id = $2",
                )
                .bind(profile.as_str())
                .bind(&pack_id)
                .bind(&validated_at)
                .execute(&mut *transaction)
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
                sqlx::query(
                    "INSERT INTO catalog.pack_lifecycle_events
                         (profile_id, pack_id, from_state, to_state, action, created_at)
                     VALUES ($1, $2, 'draft', 'validated', 'validate', $3::timestamptz)",
                )
                .bind(profile.as_str())
                .bind(&pack_id)
                .bind(&validated_at)
                .execute(&mut *transaction)
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            }
            transaction
                .commit()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            load_record(&pool, &profile, &pack_id)
                .await?
                .ok_or(PackRegistryError::NotFound)
        })
    }

    fn freeze(
        &self,
        profile: ProfileId,
        pack_id: String,
        frozen_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let state: Option<(String,)> = sqlx::query_as(
                "SELECT state FROM catalog.packs
                  WHERE profile_id = $1 AND pack_id = $2 FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(&pack_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let state = state.ok_or(PackRegistryError::NotFound)?.0;
            let state = PackState::from_code(&state).ok_or_else(|| {
                PackRegistryError::Store(format!("a stored pack has an unknown state: {state}"))
            })?;
            match state {
                PackState::Frozen => {}
                PackState::Retired => return Err(PackRegistryError::Retired),
                PackState::Draft => {
                    return Err(PackRegistryError::NotValidated(PackState::Draft));
                }
                PackState::Validated => {
                    sqlx::query(
                        "UPDATE catalog.packs SET state = 'frozen', frozen_at = $3::timestamptz
                          WHERE profile_id = $1 AND pack_id = $2",
                    )
                    .bind(profile.as_str())
                    .bind(&pack_id)
                    .bind(&frozen_at)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
                    sqlx::query(
                        "INSERT INTO catalog.pack_lifecycle_events
                             (profile_id, pack_id, from_state, to_state, action, created_at)
                         VALUES ($1, $2, 'validated', 'frozen', 'freeze', $3::timestamptz)",
                    )
                    .bind(profile.as_str())
                    .bind(&pack_id)
                    .bind(&frozen_at)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
                }
            }
            transaction
                .commit()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            load_record(&pool, &profile, &pack_id)
                .await?
                .ok_or(PackRegistryError::NotFound)
        })
    }

    fn retire(
        &self,
        profile: ProfileId,
        pack_id: String,
        retired_at: String,
    ) -> BoxFuture<Result<PackRecord, PackRegistryError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let state: Option<(String,)> = sqlx::query_as(
                "SELECT state FROM catalog.packs
                  WHERE profile_id = $1 AND pack_id = $2 FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(&pack_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            let state = state.ok_or(PackRegistryError::NotFound)?.0;
            let state = PackState::from_code(&state).ok_or_else(|| {
                PackRegistryError::Store(format!("a stored pack has an unknown state: {state}"))
            })?;
            match state {
                PackState::Retired => {}
                PackState::Frozen => {
                    sqlx::query(
                        "UPDATE catalog.packs SET state = 'retired', retired_at = $3::timestamptz
                          WHERE profile_id = $1 AND pack_id = $2",
                    )
                    .bind(profile.as_str())
                    .bind(&pack_id)
                    .bind(&retired_at)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
                    sqlx::query(
                        "INSERT INTO catalog.pack_lifecycle_events
                             (profile_id, pack_id, from_state, to_state, action, created_at)
                         VALUES ($1, $2, 'frozen', 'retired', 'retire', $3::timestamptz)",
                    )
                    .bind(profile.as_str())
                    .bind(&pack_id)
                    .bind(&retired_at)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| PackRegistryError::Store(error.to_string()))?;
                }
                other => return Err(PackRegistryError::NotFrozen(other)),
            }
            transaction
                .commit()
                .await
                .map_err(|error| PackRegistryError::Store(error.to_string()))?;
            load_record(&pool, &profile, &pack_id)
                .await?
                .ok_or(PackRegistryError::NotFound)
        })
    }
}
