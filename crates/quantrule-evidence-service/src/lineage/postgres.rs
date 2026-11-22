//! The Postgres run lineage store.
//!
//! `runs.run_lineage` is the header and `runs.run_lineage_references` is the
//! immutable set. The row is written in one transaction with the run visibility
//! check; a repeat with the same set returns the stored row, and a different set
//! is refused. The foreign key removes the lineage with its run.

use quantrule_api::{
    Digest, PackProvenance, ProfileId, ReferenceKind, RunArtifactReference, RunId, RunLineage,
};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::{LineageError, RunLineageStore, normalize};

/// A run lineage store backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgLineage {
    pool: PgPool,
}

impl PgLineage {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

async fn visible(pool: &PgPool, profile: &ProfileId, run_id: &RunId) -> Result<bool, LineageError> {
    let row: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM runs.submissions WHERE profile_id = $1 AND run_id = $2")
            .bind(profile.as_str())
            .bind(run_id.to_string())
            .fetch_optional(pool)
            .await
            .map_err(|error| LineageError::Store(error.to_string()))?;
    Ok(row.is_some())
}

async fn load(pool: &PgPool, run_id: &RunId) -> Result<Option<RunLineage>, LineageError> {
    let header: Option<(String,)> = sqlx::query_as(
        "SELECT to_char(recorded_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
           FROM runs.run_lineage WHERE run_id = $1",
    )
    .bind(run_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|error| LineageError::Store(error.to_string()))?;
    let Some((recorded_at,)) = header else {
        return Ok(None);
    };
    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT reference_kind, reference_id, reference_hash, pack::text
           FROM runs.run_lineage_references
          WHERE run_id = $1
          ORDER BY reference_kind",
    )
    .bind(run_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|error| LineageError::Store(error.to_string()))?;
    let mut references = Vec::with_capacity(rows.len());
    for (kind, id, hash, pack) in rows {
        let kind = ReferenceKind::from_code(&kind).ok_or_else(|| {
            LineageError::Store(format!("a stored lineage has an unknown kind: {kind}"))
        })?;
        let id = Digest::parse(&id).map_err(|error| {
            LineageError::Store(format!("a stored lineage id is malformed: {error}"))
        })?;
        let hash = Digest::parse(&hash).map_err(|error| {
            LineageError::Store(format!("a stored lineage hash is malformed: {error}"))
        })?;
        let pack = pack
            .map(|text| {
                serde_json::from_str::<PackProvenance>(&text).map_err(|error| {
                    LineageError::Store(format!("a stored pack provenance is malformed: {error}"))
                })
            })
            .transpose()?;
        references.push(RunArtifactReference {
            kind,
            id,
            hash,
            pack,
        });
    }
    references.sort_by_key(|reference| reference.kind);
    Ok(Some(RunLineage {
        run_id: run_id.clone(),
        references,
        recorded_at,
    }))
}

impl RunLineageStore for PgLineage {
    fn record(
        &self,
        profile: ProfileId,
        run_id: RunId,
        references: Vec<RunArtifactReference>,
        recorded_at: String,
    ) -> BoxFuture<Result<RunLineage, LineageError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let references = normalize(references)?;
            if !visible(&pool, &profile, &run_id).await? {
                return Err(LineageError::Missing);
            }
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| LineageError::Store(error.to_string()))?;
            let existing: Option<(String,)> = sqlx::query_as(
                "SELECT recorded_at::text FROM runs.run_lineage WHERE run_id = $1 FOR UPDATE",
            )
            .bind(run_id.to_string())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| LineageError::Store(error.to_string()))?;
            if existing.is_some() {
                transaction
                    .commit()
                    .await
                    .map_err(|error| LineageError::Store(error.to_string()))?;
                let stored = load(&pool, &run_id).await?.ok_or(LineageError::Missing)?;
                if stored.references == references {
                    return Ok(stored);
                }
                return Err(LineageError::Conflict);
            }
            let inserted = sqlx::query(
                "INSERT INTO runs.run_lineage (run_id, recorded_at)
                 VALUES ($1, $2::timestamptz)
                 ON CONFLICT (run_id) DO NOTHING",
            )
            .bind(run_id.to_string())
            .bind(&recorded_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| LineageError::Store(error.to_string()))?;
            if inserted.rows_affected() == 0 {
                transaction
                    .commit()
                    .await
                    .map_err(|error| LineageError::Store(error.to_string()))?;
                let stored = load(&pool, &run_id).await?.ok_or(LineageError::Missing)?;
                if stored.references == references {
                    return Ok(stored);
                }
                return Err(LineageError::Conflict);
            }
            for reference in &references {
                let pack = reference
                    .pack
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(|error| LineageError::Store(error.to_string()))?;
                sqlx::query(
                    "INSERT INTO runs.run_lineage_references
                         (run_id, reference_kind, reference_id, reference_hash, pack)
                     VALUES ($1, $2, $3, $4, $5::jsonb)",
                )
                .bind(run_id.to_string())
                .bind(reference.kind.code())
                .bind(reference.id.to_string())
                .bind(reference.hash.to_string())
                .bind(pack)
                .execute(&mut *transaction)
                .await
                .map_err(|error| LineageError::Store(error.to_string()))?;
            }
            transaction
                .commit()
                .await
                .map_err(|error| LineageError::Store(error.to_string()))?;
            load(&pool, &run_id).await?.ok_or(LineageError::Missing)
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        run_id: RunId,
    ) -> BoxFuture<Result<Option<RunLineage>, LineageError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            if !visible(&pool, &profile, &run_id).await? {
                return Err(LineageError::Missing);
            }
            load(&pool, &run_id).await
        })
    }
}
