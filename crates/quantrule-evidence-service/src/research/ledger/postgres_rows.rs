//! Row types and loaders for the Postgres research ledger.

use quantrule_api::{
    BaselineRecord, Digest, ImprovementDecision, ImprovementManifest, ImprovementRunDetail,
    ImprovementRunState, ImprovementRunSummary, ImprovementTier, InvalidationRecord, RunId,
};
use sqlx::PgPool;

use super::postgres::DetailRow;
use super::store::ImprovementStoreError;

pub(crate) type SummaryRow = (
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(crate) type BaselineRow = (
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

pub(crate) type InvalidationRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
);

pub(crate) const DETAIL_SELECT: &str = "SELECT improvement_id, manifest::text, pipeline_run_id,
            canonical_digest, state, decision, decision_note,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            to_char(decided_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
       FROM research.improvement_runs
      WHERE profile_id = $1 AND improvement_id = $2";

pub(crate) async fn load_detail(
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

pub(crate) fn summary_from_row(
    row: SummaryRow,
) -> Result<ImprovementRunSummary, ImprovementStoreError> {
    let (manifest, state, decision, created_at, submitted_at, decided_at, archived_at) = row;
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
        archived_at,
    })
}

pub(crate) fn tier(value: &str) -> Result<ImprovementTier, ImprovementStoreError> {
    ImprovementTier::parse(value)
        .ok_or_else(|| ImprovementStoreError(format!("a stored baseline tier is unknown: {value}")))
}

pub(crate) fn baseline_from_row(row: BaselineRow) -> Result<BaselineRecord, ImprovementStoreError> {
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

pub(crate) fn invalidation_from_row(
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

pub(crate) async fn load_baseline(
    pool: &PgPool,
    profile: &str,
    baseline_id: &str,
) -> Result<Option<BaselineRecord>, ImprovementStoreError> {
    let row: Option<BaselineRow> = sqlx::query_as(
        "SELECT baseline_id, tier, run_id, improvement_id, accepted_by,
                to_char(accepted_on, 'YYYY-MM-DD'), scope, supersedes,
                to_char(created_at AT TIME ZONE 'UTC',
                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
           FROM research.baselines
          WHERE profile_id = $1 AND baseline_id = $2",
    )
    .bind(profile)
    .bind(baseline_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ImprovementStoreError(error.to_string()))?;
    row.map(baseline_from_row).transpose()
}

pub(crate) async fn load_invalidation(
    pool: &PgPool,
    profile: &str,
    invalidation_id: &str,
) -> Result<Option<InvalidationRecord>, ImprovementStoreError> {
    let row: Option<InvalidationRow> = sqlx::query_as(
        "SELECT invalidation_id, to_char(date, 'YYYY-MM-DD'), \"commit\",
                removed, reason, superseded_by,
                to_char(created_at AT TIME ZONE 'UTC',
                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
           FROM research.invalidations
          WHERE profile_id = $1 AND invalidation_id = $2",
    )
    .bind(profile)
    .bind(invalidation_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ImprovementStoreError(error.to_string()))?;
    row.map(invalidation_from_row).transpose()
}
