//! The SQLite research ledger.
//!
//! The manifest is stored once as JSON and never rewritten. A pipeline run is
//! linked once, the canonical digest is recorded once when it is available, and
//! the human decision is recorded once; `BEFORE UPDATE` triggers enforce both
//! rules as they do in Postgres. A non-accepted decision may omit the digest.
//! The baseline and invalidation registries are append-only. The committed
//! `docs/improvement/` registry remains the human-owned snapshot.

use quantrule_api::{
    BaselineRecord, Digest, ImprovementDecision, ImprovementManifest, ImprovementRunDetail,
    ImprovementRunState, ImprovementRunSummary, ImprovementTier, InvalidationRecord, ProfileId,
    RunId,
};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use super::sqlite_rows::{StoredRun, detail_from_stored, read_run, summary_from_stored, tier};
use super::store::{
    CreateOutcome, DecisionWrite, ImprovementStore, ImprovementStoreError, ResearchArchiveOutcome,
    ResearchDatasetReference, ResearchEraseOutcome, ResearchEraseReceipt, validate_baseline,
    validate_decision_registry, validate_invalidation,
};
use crate::sqlite::{
    BASELINES_TABLE, IMPROVEMENT_RUNS_TABLE, INVALIDATIONS_TABLE, RESEARCH_RETENTION_TABLE,
};

/// A research ledger backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteImprovementStore {
    database: SqliteDatabase,
}

impl SqliteImprovementStore {
    /// Build the store over an open SQLite database.
    #[must_use]
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }

    /// The database the store reads and writes.
    #[must_use]
    pub fn database(&self) -> &SqliteDatabase {
        &self.database
    }
}

impl ImprovementStore for SqliteImprovementStore {
    fn create(
        &self,
        profile: ProfileId,
        manifest: ImprovementManifest,
        created_at: String,
    ) -> BoxFuture<Result<CreateOutcome, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            manifest
                .validate()
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let owner = profile.as_str().to_owned();
            let key = manifest.improvement_id.clone();
            let body = serde_json::to_string(&manifest)
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let insert_sql = format!(
                "INSERT INTO {IMPROVEMENT_RUNS_TABLE}
                     (profile_id, improvement_id, manifest, state, created_at)
                 VALUES (?1, ?2, ?3, 'created', ?4)
                 ON CONFLICT (profile_id, improvement_id) DO NOTHING"
            );
            let insert_owner = owner.clone();
            let insert_key = key.clone();
            let changed = database
                .call(move |connection| {
                    connection.execute(
                        &insert_sql,
                        params![insert_owner, insert_key, body, created_at],
                    )
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let read_owner = owner;
            let read_key = key;
            let stored = database
                .call(move |connection| read_run(connection, &read_owner, &read_key))
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?
                .ok_or_else(|| {
                    ImprovementStoreError("the research record vanished after create".to_owned())
                })?;
            let detail = detail_from_stored(stored)?;
            if changed == 1 {
                Ok(CreateOutcome::Created(detail))
            } else if detail.manifest == manifest {
                Ok(CreateOutcome::Existing(detail))
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
        let database = self.database.clone();
        Box::pin(async move {
            let owner = profile.as_str().to_owned();
            let update_key = improvement_id.clone();
            let run_id = pipeline_run_id.to_string();
            let sql = format!(
                "UPDATE {IMPROVEMENT_RUNS_TABLE}
                    SET pipeline_run_id = ?3, state = 'submitted', submitted_at = ?4
                  WHERE profile_id = ?1 AND improvement_id = ?2 AND state = 'created'"
            );
            let owner_for_update = owner.clone();
            let changed = database
                .call(move |connection| {
                    connection.execute(
                        &sql,
                        params![owner_for_update, update_key, run_id, submitted_at],
                    )
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let stored = database
                .call(move |connection| read_run(connection, &owner, &improvement_id))
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?
                .ok_or_else(|| {
                    ImprovementStoreError("the improvement record is unknown".to_owned())
                })?;
            let detail = detail_from_stored(stored)?;
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
        let database = self.database.clone();
        Box::pin(async move {
            let owner = profile.as_str().to_owned();
            let update_key = improvement_id.clone();
            let digest = canonical_digest.to_string();
            let sql = format!(
                "UPDATE {IMPROVEMENT_RUNS_TABLE}
                    SET canonical_digest = ?3
                  WHERE profile_id = ?1 AND improvement_id = ?2
                    AND state <> 'created' AND canonical_digest IS NULL"
            );
            let owner_for_update = owner.clone();
            let changed = database
                .call(move |connection| {
                    connection.execute(&sql, params![owner_for_update, update_key, digest])
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let stored = database
                .call(move |connection| read_run(connection, &owner, &improvement_id))
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?
                .ok_or_else(|| {
                    ImprovementStoreError("the improvement record is unknown".to_owned())
                })?;
            let detail = detail_from_stored(stored)?;
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
        let database = self.database.clone();
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
            let owner = profile.as_str().to_owned();
            let key = improvement_id.clone();
            let stored = database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(|error| error.to_string())?;
                    let row: Option<(String, Option<String>)> = transaction
                        .query_row(
                            "SELECT state, canonical_digest FROM research__improvement_runs
                              WHERE profile_id = ?1 AND improvement_id = ?2",
                            params![owner, key],
                            |row| Ok((row.get(0)?, row.get(1)?)),
                        )
                        .optional()
                        .map_err(|error| error.to_string())?;
                    let Some((state, digest)) = row else {
                        return Err("the improvement record is unknown".to_owned());
                    };
                    if state == ImprovementRunState::Created.as_str() {
                        return Err("a decision needs a submitted pipeline run".to_owned());
                    }
                    if state == ImprovementRunState::Decided.as_str() {
                        return Err("the decision is already recorded".to_owned());
                    }
                    if decision == ImprovementDecision::Accepted && digest.is_none() {
                        return Err("an accepted decision needs the canonical digest".to_owned());
                    }
                    if let Some(baseline) = &baseline {
                        let exists: Option<i64> = transaction
                            .query_row(
                                "SELECT 1 FROM research__baselines
                                  WHERE profile_id = ?1 AND baseline_id = ?2",
                                params![owner, baseline.baseline_id],
                                |row| row.get(0),
                            )
                            .optional()
                            .map_err(|error| error.to_string())?;
                        if exists.is_some() {
                            return Err("the baseline id already exists".to_owned());
                        }
                    }
                    if let Some(invalidation) = &invalidation {
                        let exists: Option<i64> = transaction
                            .query_row(
                                "SELECT 1 FROM research__invalidations
                                  WHERE profile_id = ?1 AND invalidation_id = ?2",
                                params![owner, invalidation.invalidation_id],
                                |row| row.get(0),
                            )
                            .optional()
                            .map_err(|error| error.to_string())?;
                        if exists.is_some() {
                            return Err("the invalidation id already exists".to_owned());
                        }
                    }
                    let changed = transaction
                        .execute(
                            "UPDATE research__improvement_runs
                                SET state = 'decided', decision = ?3, decision_note = ?4,
                                    decided_at = ?5
                              WHERE profile_id = ?1 AND improvement_id = ?2
                                AND state = 'submitted'",
                            params![owner, key, decision.as_str(), note, decided_at],
                        )
                        .map_err(|error| error.to_string())?;
                    if changed != 1 {
                        return Err("the decision was not recorded".to_owned());
                    }
                    if let Some(baseline) = baseline {
                        transaction
                            .execute(
                                "INSERT INTO research__baselines
                                     (profile_id, baseline_id, tier, run_id, improvement_id,
                                      accepted_by, accepted_on, scope, supersedes, created_at)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                                params![
                                    owner,
                                    baseline.baseline_id,
                                    baseline.tier.as_str(),
                                    baseline.run_id,
                                    baseline.improvement_id,
                                    baseline.accepted_by,
                                    baseline.accepted_on,
                                    baseline.scope,
                                    baseline.supersedes,
                                    baseline.created_at
                                ],
                            )
                            .map_err(|error| error.to_string())?;
                    }
                    if let Some(invalidation) = invalidation {
                        transaction
                            .execute(
                                "INSERT INTO research__invalidations
                                     (profile_id, invalidation_id, date, \"commit\", removed,
                                      reason, superseded_by, created_at)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                params![
                                    owner,
                                    invalidation.invalidation_id,
                                    invalidation.date,
                                    invalidation.commit,
                                    invalidation.removed,
                                    invalidation.reason,
                                    invalidation.superseded_by,
                                    invalidation.created_at
                                ],
                            )
                            .map_err(|error| error.to_string())?;
                    }
                    let stored = read_run(&transaction, &owner, &key)
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| "the improvement record is unknown".to_owned())?;
                    transaction.commit().map_err(|error| error.to_string())?;
                    Ok(stored)
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            detail_from_stored(stored)
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
        let database = self.database.clone();
        Box::pin(async move {
            if note.trim().is_empty() {
                return Err(ImprovementStoreError(
                    "a decision needs a non-empty note".to_owned(),
                ));
            }
            let owner = profile.as_str().to_owned();
            let update_key = improvement_id.clone();
            let sql = format!(
                "UPDATE {IMPROVEMENT_RUNS_TABLE}
                    SET state = 'decided', decision = ?3, decision_note = ?4,
                        decided_at = ?5
                  WHERE profile_id = ?1 AND improvement_id = ?2 AND state = 'submitted'
                    AND (?3 <> 'accepted' OR canonical_digest IS NOT NULL)"
            );
            let owner_for_update = owner.clone();
            let changed = database
                .call(move |connection| {
                    connection.execute(
                        &sql,
                        params![
                            owner_for_update,
                            update_key,
                            decision.as_str(),
                            note,
                            decided_at
                        ],
                    )
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let stored = database
                .call(move |connection| read_run(connection, &owner, &improvement_id))
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?
                .ok_or_else(|| {
                    ImprovementStoreError("the improvement record is unknown".to_owned())
                })?;
            let detail = detail_from_stored(stored)?;
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
        let database = self.database.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).map_err(|_| {
                ImprovementStoreError("the list limit does not fit an integer".to_owned())
            })?;
            let owner = profile.as_str().to_owned();
            let sql = format!(
                "SELECT improvement_id, manifest, pipeline_run_id, canonical_digest,
                        state, decision, decision_note, created_at, submitted_at, decided_at,
                        archived_at
                   FROM {IMPROVEMENT_RUNS_TABLE}
                  WHERE profile_id = ?1 AND archived_at IS NULL
                  ORDER BY created_at DESC, improvement_id
                  LIMIT ?2"
            );
            let rows = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&sql)?;
                    let rows = statement.query_map(params![owner, limit], |row| {
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
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter().map(summary_from_stored).collect()
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
        limit: usize,
    ) -> BoxFuture<Result<Vec<ImprovementRunSummary>, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).map_err(|_| {
                ImprovementStoreError("the list limit does not fit an integer".to_owned())
            })?;
            let owner = profile.as_str().to_owned();
            let sql = format!(
                "SELECT improvement_id, manifest, pipeline_run_id, canonical_digest,
                        state, decision, decision_note, created_at, submitted_at, decided_at,
                        archived_at
                   FROM {IMPROVEMENT_RUNS_TABLE}
                  WHERE profile_id = ?1
                  ORDER BY created_at DESC, improvement_id
                  LIMIT ?2"
            );
            let rows = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&sql)?;
                    let rows = statement.query_map(params![owner, limit], |row| {
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
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter().map(summary_from_stored).collect()
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        improvement_id: String,
    ) -> BoxFuture<Result<Option<ImprovementRunDetail>, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let owner = profile.as_str().to_owned();
            let stored = database
                .call(move |connection| read_run(connection, &owner, &improvement_id))
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            stored.map(detail_from_stored).transpose()
        })
    }

    fn archive(
        &self,
        profile: ProfileId,
        improvement_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<ResearchArchiveOutcome, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let owner = profile.as_str().to_owned();
            let read_owner = owner.clone();
            let read_id = improvement_id.clone();
            let archived_for_read = archived_at.clone();
            let outcome = database
                .clone()
                .call(
                    move |connection| -> rusqlite::Result<ResearchArchiveOutcome> {
                        let transaction =
                            connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                        let updated = transaction.execute(
                            &format!(
                                "UPDATE {IMPROVEMENT_RUNS_TABLE} SET archived_at = ?3
                                  WHERE profile_id = ?1 AND improvement_id = ?2
                                    AND archived_at IS NULL"
                            ),
                            params![owner, improvement_id, archived_at],
                        )?;
                        if updated == 0 {
                            let exists: Option<i64> = transaction
                                .query_row(
                                    &format!(
                                        "SELECT 1 FROM {IMPROVEMENT_RUNS_TABLE}
                                      WHERE profile_id = ?1 AND improvement_id = ?2"
                                    ),
                                    params![owner, improvement_id],
                                    |row| row.get(0),
                                )
                                .optional()?;
                            return Ok(if exists.is_some() {
                                ResearchArchiveOutcome::AlreadyArchived
                            } else {
                                ResearchArchiveOutcome::Missing
                            });
                        }
                        transaction.execute(
                            &format!(
                                "INSERT INTO {RESEARCH_RETENTION_TABLE}
                                 (profile_id, kind, record_id, action, reason, bytes)
                             VALUES (?1, 'research', ?2, 'archive', 'operator.archive', 0)"
                            ),
                            params![owner, improvement_id],
                        )?;
                        transaction.commit()?;
                        Ok(ResearchArchiveOutcome::Archived(ImprovementRunSummary {
                            improvement_id: String::new(),
                            tier: ImprovementTier::Model,
                            state: ImprovementRunState::Created,
                            decision: None,
                            created_at: String::new(),
                            submitted_at: None,
                            decided_at: None,
                            archived_at: Some(archived_for_read),
                        }))
                    },
                )
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let ResearchArchiveOutcome::Archived(_) = outcome else {
                return Ok(outcome);
            };
            let stored = database
                .call(move |connection| read_run(connection, &read_owner, &read_id))
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?
                .ok_or_else(|| {
                    ImprovementStoreError("the research record vanished after archive".to_owned())
                })?;
            Ok(ResearchArchiveOutcome::Archived(summary_from_stored(
                stored,
            )?))
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        improvement_id: String,
    ) -> BoxFuture<Result<ResearchEraseOutcome, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(
                    move |connection| -> Result<ResearchEraseOutcome, ImprovementStoreError> {
                        let transaction = connection
                            .transaction_with_behavior(TransactionBehavior::Immediate)
                            .map_err(|error| ImprovementStoreError(error.to_string()))?;
                        let stored = read_run(&transaction, profile.as_str(), &improvement_id)
                            .map_err(|error| ImprovementStoreError(error.to_string()))?;
                        let Some(stored) = stored else {
                            return Ok(ResearchEraseOutcome::Missing);
                        };
                        if stored.archived_at.is_none() {
                            return Ok(ResearchEraseOutcome::NotArchived);
                        }
                        let detail = detail_from_stored(stored)?;
                        if detail.state == ImprovementRunState::Decided {
                            return Ok(ResearchEraseOutcome::Protected);
                        }
                        transaction
                            .execute(
                                &format!(
                                    "DELETE FROM {IMPROVEMENT_RUNS_TABLE}
                                  WHERE profile_id = ?1 AND improvement_id = ?2"
                                ),
                                params![profile.as_str(), improvement_id],
                            )
                            .map_err(|error| ImprovementStoreError(error.to_string()))?;
                        transaction
                            .execute(
                                &format!(
                                    "INSERT INTO {RESEARCH_RETENTION_TABLE}
                                     (profile_id, kind, record_id, action, reason, bytes)
                                 VALUES (?1, 'research', ?2, 'erase', 'operator.erase', 0)"
                                ),
                                params![profile.as_str(), improvement_id],
                            )
                            .map_err(|error| ImprovementStoreError(error.to_string()))?;
                        transaction
                            .commit()
                            .map_err(|error| ImprovementStoreError(error.to_string()))?;
                        Ok(ResearchEraseOutcome::Erased(ResearchEraseReceipt {
                            improvement_id,
                            data_version: detail.manifest.data_version,
                        }))
                    },
                )
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))
        })
    }

    fn submitted_datasets(
        &self,
    ) -> BoxFuture<Result<Vec<ResearchDatasetReference>, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let rows: Vec<(String, String, String)> = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&format!(
                        "SELECT profile_id, improvement_id, manifest
                           FROM {IMPROVEMENT_RUNS_TABLE}
                          ORDER BY profile_id, improvement_id"
                    ))?;
                    let mapped = statement.query_map([], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    })?;
                    mapped.collect::<rusqlite::Result<Vec<_>>>()
                })
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
        let database = self.database.clone();
        Box::pin(async move {
            validate_baseline(&record)?;
            let owner = profile.as_str().to_owned();
            let row = record.clone();
            let sql = format!(
                "INSERT INTO {BASELINES_TABLE}
                     (profile_id, baseline_id, tier, run_id, improvement_id,
                      accepted_by, accepted_on, scope, supersedes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (profile_id, baseline_id) DO NOTHING"
            );
            let changed = database
                .call(move |connection| {
                    connection.execute(
                        &sql,
                        params![
                            owner,
                            row.baseline_id,
                            row.tier.as_str(),
                            row.run_id,
                            row.improvement_id,
                            row.accepted_by,
                            row.accepted_on,
                            row.scope,
                            row.supersedes,
                            row.created_at
                        ],
                    )
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            if changed == 0 {
                return Err(ImprovementStoreError(
                    "the baseline id already exists".to_owned(),
                ));
            }
            Ok(record)
        })
    }

    fn add_invalidation(
        &self,
        profile: ProfileId,
        record: InvalidationRecord,
    ) -> BoxFuture<Result<InvalidationRecord, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            validate_invalidation(&record)?;
            let owner = profile.as_str().to_owned();
            let row = record.clone();
            let sql = format!(
                "INSERT INTO {INVALIDATIONS_TABLE}
                     (profile_id, invalidation_id, date, \"commit\", removed,
                      reason, superseded_by, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT (profile_id, invalidation_id) DO NOTHING"
            );
            let changed = database
                .call(move |connection| {
                    connection.execute(
                        &sql,
                        params![
                            owner,
                            row.invalidation_id,
                            row.date,
                            row.commit,
                            row.removed,
                            row.reason,
                            row.superseded_by,
                            row.created_at
                        ],
                    )
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            if changed == 0 {
                return Err(ImprovementStoreError(
                    "the invalidation id already exists".to_owned(),
                ));
            }
            Ok(record)
        })
    }

    fn baselines(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<BaselineRecord>, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let owner = profile.as_str().to_owned();
            let sql = format!(
                "SELECT baseline_id, tier, run_id, improvement_id, accepted_by,
                        accepted_on, scope, supersedes, created_at
                   FROM {BASELINES_TABLE}
                  WHERE profile_id = ?1
                  ORDER BY accepted_on DESC, created_at DESC, baseline_id"
            );
            let rows = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&sql)?;
                    let rows = statement.query_map(params![owner], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, String>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, String>(8)?,
                        ))
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter()
                .map(
                    |(
                        baseline_id,
                        tier_name,
                        run_id,
                        improvement_id,
                        accepted_by,
                        accepted_on,
                        scope,
                        supersedes,
                        created_at,
                    )| {
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
                    },
                )
                .collect()
        })
    }

    fn invalidations(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<InvalidationRecord>, ImprovementStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let owner = profile.as_str().to_owned();
            let sql = format!(
                "SELECT invalidation_id, date, \"commit\", removed, reason,
                        superseded_by, created_at
                   FROM {INVALIDATIONS_TABLE}
                  WHERE profile_id = ?1
                  ORDER BY date DESC, invalidation_id"
            );
            let rows = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&sql)?;
                    let rows = statement.query_map(params![owner], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, String>(6)?,
                        ))
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })
                .await
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            rows.into_iter()
                .map(
                    |(
                        invalidation_id,
                        date,
                        commit,
                        removed,
                        reason,
                        superseded_by,
                        created_at,
                    )| {
                        Ok(InvalidationRecord {
                            invalidation_id,
                            date,
                            commit,
                            removed,
                            reason,
                            superseded_by,
                            created_at,
                        })
                    },
                )
                .collect()
        })
    }
}
