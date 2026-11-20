//! Row types and loaders for the SQLite research ledger.

use quantrule_api::{
    Digest, ImprovementDecision, ImprovementManifest, ImprovementRunDetail, ImprovementRunState,
    ImprovementRunSummary, ImprovementTier, RunId,
};
use rusqlite::{OptionalExtension, params};

use crate::sqlite::IMPROVEMENT_RUNS_TABLE;

use super::store::ImprovementStoreError;

#[derive(Debug)]
pub(crate) struct StoredRun {
    pub(crate) improvement_id: String,
    pub(crate) manifest: String,
    pub(crate) pipeline_run_id: Option<String>,
    pub(crate) canonical_digest: Option<String>,
    pub(crate) state: String,
    pub(crate) decision: Option<String>,
    pub(crate) decision_note: Option<String>,
    pub(crate) created_at: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) decided_at: Option<String>,
    pub(crate) archived_at: Option<String>,
}

pub(crate) fn read_run(
    connection: &rusqlite::Connection,
    profile_id: &str,
    improvement_id: &str,
) -> rusqlite::Result<Option<StoredRun>> {
    let sql = format!(
        "SELECT improvement_id, manifest, pipeline_run_id, canonical_digest,
                state, decision, decision_note, created_at, submitted_at, decided_at,
                archived_at
           FROM {IMPROVEMENT_RUNS_TABLE}
          WHERE profile_id = ?1 AND improvement_id = ?2"
    );
    connection
        .query_row(&sql, params![profile_id, improvement_id], |row| {
            Ok(StoredRun {
                improvement_id: row.get(0)?,
                manifest: row.get(1)?,
                pipeline_run_id: row.get(2)?,
                canonical_digest: row.get(3)?,
                state: row.get(4)?,
                decision: row.get(5)?,
                decision_note: row.get(6)?,
                created_at: row.get(7)?,
                submitted_at: row.get(8)?,
                decided_at: row.get(9)?,
                archived_at: row.get(10)?,
            })
        })
        .optional()
}

pub(crate) fn detail_from_stored(
    stored: StoredRun,
) -> Result<ImprovementRunDetail, ImprovementStoreError> {
    let manifest: ImprovementManifest =
        serde_json::from_str(&stored.manifest).map_err(|error| {
            ImprovementStoreError(format!("a stored research manifest is malformed: {error}"))
        })?;
    let state = ImprovementRunState::parse(&stored.state).ok_or_else(|| {
        ImprovementStoreError(format!(
            "a stored research state is unknown: {}",
            stored.state
        ))
    })?;
    let decision = stored
        .decision
        .as_deref()
        .map(|value| {
            ImprovementDecision::parse(value).ok_or_else(|| {
                ImprovementStoreError(format!("a stored research decision is unknown: {value}"))
            })
        })
        .transpose()?;
    let pipeline_run_id = stored
        .pipeline_run_id
        .as_deref()
        .map(|value| {
            Digest::parse(value).map(RunId).map_err(|error| {
                ImprovementStoreError(format!("a stored run id is malformed: {error}"))
            })
        })
        .transpose()?;
    let canonical_digest = stored
        .canonical_digest
        .as_deref()
        .map(|value| {
            Digest::parse(value).map_err(|error| {
                ImprovementStoreError(format!("a stored canonical digest is malformed: {error}"))
            })
        })
        .transpose()?;
    Ok(ImprovementRunDetail {
        improvement_id: stored.improvement_id,
        tier: manifest.tier,
        state,
        decision,
        decision_note: stored.decision_note,
        manifest,
        pipeline_run_id,
        canonical_digest,
        created_at: stored.created_at,
        submitted_at: stored.submitted_at,
        decided_at: stored.decided_at,
    })
}

pub(crate) fn summary_from_stored(
    stored: StoredRun,
) -> Result<ImprovementRunSummary, ImprovementStoreError> {
    let archived_at = stored.archived_at.clone();
    let mut summary = detail_from_stored(stored)?.summary();
    summary.archived_at = archived_at;
    Ok(summary)
}

pub(crate) fn tier(value: &str) -> Result<ImprovementTier, ImprovementStoreError> {
    ImprovementTier::parse(value)
        .ok_or_else(|| ImprovementStoreError(format!("a stored baseline tier is unknown: {value}")))
}
