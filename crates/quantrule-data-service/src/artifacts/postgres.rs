//! The Postgres artifact catalog.
//!
//! `catalog.artifacts` is the profile-scoped metadata row, `artifact_references`
//! is the consumer promise that protects it, and the existing
//! `catalog.retention_events` is the tombstone that survives the row. The
//! retention pass locks the eligible rows, deletes them, checks object sharing,
//! and writes the tombstone in one transaction. The caller deletes an unshared
//! object afterwards.

use quantrule_api::{
    ArtifactKind, ArtifactRecord, ArtifactReference, ArtifactSummary, CollectedArtifact,
    DatasetReferenceKind, Digest, ProfileId,
};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::store::{ArtifactCatalog, ArtifactError, ArtifactRegisterOutcome, NewArtifact};

/// An artifact catalog backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgArtifacts {
    pool: PgPool,
}

impl PgArtifacts {
    /// Build the catalog over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type ArtifactRow = (String, String, i64, String, String);

fn record_of(row: ArtifactRow) -> Result<ArtifactRecord, ArtifactError> {
    let kind = ArtifactKind::from_code(&row.1).ok_or_else(|| {
        ArtifactError::Store(format!("a stored artifact has an unknown kind: {}", row.1))
    })?;
    let artifact_id = Digest::parse(&row.0).map_err(|error| {
        ArtifactError::Store(format!("a stored artifact id is malformed: {error}"))
    })?;
    let bytes = u64::try_from(row.2)
        .map_err(|_| ArtifactError::Store("a stored artifact size is negative".to_owned()))?;
    Ok(ArtifactRecord {
        summary: ArtifactSummary {
            artifact_id: artifact_id.clone(),
            kind,
            bytes,
            created_at: row.3,
        },
        object_key: row.4,
        sha256: artifact_id,
    })
}

const RECORD_SELECT: &str = "SELECT artifact_id, artifact_kind, bytes,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            object_key
       FROM catalog.artifacts
      WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3";

const LIST_SELECT: &str = "SELECT artifact_id, artifact_kind, bytes,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            object_key
       FROM catalog.artifacts
      WHERE profile_id = $1 AND artifact_kind = $2
      ORDER BY artifact_id";

async fn load(
    pool: &PgPool,
    profile: &ProfileId,
    kind: ArtifactKind,
    artifact_id: &str,
) -> Result<Option<ArtifactRecord>, ArtifactError> {
    let row: Option<ArtifactRow> = sqlx::query_as(RECORD_SELECT)
        .bind(profile.as_str())
        .bind(kind.code())
        .bind(artifact_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| ArtifactError::Store(error.to_string()))?;
    row.map(record_of).transpose()
}

impl ArtifactCatalog for PgArtifacts {
    fn register(
        &self,
        profile: ProfileId,
        artifact: NewArtifact,
    ) -> BoxFuture<Result<ArtifactRegisterOutcome, ArtifactError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let inserted = sqlx::query(
                "INSERT INTO catalog.artifacts
                     (profile_id, artifact_kind, artifact_id, object_key, sha256, bytes,
                      created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7::timestamptz)
                 ON CONFLICT (profile_id, artifact_kind, artifact_id) DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(artifact.kind.code())
            .bind(artifact.artifact_id.to_string())
            .bind(&artifact.object_key)
            .bind(artifact.artifact_id.to_string())
            .bind(i64::try_from(artifact.bytes).unwrap_or(i64::MAX))
            .bind(&artifact.created_at)
            .execute(&pool)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            let record = load(
                &pool,
                &profile,
                artifact.kind,
                &artifact.artifact_id.to_string(),
            )
            .await?
            .ok_or(ArtifactError::NotFound)?;
            Ok(ArtifactRegisterOutcome {
                record,
                created: inserted.rows_affected() == 1,
            })
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
    ) -> BoxFuture<Result<Option<ArtifactRecord>, ArtifactError>> {
        let pool = self.pool.clone();
        Box::pin(async move { load(&pool, &profile, kind, &artifact_id).await })
    }

    fn list(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
    ) -> BoxFuture<Result<Vec<ArtifactRecord>, ArtifactError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<ArtifactRow> = sqlx::query_as(LIST_SELECT)
                .bind(profile.as_str())
                .bind(kind.code())
                .fetch_all(&pool)
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
            rows.into_iter().map(record_of).collect()
        })
    }

    fn add_reference(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
        reference: ArtifactReference,
    ) -> BoxFuture<Result<u64, ArtifactError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT artifact_id FROM catalog.artifacts
                  WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3
                  FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&artifact_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            if exists.is_none() {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| ArtifactError::Store(error.to_string()))?;
                return Err(ArtifactError::NotFound);
            }
            sqlx::query(
                "INSERT INTO catalog.artifact_references
                     (profile_id, artifact_kind, artifact_id, reference_id, kind, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6::timestamptz)
                 ON CONFLICT (profile_id, artifact_kind, artifact_id, reference_id) DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&artifact_id)
            .bind(&reference.reference_id)
            .bind(reference.kind.as_str())
            .bind(&reference.created_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            let remaining: (i64,) = sqlx::query_as(
                "SELECT count(*) FROM catalog.artifact_references
                  WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&artifact_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
            Ok(u64::try_from(remaining.0).unwrap_or(0))
        })
    }

    fn release_reference(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
        reference_id: String,
    ) -> BoxFuture<Result<u64, ArtifactError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT artifact_id FROM catalog.artifacts
                  WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3
                  FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&artifact_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            if exists.is_none() {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| ArtifactError::Store(error.to_string()))?;
                return Err(ArtifactError::NotFound);
            }
            sqlx::query(
                "DELETE FROM catalog.artifact_references
                  WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3
                    AND reference_id = $4",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&artifact_id)
            .bind(&reference_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            let remaining: (i64,) = sqlx::query_as(
                "SELECT count(*) FROM catalog.artifact_references
                  WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&artifact_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
            Ok(u64::try_from(remaining.0).unwrap_or(0))
        })
    }

    fn references(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        artifact_id: String,
    ) -> BoxFuture<Result<Vec<ArtifactReference>, ArtifactError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            if load(&pool, &profile, kind, &artifact_id).await?.is_none() {
                return Err(ArtifactError::NotFound);
            }
            let rows: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT reference_id, kind,
                        to_char(created_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                   FROM catalog.artifact_references
                  WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3
                  ORDER BY reference_id",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&artifact_id)
            .fetch_all(&pool)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            rows.into_iter()
                .map(|(reference_id, kind, created_at)| {
                    let kind = DatasetReferenceKind::parse(&kind).ok_or_else(|| {
                        ArtifactError::Store(format!(
                            "a stored artifact reference has an unknown kind: {kind}"
                        ))
                    })?;
                    Ok(ArtifactReference {
                        reference_id,
                        kind,
                        created_at,
                    })
                })
                .collect()
        })
    }

    fn collect(
        &self,
        profile: ProfileId,
        kind: ArtifactKind,
        cutoff: String,
        collected_at: String,
    ) -> BoxFuture<Result<Vec<CollectedArtifact>, ArtifactError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
            let rows: Vec<(String, String, i64)> = sqlx::query_as(
                "SELECT a.artifact_id, a.object_key, a.bytes
                   FROM catalog.artifacts a
                  WHERE a.profile_id = $1 AND a.artifact_kind = $2
                    AND a.created_at <= $3::timestamptz
                    AND NOT EXISTS (
                        SELECT 1 FROM catalog.artifact_references r
                         WHERE r.profile_id = a.profile_id
                           AND r.artifact_kind = a.artifact_kind
                           AND r.artifact_id = a.artifact_id)
                  ORDER BY a.artifact_id
                  FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(kind.code())
            .bind(&cutoff)
            .fetch_all(&mut *transaction)
            .await
            .map_err(|error| ArtifactError::Store(error.to_string()))?;
            let mut collected = Vec::with_capacity(rows.len());
            for (artifact_id, object_key, bytes) in rows {
                sqlx::query(
                    "DELETE FROM catalog.artifacts
                      WHERE profile_id = $1 AND artifact_kind = $2 AND artifact_id = $3",
                )
                .bind(profile.as_str())
                .bind(kind.code())
                .bind(&artifact_id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
                let shared: Option<(i32,)> =
                    sqlx::query_as("SELECT 1 FROM catalog.artifacts WHERE object_key = $1 LIMIT 1")
                        .bind(&object_key)
                        .fetch_optional(&mut *transaction)
                        .await
                        .map_err(|error| ArtifactError::Store(error.to_string()))?;
                let bytes = u64::try_from(bytes).map_err(|_| {
                    ArtifactError::Store("a stored artifact size is negative".to_owned())
                })?;
                sqlx::query(
                    "INSERT INTO catalog.retention_events
                         (profile_id, kind, record_id, action, reason, bytes, created_at)
                     VALUES ($1, 'artifact', $2, 'erase', 'retention.collect', $3,
                             $4::timestamptz)",
                )
                .bind(profile.as_str())
                .bind(&artifact_id)
                .bind(i64::try_from(bytes).unwrap_or(i64::MAX))
                .bind(&collected_at)
                .execute(&mut *transaction)
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
                let artifact_id = Digest::parse(&artifact_id).map_err(|error| {
                    ArtifactError::Store(format!("a stored artifact id is malformed: {error}"))
                })?;
                collected.push(CollectedArtifact {
                    artifact_id,
                    object_key,
                    bytes,
                    object_shared: shared.is_some(),
                });
            }
            transaction
                .commit()
                .await
                .map_err(|error| ArtifactError::Store(error.to_string()))?;
            Ok(collected)
        })
    }
}
