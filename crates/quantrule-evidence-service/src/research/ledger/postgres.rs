//! The Postgres research ledger.
//!
//! The manifest is stored whole as `jsonb` and never rewritten. A pipeline run
//! is linked once, the canonical digest is recorded once when it is available,
//! and the human decision is recorded once; database triggers enforce both rules
//! in addition to the store. A non-accepted decision may omit the digest. The
//! baseline and invalidation registries are append-only. The committed
//! `docs/improvement/` registry remains the human-owned snapshot.

use quantrule_api::{
    BaselineRecord, Digest, ImprovementDecision, ImprovementManifest, ImprovementRunDetail,
    ImprovementRunState, ImprovementRunSummary, InvalidationRecord, ProfileId, RunId,
};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::postgres_rows::{
    BaselineRow, InvalidationRow, SummaryRow, baseline_from_row, invalidation_from_row,
    load_baseline, load_detail, load_invalidation, summary_from_row,
};
use super::store::{
    CreateOutcome, DecisionWrite, ImprovementStore, ImprovementStoreError, ResearchArchiveOutcome,
    ResearchDatasetReference, ResearchEraseOutcome, ResearchEraseReceipt, validate_baseline,
    validate_decision_registry, validate_invalidation,
};

/// A research ledger backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgImprovementStore {
    pool: PgPool,
}

impl PgImprovementStore {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

pub(crate) type DetailRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

impl ImprovementStore for PgImprovementStore {
    fn create(
        &self,
        profile: ProfileId,
        manifest: ImprovementManifest,
        created_at: String,
    ) -> BoxFuture<Result<CreateOutcome, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            manifest
                .validate()
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let profile_id = profile.as_str().to_owned();
            let improvement_id = manifest.improvement_id.clone();
            let body = serde_json::to_string(&manifest)
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let inserted = sqlx::query(
                "INSERT INTO research.improvement_runs
                     (profile_id, improvement_id, manifest, state, created_at)
                 VALUES ($1, $2, $3::jsonb, 'created', $4::timestamptz)
                 ON CONFLICT (profile_id, improvement_id) DO NOTHING",
            )
            .bind(&profile_id)
            .bind(&improvement_id)
            .bind(&body)
            .bind(&created_at)
            .execute(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?
            .rows_affected();
            let existing = load_detail(&pool, &profile_id, &improvement_id)
                .await?
                .ok_or_else(|| {
                    ImprovementStoreError("the research record vanished after create".to_owned())
                })?;
            if inserted == 1 {
                Ok(CreateOutcome::Created(existing))
            } else if existing.manifest == manifest {
                Ok(CreateOutcome::Existing(existing))
            } else {
                Err(ImprovementStoreError(
                    "the improvement id already names another manifest".to_owned(),
                ))
            }
        })
    }

    fn link_run(
        &self,
        profile: ProfileId,
        improvement_id: String,
        pipeline_run_id: RunId,
        submitted_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let run_id = pipeline_run_id.to_string();
            let changed = sqlx::query(
                "UPDATE research.improvement_runs
                    SET pipeline_run_id = $3, state = 'submitted',
                        submitted_at = $4::timestamptz
                  WHERE profile_id = $1 AND improvement_id = $2 AND state = 'created'",
            )
            .bind(&profile_id)
            .bind(&improvement_id)
            .bind(&run_id)
            .bind(&submitted_at)
            .execute(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?
            .rows_affected();
            let detail = load_detail(&pool, &profile_id, &improvement_id)
                .await?
                .ok_or_else(|| {
                    ImprovementStoreError("the improvement record is unknown".to_owned())
                })?;
            if changed == 1 {
                return Ok(detail);
            }
            let reason = match detail.state {
                ImprovementRunState::Created => "the pipeline run was not linked",
                ImprovementRunState::Submitted => "the pipeline run is already linked",
                ImprovementRunState::Decided => "the improvement record is already decided",
            };
            Err(ImprovementStoreError(reason.to_owned()))
        })
    }

    fn link_digest(
        &self,
        profile: ProfileId,
        improvement_id: String,
        canonical_digest: Digest,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let digest = canonical_digest.to_string();
            let changed = sqlx::query(
                "UPDATE research.improvement_runs
                    SET canonical_digest = $3
                  WHERE profile_id = $1 AND improvement_id = $2
                    AND state <> 'created' AND canonical_digest IS NULL",
            )
            .bind(&profile_id)
            .bind(&improvement_id)
            .bind(&digest)
            .execute(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?
            .rows_affected();
            let detail = load_detail(&pool, &profile_id, &improvement_id)
                .await?
                .ok_or_else(|| {
                    ImprovementStoreError("the improvement record is unknown".to_owned())
                })?;
            if changed == 1 {
                return Ok(detail);
            }
            let reason = match detail.state {
                ImprovementRunState::Created => "a canonical digest needs a submitted pipeline run",
                ImprovementRunState::Submitted | ImprovementRunState::Decided
                    if detail.canonical_digest.is_some() =>
                {
                    "the canonical digest is already recorded"
                }
                _ => "the canonical digest was not recorded",
            };
            Err(ImprovementStoreError(reason.to_owned()))
        })
    }

    fn decide_with_registry(
        &self,
        profile: ProfileId,
        improvement_id: String,
        write: DecisionWrite,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let DecisionWrite {
                decision,
                note,
                decided_at,
                baseline,
                invalidation,
            } = write;
            if note.trim().is_empty() {
                return Err(ImprovementStoreError(
                    "a decision needs a non-empty note".to_owned(),
                ));
            }
            validate_decision_registry(decision, baseline.as_ref(), invalidation.as_ref())?;
            let profile_id = profile.as_str().to_owned();
            let mut tx = pool
                .begin()
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let row: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT state, canonical_digest FROM research.improvement_runs
                  WHERE profile_id = $1 AND improvement_id = $2 FOR UPDATE",
            )
            .bind(&profile_id)
            .bind(&improvement_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let Some((state, digest)) = row else {
                return Err(ImprovementStoreError(
                    "the improvement record is unknown".to_owned(),
                ));
            };
            if state == ImprovementRunState::Created.as_str() {
                return Err(ImprovementStoreError(
                    "a decision needs a submitted pipeline run".to_owned(),
                ));
            }
            if state == ImprovementRunState::Decided.as_str() {
                return Err(ImprovementStoreError(
                    "the decision is already recorded".to_owned(),
                ));
            }
            if decision == ImprovementDecision::Accepted && digest.is_none() {
                return Err(ImprovementStoreError(
                    "an accepted decision needs the canonical digest".to_owned(),
                ));
            }
            if let Some(baseline) = &baseline {
                let existing: Option<(i32,)> = sqlx::query_as(
                    "SELECT 1 FROM research.baselines
                      WHERE profile_id = $1 AND baseline_id = $2",
                )
                .bind(&profile_id)
                .bind(&baseline.baseline_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
                if existing.is_some() {
                    return Err(ImprovementStoreError(
                        "the baseline id already exists".to_owned(),
                    ));
                }
            }
            if let Some(invalidation) = &invalidation {
                let existing: Option<(i32,)> = sqlx::query_as(
                    "SELECT 1 FROM research.invalidations
                      WHERE profile_id = $1 AND invalidation_id = $2",
                )
                .bind(&profile_id)
                .bind(&invalidation.invalidation_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
                if existing.is_some() {
                    return Err(ImprovementStoreError(
                        "the invalidation id already exists".to_owned(),
                    ));
                }
            }
            let changed = sqlx::query(
                "UPDATE research.improvement_runs
                    SET state = 'decided', decision = $3, decision_note = $4,
                        decided_at = $5::timestamptz
                  WHERE profile_id = $1 AND improvement_id = $2 AND state = 'submitted'",
            )
            .bind(&profile_id)
            .bind(&improvement_id)
            .bind(decision.as_str())
            .bind(&note)
            .bind(&decided_at)
            .execute(&mut *tx)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?
            .rows_affected();
            if changed != 1 {
                return Err(ImprovementStoreError(
                    "the decision was not recorded".to_owned(),
                ));
            }
            if let Some(baseline) = baseline {
                sqlx::query(
                    "INSERT INTO research.baselines
                         (profile_id, baseline_id, tier, run_id, improvement_id,
                          accepted_by, accepted_on, scope, supersedes, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7::date, $8, $9, $10::timestamptz)",
                )
                .bind(&profile_id)
                .bind(&baseline.baseline_id)
                .bind(baseline.tier.as_str())
                .bind(baseline.run_id.as_deref())
                .bind(baseline.improvement_id.as_deref())
                .bind(&baseline.accepted_by)
                .bind(&baseline.accepted_on)
                .bind(&baseline.scope)
                .bind(baseline.supersedes.as_deref())
                .bind(&baseline.created_at)
                .execute(&mut *tx)
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            }
            if let Some(invalidation) = invalidation {
                sqlx::query(
                    "INSERT INTO research.invalidations
                         (profile_id, invalidation_id, date, \"commit\", removed,
                          reason, superseded_by, created_at)
                     VALUES ($1, $2, $3::date, $4, $5, $6, $7, $8::timestamptz)",
                )
                .bind(&profile_id)
                .bind(&invalidation.invalidation_id)
                .bind(&invalidation.date)
                .bind(&invalidation.commit)
                .bind(&invalidation.removed)
                .bind(&invalidation.reason)
                .bind(invalidation.superseded_by.as_deref())
                .bind(&invalidation.created_at)
                .execute(&mut *tx)
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            }
            tx.commit()
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            load_detail(&pool, &profile_id, &improvement_id)
                .await?
                .ok_or_else(|| {
                    ImprovementStoreError("the improvement record is unknown".to_owned())
                })
        })
    }

    fn decide(
        &self,
        profile: ProfileId,
        improvement_id: String,
        decision: ImprovementDecision,
        note: String,
        decided_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            if note.trim().is_empty() {
                return Err(ImprovementStoreError(
                    "a decision needs a non-empty note".to_owned(),
                ));
            }
            let profile_id = profile.as_str().to_owned();
            let changed = sqlx::query(
                "UPDATE research.improvement_runs
                    SET state = 'decided', decision = $3, decision_note = $4,
                        decided_at = $5::timestamptz
                  WHERE profile_id = $1 AND improvement_id = $2 AND state = 'submitted'
                    AND ($3 <> 'accepted' OR canonical_digest IS NOT NULL)",
            )
            .bind(&profile_id)
            .bind(&improvement_id)
            .bind(decision.as_str())
            .bind(&note)
            .bind(&decided_at)
            .execute(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?
            .rows_affected();
            let detail = load_detail(&pool, &profile_id, &improvement_id)
                .await?
                .ok_or_else(|| {
                    ImprovementStoreError("the improvement record is unknown".to_owned())
                })?;
            if changed == 1 {
                return Ok(detail);
            }
            let reason = match detail.state {
                ImprovementRunState::Created => "a decision needs a submitted pipeline run",
                ImprovementRunState::Submitted
                    if decision == ImprovementDecision::Accepted
                        && detail.canonical_digest.is_none() =>
                {
                    "an accepted decision needs the canonical digest"
                }
                ImprovementRunState::Submitted => "the decision was not recorded",
                ImprovementRunState::Decided => "the decision is already recorded",
            };
            Err(ImprovementStoreError(reason.to_owned()))
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        limit: usize,
    ) -> BoxFuture<Result<Vec<ImprovementRunSummary>, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).map_err(|_| {
                ImprovementStoreError("the list limit does not fit an integer".to_owned())
            })?;
            let rows: Vec<SummaryRow> = sqlx::query_as(
                "SELECT manifest::text, state, decision,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(decided_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                   FROM research.improvement_runs
                  WHERE profile_id = $1 AND archived_at IS NULL
                  ORDER BY created_at DESC, improvement_id
                  LIMIT $2",
            )
            .bind(profile.as_str())
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter().map(summary_from_row).collect()
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
        limit: usize,
    ) -> BoxFuture<Result<Vec<ImprovementRunSummary>, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).map_err(|_| {
                ImprovementStoreError("the list limit does not fit an integer".to_owned())
            })?;
            let rows: Vec<SummaryRow> = sqlx::query_as(
                "SELECT manifest::text, state, decision,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(decided_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                   FROM research.improvement_runs
                  WHERE profile_id = $1
                  ORDER BY created_at DESC, improvement_id
                  LIMIT $2",
            )
            .bind(profile.as_str())
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter().map(summary_from_row).collect()
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        improvement_id: String,
    ) -> BoxFuture<Result<Option<ImprovementRunDetail>, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move { load_detail(&pool, profile.as_str(), &improvement_id).await })
    }

    fn archive(
        &self,
        profile: ProfileId,
        improvement_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<ResearchArchiveOutcome, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE research.improvement_runs SET archived_at = $3::timestamptz
                  WHERE profile_id = $1 AND improvement_id = $2 AND archived_at IS NULL",
            )
            .bind(profile.as_str())
            .bind(&improvement_id)
            .bind(&archived_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT improvement_id FROM research.improvement_runs
                      WHERE profile_id = $1 AND improvement_id = $2",
                )
                .bind(profile.as_str())
                .bind(&improvement_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
                transaction
                    .rollback()
                    .await
                    .map_err(|error| ImprovementStoreError(error.to_string()))?;
                return Ok(if exists.is_some() {
                    ResearchArchiveOutcome::AlreadyArchived
                } else {
                    ResearchArchiveOutcome::Missing
                });
            }
            sqlx::query(
                "INSERT INTO research.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'research', $2, 'archive', 'operator.archive', 0)",
            )
            .bind(profile.as_str())
            .bind(&improvement_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let row: Option<SummaryRow> = sqlx::query_as(
                "SELECT manifest::text, state, decision,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(decided_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                   FROM research.improvement_runs
                  WHERE profile_id = $1 AND improvement_id = $2",
            )
            .bind(profile.as_str())
            .bind(&improvement_id)
            .fetch_optional(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let row = row.ok_or_else(|| {
                ImprovementStoreError("the research record vanished after archive".to_owned())
            })?;
            Ok(ResearchArchiveOutcome::Archived(summary_from_row(row)?))
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        improvement_id: String,
    ) -> BoxFuture<Result<ResearchEraseOutcome, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let row: Option<(Option<String>, String, String)> = sqlx::query_as(
                "SELECT to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        state, manifest::text
                   FROM research.improvement_runs
                  WHERE profile_id = $1 AND improvement_id = $2",
            )
            .bind(profile.as_str())
            .bind(&improvement_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let Some((archived_at, state, manifest)) = row else {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| ImprovementStoreError(error.to_string()))?;
                return Ok(ResearchEraseOutcome::Missing);
            };
            if archived_at.is_none() {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| ImprovementStoreError(error.to_string()))?;
                return Ok(ResearchEraseOutcome::NotArchived);
            }
            let state = ImprovementRunState::parse(&state).ok_or_else(|| {
                ImprovementStoreError(format!("a stored research state is unknown: {state}"))
            })?;
            if state == ImprovementRunState::Decided {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| ImprovementStoreError(error.to_string()))?;
                return Ok(ResearchEraseOutcome::Protected);
            }
            sqlx::query(
                "DELETE FROM research.improvement_runs
                  WHERE profile_id = $1 AND improvement_id = $2",
            )
            .bind(profile.as_str())
            .bind(&improvement_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            sqlx::query(
                "INSERT INTO research.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'research', $2, 'erase', 'operator.erase', 0)",
            )
            .bind(profile.as_str())
            .bind(&improvement_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let manifest: ImprovementManifest =
                serde_json::from_str(&manifest).map_err(|error| {
                    ImprovementStoreError(format!(
                        "a stored research manifest is malformed: {error}"
                    ))
                })?;
            Ok(ResearchEraseOutcome::Erased(ResearchEraseReceipt {
                improvement_id,
                data_version: manifest.data_version,
            }))
        })
    }

    fn submitted_datasets(
        &self,
    ) -> BoxFuture<Result<Vec<ResearchDatasetReference>, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT profile_id, improvement_id, manifest::text
                   FROM research.improvement_runs
                  ORDER BY profile_id, improvement_id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter()
                .map(|(profile, improvement_id, manifest)| {
                    let manifest: ImprovementManifest =
                        serde_json::from_str(&manifest).map_err(|error| {
                            ImprovementStoreError(format!(
                                "a stored research manifest is malformed: {error}"
                            ))
                        })?;
                    Ok(ResearchDatasetReference {
                        profile: ProfileId::parse(&profile).map_err(|error| {
                            ImprovementStoreError(format!("a stored profile is malformed: {error}"))
                        })?,
                        improvement_id,
                        data_version: manifest.data_version,
                    })
                })
                .collect()
        })
    }

    fn add_baseline(
        &self,
        profile: ProfileId,
        record: BaselineRecord,
    ) -> BoxFuture<Result<BaselineRecord, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            validate_baseline(&record)?;
            let profile_id = profile.as_str().to_owned();
            let inserted = sqlx::query(
                "INSERT INTO research.baselines
                     (profile_id, baseline_id, tier, run_id, improvement_id,
                      accepted_by, accepted_on, scope, supersedes, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7::date, $8, $9, $10::timestamptz)
                 ON CONFLICT (profile_id, baseline_id) DO NOTHING",
            )
            .bind(&profile_id)
            .bind(&record.baseline_id)
            .bind(record.tier.as_str())
            .bind(record.run_id.as_deref())
            .bind(record.improvement_id.as_deref())
            .bind(&record.accepted_by)
            .bind(&record.accepted_on)
            .bind(&record.scope)
            .bind(record.supersedes.as_deref())
            .bind(&record.created_at)
            .execute(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?
            .rows_affected();
            if inserted == 0 {
                return Err(ImprovementStoreError(
                    "the baseline id already exists".to_owned(),
                ));
            }
            load_baseline(&pool, &profile_id, &record.baseline_id)
                .await?
                .ok_or_else(|| {
                    ImprovementStoreError("the baseline vanished after append".to_owned())
                })
        })
    }

    fn add_invalidation(
        &self,
        profile: ProfileId,
        record: InvalidationRecord,
    ) -> BoxFuture<Result<InvalidationRecord, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            validate_invalidation(&record)?;
            let profile_id = profile.as_str().to_owned();
            let inserted = sqlx::query(
                "INSERT INTO research.invalidations
                     (profile_id, invalidation_id, date, \"commit\", removed,
                      reason, superseded_by, created_at)
                 VALUES ($1, $2, $3::date, $4, $5, $6, $7, $8::timestamptz)
                 ON CONFLICT (profile_id, invalidation_id) DO NOTHING",
            )
            .bind(&profile_id)
            .bind(&record.invalidation_id)
            .bind(&record.date)
            .bind(&record.commit)
            .bind(&record.removed)
            .bind(&record.reason)
            .bind(record.superseded_by.as_deref())
            .bind(&record.created_at)
            .execute(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?
            .rows_affected();
            if inserted == 0 {
                return Err(ImprovementStoreError(
                    "the invalidation id already exists".to_owned(),
                ));
            }
            load_invalidation(&pool, &profile_id, &record.invalidation_id)
                .await?
                .ok_or_else(|| {
                    ImprovementStoreError("the invalidation vanished after append".to_owned())
                })
        })
    }

    fn baselines(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<BaselineRecord>, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<BaselineRow> = sqlx::query_as(
                "SELECT baseline_id, tier, run_id, improvement_id, accepted_by,
                        to_char(accepted_on, 'YYYY-MM-DD'), scope, supersedes,
                        to_char(created_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                   FROM research.baselines
                  WHERE profile_id = $1
                  ORDER BY accepted_on DESC, created_at DESC, baseline_id",
            )
            .bind(profile.as_str())
            .fetch_all(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter().map(baseline_from_row).collect()
        })
    }

    fn invalidations(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<InvalidationRecord>, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<InvalidationRow> = sqlx::query_as(
                "SELECT invalidation_id, to_char(date, 'YYYY-MM-DD'), \"commit\",
                        removed, reason, superseded_by,
                        to_char(created_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                   FROM research.invalidations
                  WHERE profile_id = $1
                  ORDER BY date DESC, invalidation_id",
            )
            .bind(profile.as_str())
            .fetch_all(&pool)
            .await
            .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter().map(invalidation_from_row).collect()
        })
    }
}
