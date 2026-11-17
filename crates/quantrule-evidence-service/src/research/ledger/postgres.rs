//! The Postgres research ledger.
//!
//! The manifest is stored whole as `jsonb` and never rewritten. A pipeline run
//! and its canonical digest are linked once, and the human decision is recorded
//! once; database triggers enforce both rules in addition to the store. The
//! committed `docs/improvement/` registry remains the human-owned snapshot.

use quantrule_api::{
    BaselineRecord, Digest, ImprovementDecision, ImprovementManifest, ImprovementRunDetail,
    ImprovementRunState, ImprovementRunSummary, ImprovementTier, InvalidationRecord, ProfileId,
    RunId,
};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::store::{CreateOutcome, ImprovementStore, ImprovementStoreError};

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

type DetailRow = (
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

type SummaryRow = (
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

type BaselineRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
    Option<String>,
    String,
);

type InvalidationRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
);

const DETAIL_SELECT: &str = "SELECT improvement_id, manifest::text, pipeline_run_id,
            canonical_digest, state, decision, decision_note,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(decided_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
       FROM research.improvement_runs
      WHERE profile_id = $1 AND improvement_id = $2";

async fn load_detail(
    pool: &PgPool,
    profile: &str,
    improvement_id: &str,
) -> Result<Option<ImprovementRunDetail>, ImprovementStoreError> {
    let row: Option<DetailRow> = sqlx::query_as(DETAIL_SELECT)
        .bind(profile)
        .bind(improvement_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| ImprovementStoreError(error.to_string()))?;
    row.map(detail_from_row).transpose()
}

fn detail_from_row(row: DetailRow) -> Result<ImprovementRunDetail, ImprovementStoreError> {
    let (
        improvement_id,
        manifest,
        pipeline_run_id,
        canonical_digest,
        state,
        decision,
        decision_note,
        created_at,
        submitted_at,
        decided_at,
    ) = row;
    let manifest: ImprovementManifest = serde_json::from_str(&manifest).map_err(|error| {
        ImprovementStoreError(format!("a stored research manifest is malformed: {error}"))
    })?;
    let state = ImprovementRunState::parse(&state).ok_or_else(|| {
        ImprovementStoreError(format!("a stored research state is unknown: {state}"))
    })?;
    let decision = decision
        .as_deref()
        .map(|value| {
            ImprovementDecision::parse(value).ok_or_else(|| {
                ImprovementStoreError(format!("a stored research decision is unknown: {value}"))
            })
        })
        .transpose()?;
    let pipeline_run_id = pipeline_run_id
        .as_deref()
        .map(|value| {
            Digest::parse(value).map(RunId).map_err(|error| {
                ImprovementStoreError(format!("a stored run id is malformed: {error}"))
            })
        })
        .transpose()?;
    let canonical_digest = canonical_digest
        .as_deref()
        .map(|value| {
            Digest::parse(value).map_err(|error| {
                ImprovementStoreError(format!("a stored canonical digest is malformed: {error}"))
            })
        })
        .transpose()?;
    Ok(ImprovementRunDetail {
        improvement_id,
        tier: manifest.tier,
        state,
        decision,
        decision_note,
        manifest,
        pipeline_run_id,
        canonical_digest,
        created_at,
        submitted_at,
        decided_at,
    })
}

fn summary_from_row(row: SummaryRow) -> Result<ImprovementRunSummary, ImprovementStoreError> {
    let (manifest, state, decision, created_at, submitted_at, decided_at) = row;
    let manifest: ImprovementManifest = serde_json::from_str(&manifest).map_err(|error| {
        ImprovementStoreError(format!("a stored research manifest is malformed: {error}"))
    })?;
    let state = ImprovementRunState::parse(&state).ok_or_else(|| {
        ImprovementStoreError(format!("a stored research state is unknown: {state}"))
    })?;
    let decision = decision
        .as_deref()
        .map(|value| {
            ImprovementDecision::parse(value).ok_or_else(|| {
                ImprovementStoreError(format!("a stored research decision is unknown: {value}"))
            })
        })
        .transpose()?;
    Ok(ImprovementRunSummary {
        improvement_id: manifest.improvement_id,
        tier: manifest.tier,
        state,
        decision,
        created_at,
        submitted_at,
        decided_at,
    })
}

fn tier(value: &str) -> Result<ImprovementTier, ImprovementStoreError> {
    ImprovementTier::parse(value)
        .ok_or_else(|| ImprovementStoreError(format!("a stored baseline tier is unknown: {value}")))
}

fn baseline_from_row(row: BaselineRow) -> Result<BaselineRecord, ImprovementStoreError> {
    let (
        baseline_id,
        tier_name,
        run_id,
        improvement_id,
        accepted_by,
        accepted_on,
        scope,
        supersedes,
        created_at,
    ) = row;
    Ok(BaselineRecord {
        baseline_id,
        tier: tier(&tier_name)?,
        run_id,
        improvement_id,
        accepted_by,
        accepted_on,
        scope,
        supersedes,
        created_at,
    })
}

fn invalidation_from_row(
    row: InvalidationRow,
) -> Result<InvalidationRecord, ImprovementStoreError> {
    let (invalidation_id, date, commit, removed, reason, superseded_by, created_at) = row;
    Ok(InvalidationRecord {
        invalidation_id,
        date,
        commit,
        removed,
        reason,
        superseded_by,
        created_at,
    })
}

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

    fn link_pipeline_run(
        &self,
        profile: ProfileId,
        improvement_id: String,
        pipeline_run_id: RunId,
        canonical_digest: Digest,
        submitted_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let run_id = pipeline_run_id.to_string();
            let digest = canonical_digest.to_string();
            let changed = sqlx::query(
                "UPDATE research.improvement_runs
                    SET pipeline_run_id = $3, canonical_digest = $4,
                        state = 'submitted', submitted_at = $5::timestamptz
                  WHERE profile_id = $1 AND improvement_id = $2 AND state = 'created'",
            )
            .bind(&profile_id)
            .bind(&improvement_id)
            .bind(&run_id)
            .bind(&digest)
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
                  WHERE profile_id = $1 AND improvement_id = $2 AND state = 'submitted'",
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
                        to_char(decided_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
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
                  ORDER BY accepted_on DESC, baseline_id",
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
