//! The Postgres simulation repository.
//!
//! Every statement names `simulation.runs` or `simulation.marks` and filters by
//! the injected profile. No statement reads or writes `runs`, `catalog`,
//! `research`, or `gateway`. Marking mutations lock the run row and check the
//! decision inside the same transaction, so a decision that lands concurrently
//! cannot be bypassed.

use quantrule_api::{MarkOrigin, ProfileId, SimulationId, SimulationStatus};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_simulation::{CheckRecord, Selection, WindowSpec};
use sqlx::{PgPool, Row};

use crate::records::{
    DecisionKind, DecisionRecord, MarkRecord, NewSimulation, RunRecord, SimulationSummary,
    action_text, origin_text, parse_action, parse_origin, parse_quality, parse_side, parse_status,
    quality_text, side_text, status_text,
};
use crate::store::{ArchiveOutcome, EraseOutcome, SimulationStore, StoreError};

/// A simulation repository backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgSimulationStore {
    pool: PgPool,
}

impl PgSimulationStore {
    /// Build the repository over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const RUN_COLUMNS: &str = "
    r.sim_id, r.premise, r.bundle, r.inputs, r.inputs_digest, r.seed, r.criteria,
    r.window_spec::text AS window_spec,
    r.oracle_spec::text AS oracle_spec,
    r.check_config::text AS check_config,
    r.selection::text AS selection,
    r.status,
    r.check_result::text AS check_result,
    r.decision, r.decision_note,
    to_char(r.decided_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS decided_at,
    to_char(r.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at,
    (SELECT count(*) FROM simulation.marks m
      WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS human_marks,
    (SELECT count(*) FROM simulation.marks m
      WHERE m.sim_id = r.sim_id AND m.origin = 'oracle') AS oracle_marks,
    (SELECT count(DISTINCT m.chart) FROM simulation.marks m
      WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marked_windows";

fn backend(error: sqlx::Error) -> StoreError {
    StoreError::Backend(error.to_string())
}

fn invalid(message: impl Into<String>) -> StoreError {
    StoreError::Invalid(message.into())
}

fn unsigned(value: i64, field: &str) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| invalid(format!("the stored {field} is negative")))
}

fn parse_json<T: serde::de::DeserializeOwned>(
    value: Option<String>,
    field: &str,
) -> Result<Option<T>, StoreError> {
    value
        .map(|text| {
            serde_json::from_str(&text)
                .map_err(|error| invalid(format!("the stored {field} is malformed: {error}")))
        })
        .transpose()
}

fn decision_of(row: &sqlx::postgres::PgRow) -> Result<Option<DecisionRecord>, StoreError> {
    let Some(word) = row
        .try_get::<Option<String>, _>("decision")
        .map_err(backend)?
    else {
        return Ok(None);
    };
    let decision = DecisionKind::parse(&word)
        .ok_or_else(|| invalid(format!("the stored decision {word:?} is unknown")))?;
    let note = row
        .try_get::<Option<String>, _>("decision_note")
        .map_err(backend)?
        .unwrap_or_default();
    let decided_at_utc = row
        .try_get::<Option<String>, _>("decided_at")
        .map_err(backend)?
        .ok_or_else(|| invalid("a decided run has no decision time"))?;
    Ok(Some(DecisionRecord {
        decision,
        note,
        decided_at_utc,
    }))
}

fn run_from_row(row: &sqlx::postgres::PgRow) -> Result<RunRecord, StoreError> {
    let status_word: String = row.try_get("status").map_err(backend)?;
    let status = parse_status(&status_word)
        .ok_or_else(|| invalid(format!("the stored status {status_word:?} is unknown")))?;
    let window_spec: String = row.try_get("window_spec").map_err(backend)?;
    let window_spec: WindowSpec = serde_json::from_str(&window_spec)
        .map_err(|error| invalid(format!("the stored window spec is malformed: {error}")))?;
    let oracle_spec: String = row.try_get("oracle_spec").map_err(backend)?;
    let oracle_spec = serde_json::from_str(&oracle_spec)
        .map_err(|error| invalid(format!("the stored oracle spec is malformed: {error}")))?;
    let check_config: String = row.try_get("check_config").map_err(backend)?;
    let check_config = serde_json::from_str(&check_config)
        .map_err(|error| invalid(format!("the stored check config is malformed: {error}")))?;
    let selection: Option<String> = row.try_get("selection").map_err(backend)?;
    let selection: Option<Selection> = parse_json(selection, "selection")?;
    let check_result: Option<String> = row.try_get("check_result").map_err(backend)?;
    let check_result: Option<CheckRecord> = parse_json(check_result, "check result")?;
    Ok(RunRecord {
        sim_id: row.try_get("sim_id").map_err(backend)?,
        premise: row.try_get("premise").map_err(backend)?,
        bundle: quantrule_api::Digest::parse(&row.try_get::<String, _>("bundle").map_err(backend)?)
            .map_err(|error| invalid(format!("the stored bundle is malformed: {error}")))?,
        inputs: row.try_get("inputs").map_err(backend)?,
        inputs_digest: quantrule_api::Digest::parse(
            &row.try_get::<String, _>("inputs_digest").map_err(backend)?,
        )
        .map_err(|error| invalid(format!("the stored inputs digest is malformed: {error}")))?,
        seed: unsigned(row.try_get("seed").map_err(backend)?, "seed")?,
        criteria: row.try_get("criteria").map_err(backend)?,
        window_spec,
        oracle_spec,
        check_config,
        selection,
        status,
        check_result,
        decision: decision_of(row)?,
        created_at: row.try_get("created_at").map_err(backend)?,
        human_marks: unsigned(row.try_get("human_marks").map_err(backend)?, "human count")?,
        oracle_marks: unsigned(
            row.try_get("oracle_marks").map_err(backend)?,
            "oracle count",
        )?,
        marked_windows: unsigned(
            row.try_get("marked_windows").map_err(backend)?,
            "marked-window count",
        )?,
    })
}

fn mark_from_row(row: &sqlx::postgres::PgRow) -> Result<MarkRecord, StoreError> {
    let action_word: String = row.try_get("action").map_err(backend)?;
    let quality_word: String = row.try_get("quality").map_err(backend)?;
    let side_word: String = row.try_get("side").map_err(backend)?;
    let origin_word: String = row.try_get("origin").map_err(backend)?;
    Ok(MarkRecord {
        mark_id: row.try_get("mark_id").map_err(backend)?,
        chart: unsigned(row.try_get("chart").map_err(backend)?, "chart")?,
        bar: unsigned(row.try_get("bar").map_err(backend)?, "bar")?,
        marked_time_utc: row.try_get("marked_time_utc").map_err(backend)?,
        action: parse_action(&action_word)
            .ok_or_else(|| invalid(format!("the stored action {action_word:?} is unknown")))?,
        quality: parse_quality(&quality_word)
            .ok_or_else(|| invalid(format!("the stored quality {quality_word:?} is unknown")))?,
        side: parse_side(&side_word)
            .ok_or_else(|| invalid(format!("the stored side {side_word:?} is unknown")))?,
        origin: parse_origin(&origin_word)
            .ok_or_else(|| invalid(format!("the stored origin {origin_word:?} is unknown")))?,
        note: row.try_get("note").map_err(backend)?,
        created_at: row.try_get("created_at").map_err(backend)?,
    })
}

async fn load_run(
    pool: &PgPool,
    profile: &ProfileId,
    sim_id: &str,
) -> Result<Option<RunRecord>, StoreError> {
    let sql = format!(
        "SELECT {RUN_COLUMNS} FROM simulation.runs r
         WHERE r.profile_id = $1 AND r.sim_id = $2"
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(profile.as_str())
        .bind(sim_id)
        .fetch_optional(pool)
        .await
        .map_err(backend)?;
    row.as_ref().map(run_from_row).transpose()
}

async fn load_marks(
    pool: &PgPool,
    profile: &ProfileId,
    sim_id: &str,
    origin: Option<MarkOrigin>,
) -> Result<Vec<MarkRecord>, StoreError> {
    let rows = match origin {
        Some(origin) => {
            sqlx::query(
                "SELECT mark_id, chart, bar,
                        to_char(marked_time_utc AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS marked_time_utc,
                        action, quality, side, origin, note,
                        to_char(created_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at
                 FROM simulation.marks
                 WHERE profile_id = $1 AND sim_id = $2 AND origin = $3
                 ORDER BY chart, bar, action, side, origin",
            )
            .bind(profile.as_str())
            .bind(sim_id)
            .bind(origin_text(origin))
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query(
                "SELECT mark_id, chart, bar,
                        to_char(marked_time_utc AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS marked_time_utc,
                        action, quality, side, origin, note,
                        to_char(created_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at
                 FROM simulation.marks
                 WHERE profile_id = $1 AND sim_id = $2
                 ORDER BY chart, bar, action, side, origin",
            )
            .bind(profile.as_str())
            .bind(sim_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(backend)?;
    rows.iter().map(mark_from_row).collect()
}

async fn locked_status(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    profile: &ProfileId,
    sim_id: &str,
) -> Result<SimulationStatus, StoreError> {
    let row = sqlx::query(
        "SELECT status FROM simulation.runs
         WHERE profile_id = $1 AND sim_id = $2 FOR UPDATE",
    )
    .bind(profile.as_str())
    .bind(sim_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(backend)?;
    let Some(row) = row else {
        return Err(StoreError::NotFound);
    };
    let word: String = row.try_get("status").map_err(backend)?;
    parse_status(&word).ok_or_else(|| invalid(format!("the stored status {word:?} is unknown")))
}

impl SimulationStore for PgSimulationStore {
    fn create(
        &self,
        profile: ProfileId,
        run: NewSimulation,
    ) -> BoxFuture<Result<RunRecord, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let window_spec = serde_json::to_string(&run.window_spec)
                .map_err(|error| invalid(error.to_string()))?;
            let oracle_spec = serde_json::to_string(&run.oracle_spec)
                .map_err(|error| invalid(error.to_string()))?;
            let check_config = serde_json::to_string(&run.check_config)
                .map_err(|error| invalid(error.to_string()))?;
            sqlx::query(
                "INSERT INTO simulation.runs
                     (sim_id, profile_id, premise, bundle, inputs, inputs_digest, seed,
                      criteria, window_spec, oracle_spec, check_config, status, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10::jsonb,
                         $11::jsonb, 'draft', $12::timestamptz)
                 ON CONFLICT (sim_id) DO NOTHING",
            )
            .bind(&run.sim_id)
            .bind(profile.as_str())
            .bind(&run.premise)
            .bind(run.bundle.to_string())
            .bind(&run.inputs)
            .bind(run.inputs_digest.to_string())
            .bind(i64::try_from(run.seed).map_err(|_| invalid("the seed does not fit"))?)
            .bind(&run.criteria)
            .bind(window_spec)
            .bind(oracle_spec)
            .bind(check_config)
            .bind(&run.created_at)
            .execute(&pool)
            .await
            .map_err(backend)?;
            load_run(&pool, &profile, &run.sim_id)
                .await?
                .ok_or_else(|| {
                    StoreError::Conflict("the simulation id exists in another profile".to_owned())
                })
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        status: Option<SimulationStatus>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<SimulationSummary>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows = match status {
                Some(status) => {
                    sqlx::query(
                        "SELECT r.sim_id, r.premise, r.status, r.bundle, r.archived_at,
                                COALESCE(jsonb_array_length(r.selection -> 'windows'), 0)::bigint AS windows,
                                (SELECT count(*) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marks,
                                (SELECT count(DISTINCT m.chart) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human')
                                    AS marked_windows
                         FROM simulation.runs r
                         WHERE r.profile_id = $1 AND r.status = $2
                         ORDER BY r.created_at DESC, r.sim_id
                         LIMIT $3",
                    )
                    .bind(profile.as_str())
                    .bind(status_text(status))
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
                }
                None => {
                    sqlx::query(
                        "SELECT r.sim_id, r.premise, r.status, r.bundle, r.archived_at,
                                COALESCE(jsonb_array_length(r.selection -> 'windows'), 0)::bigint AS windows,
                                (SELECT count(*) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marks,
                                (SELECT count(DISTINCT m.chart) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human')
                                    AS marked_windows
                         FROM simulation.runs r
                         WHERE r.profile_id = $1
                         ORDER BY r.created_at DESC, r.sim_id
                         LIMIT $2",
                    )
                    .bind(profile.as_str())
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
                }
            }
            .map_err(backend)?;
            rows.iter()
                .map(|row| {
                    let status_word: String = row.try_get("status").map_err(backend)?;
                    let status = parse_status(&status_word).ok_or_else(|| {
                        invalid(format!("the stored status {status_word:?} is unknown"))
                    })?;
                    Ok(SimulationSummary {
                        sim_id: SimulationId(row.try_get("sim_id").map_err(backend)?),
                        premise: row.try_get("premise").map_err(backend)?,
                        status,
                        windows: u32::try_from(row.try_get::<i64, _>("windows").map_err(backend)?)
                            .map_err(|_| invalid("the window count does not fit"))?,
                        marks: unsigned(row.try_get("marks").map_err(backend)?, "mark count")?,
                        bundle: quantrule_api::Digest::parse(
                            &row.try_get::<String, _>("bundle").map_err(backend)?,
                        )
                        .map_err(|error| {
                            invalid(format!("the stored bundle is malformed: {error}"))
                        })?,
                        marked_windows: unsigned(
                            row.try_get("marked_windows").map_err(backend)?,
                            "marked-window count",
                        )?,
                        archived_at: row.try_get("archived_at").map_err(backend)?,
                    })
                })
                .collect()
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
        status: Option<SimulationStatus>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<SimulationSummary>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows = match status {
                Some(status) => {
                    sqlx::query(
                        "SELECT r.sim_id, r.premise, r.status, r.bundle, r.archived_at,
                                COALESCE(jsonb_array_length(r.selection -> 'windows'), 0)::bigint AS windows,
                                (SELECT count(*) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marks,
                                (SELECT count(DISTINCT m.chart) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human')
                                    AS marked_windows
                         FROM simulation.runs r
                         WHERE r.profile_id = $1 AND r.status = $2
                         ORDER BY r.created_at DESC, r.sim_id
                         LIMIT $3",
                    )
                    .bind(profile.as_str())
                    .bind(status_text(status))
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
                }
                None => {
                    sqlx::query(
                        "SELECT r.sim_id, r.premise, r.status, r.bundle, r.archived_at,
                                COALESCE(jsonb_array_length(r.selection -> 'windows'), 0)::bigint AS windows,
                                (SELECT count(*) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marks,
                                (SELECT count(DISTINCT m.chart) FROM simulation.marks m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human')
                                    AS marked_windows
                         FROM simulation.runs r
                         WHERE r.profile_id = $1
                         ORDER BY r.created_at DESC, r.sim_id
                         LIMIT $2",
                    )
                    .bind(profile.as_str())
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
                }
            }
            .map_err(backend)?;
            rows.iter()
                .map(|row| {
                    let status_word: String = row.try_get("status").map_err(backend)?;
                    let status = parse_status(&status_word).ok_or_else(|| {
                        invalid(format!("the stored status {status_word:?} is unknown"))
                    })?;
                    Ok(SimulationSummary {
                        sim_id: SimulationId(row.try_get("sim_id").map_err(backend)?),
                        premise: row.try_get("premise").map_err(backend)?,
                        status,
                        windows: u32::try_from(row.try_get::<i64, _>("windows").map_err(backend)?)
                            .map_err(|_| invalid("the window count does not fit"))?,
                        marks: unsigned(row.try_get("marks").map_err(backend)?, "mark count")?,
                        bundle: quantrule_api::Digest::parse(
                            &row.try_get::<String, _>("bundle").map_err(backend)?,
                        )
                        .map_err(|error| {
                            invalid(format!("the stored bundle is malformed: {error}"))
                        })?,
                        marked_windows: unsigned(
                            row.try_get("marked_windows").map_err(backend)?,
                            "marked-window count",
                        )?,
                        archived_at: row.try_get("archived_at").map_err(backend)?,
                    })
                })
                .collect()
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        sim_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move { load_run(&pool, &profile, &sim_id).await })
    }

    fn archive(
        &self,
        profile: ProfileId,
        sim_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<ArchiveOutcome, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool.begin().await.map_err(backend)?;
            let updated = sqlx::query(
                "UPDATE simulation.runs
                    SET archived_at = $3::timestamptz
                  WHERE profile_id = $1 AND sim_id = $2 AND archived_at IS NULL",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .bind(&archived_at)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            if updated.rows_affected() == 0 {
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT sim_id FROM simulation.runs
                      WHERE profile_id = $1 AND sim_id = $2",
                )
                .bind(profile.as_str())
                .bind(&sim_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(backend)?;
                transaction.rollback().await.map_err(backend)?;
                return Ok(if exists.is_some() {
                    ArchiveOutcome::AlreadyArchived
                } else {
                    ArchiveOutcome::NotFound
                });
            }
            sqlx::query(
                "INSERT INTO simulation.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'simulation', $2, 'archive', 'operator.archive', 0)",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            transaction.commit().await.map_err(backend)?;
            let Some(record) = load_run(&pool, &profile, &sim_id).await? else {
                return Err(StoreError::NotFound);
            };
            let mut summary = record.summary();
            summary.archived_at = Some(archived_at);
            Ok(ArchiveOutcome::Archived(summary))
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        sim_id: String,
    ) -> BoxFuture<Result<EraseOutcome, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool.begin().await.map_err(backend)?;
            let row: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT status, to_char(archived_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                   FROM simulation.runs
                  WHERE profile_id = $1 AND sim_id = $2",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(backend)?;
            let Some((status_word, archived_at)) = row else {
                transaction.rollback().await.map_err(backend)?;
                return Ok(EraseOutcome::NotFound);
            };
            let status = parse_status(&status_word)
                .ok_or_else(|| invalid(format!("the stored status {status_word:?} is unknown")))?;
            if archived_at.is_none() {
                transaction.rollback().await.map_err(backend)?;
                return Ok(EraseOutcome::NotArchived);
            }
            if matches!(
                status,
                SimulationStatus::Checked | SimulationStatus::Decided
            ) {
                transaction.rollback().await.map_err(backend)?;
                return Ok(EraseOutcome::Protected(status));
            }
            sqlx::query("DELETE FROM simulation.runs WHERE profile_id = $1 AND sim_id = $2")
                .bind(profile.as_str())
                .bind(&sim_id)
                .execute(&mut *transaction)
                .await
                .map_err(backend)?;
            sqlx::query(
                "INSERT INTO simulation.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'simulation', $2, 'erase', 'operator.erase', 0)",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            transaction.commit().await.map_err(backend)?;
            Ok(EraseOutcome::Erased(sim_id))
        })
    }

    fn save_windows(
        &self,
        profile: ProfileId,
        sim_id: String,
        selection: Selection,
    ) -> BoxFuture<Result<RunRecord, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let encoded =
                serde_json::to_string(&selection).map_err(|error| invalid(error.to_string()))?;
            let mut transaction = pool.begin().await.map_err(backend)?;
            let status = locked_status(&mut transaction, &profile, &sim_id).await?;
            if status == SimulationStatus::Decided {
                return Err(StoreError::Locked);
            }
            sqlx::query(
                "UPDATE simulation.runs
                 SET selection = $3::jsonb,
                     status = CASE WHEN status = 'draft' THEN 'marking' ELSE status END
                 WHERE profile_id = $1 AND sim_id = $2",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .bind(encoded)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            transaction.commit().await.map_err(backend)?;
            load_run(&pool, &profile, &sim_id)
                .await?
                .ok_or(StoreError::NotFound)
        })
    }

    fn put_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark: MarkRecord,
    ) -> BoxFuture<Result<MarkRecord, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool.begin().await.map_err(backend)?;
            let status = locked_status(&mut transaction, &profile, &sim_id).await?;
            if status == SimulationStatus::Decided {
                return Err(StoreError::Locked);
            }
            sqlx::query(
                "INSERT INTO simulation.marks
                     (mark_id, sim_id, profile_id, chart, bar, marked_time_utc, action,
                      quality, side, origin, note, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6::timestamptz, $7, $8, $9, $10, $11,
                         $12::timestamptz)
                 ON CONFLICT (sim_id, origin, chart, bar, action, side) DO UPDATE SET
                     marked_time_utc = EXCLUDED.marked_time_utc,
                     quality = EXCLUDED.quality,
                     note = EXCLUDED.note,
                     created_at = EXCLUDED.created_at",
            )
            .bind(&mark.mark_id)
            .bind(&sim_id)
            .bind(profile.as_str())
            .bind(i64::try_from(mark.chart).map_err(|_| invalid("the chart does not fit"))?)
            .bind(i64::try_from(mark.bar).map_err(|_| invalid("the bar does not fit"))?)
            .bind(&mark.marked_time_utc)
            .bind(action_text(mark.action))
            .bind(quality_text(mark.quality))
            .bind(side_text(mark.side))
            .bind(origin_text(mark.origin))
            .bind(&mark.note)
            .bind(&mark.created_at)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            if status == SimulationStatus::Draft {
                sqlx::query(
                    "UPDATE simulation.runs SET status = 'marking'
                     WHERE profile_id = $1 AND sim_id = $2",
                )
                .bind(profile.as_str())
                .bind(&sim_id)
                .execute(&mut *transaction)
                .await
                .map_err(backend)?;
            }
            transaction.commit().await.map_err(backend)?;
            let rows = sqlx::query(
                "SELECT mark_id, chart, bar,
                        to_char(marked_time_utc AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS marked_time_utc,
                        action, quality, side, origin, note,
                        to_char(created_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at
                 FROM simulation.marks
                 WHERE profile_id = $1 AND sim_id = $2 AND origin = $3 AND chart = $4
                   AND bar = $5 AND action = $6 AND side = $7",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .bind(origin_text(mark.origin))
            .bind(i64::try_from(mark.chart).map_err(|_| invalid("the chart does not fit"))?)
            .bind(i64::try_from(mark.bar).map_err(|_| invalid("the bar does not fit"))?)
            .bind(action_text(mark.action))
            .bind(side_text(mark.side))
            .fetch_optional(&pool)
            .await
            .map_err(backend)?;
            rows.as_ref()
                .map(mark_from_row)
                .transpose()
                .map(|value| value.unwrap_or(mark))
        })
    }

    fn delete_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark_id: String,
    ) -> BoxFuture<Result<bool, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool.begin().await.map_err(backend)?;
            let status = locked_status(&mut transaction, &profile, &sim_id).await?;
            if status == SimulationStatus::Decided {
                return Err(StoreError::Locked);
            }
            let deleted = sqlx::query(
                "DELETE FROM simulation.marks
                 WHERE profile_id = $1 AND sim_id = $2 AND mark_id = $3",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .bind(&mark_id)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            transaction.commit().await.map_err(backend)?;
            Ok(deleted.rows_affected() == 1)
        })
    }

    fn list_marks(
        &self,
        profile: ProfileId,
        sim_id: String,
        origin: Option<MarkOrigin>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let Some(run) = load_run(&pool, &profile, &sim_id).await? else {
                return Err(StoreError::NotFound);
            };
            load_marks(&pool, &profile, &run.sim_id, origin).await
        })
    }

    fn replace_oracle(
        &self,
        profile: ProfileId,
        sim_id: String,
        marks: Vec<MarkRecord>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool.begin().await.map_err(backend)?;
            let status = locked_status(&mut transaction, &profile, &sim_id).await?;
            if status == SimulationStatus::Decided {
                return Err(StoreError::Locked);
            }
            sqlx::query(
                "DELETE FROM simulation.marks
                 WHERE profile_id = $1 AND sim_id = $2 AND origin = 'oracle'",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            for mark in &marks {
                sqlx::query(
                    "INSERT INTO simulation.marks
                         (mark_id, sim_id, profile_id, chart, bar, marked_time_utc, action,
                          quality, side, origin, note, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6::timestamptz, $7, $8, $9, 'oracle', $10,
                             $11::timestamptz)",
                )
                .bind(&mark.mark_id)
                .bind(&sim_id)
                .bind(profile.as_str())
                .bind(i64::try_from(mark.chart).map_err(|_| invalid("the chart does not fit"))?)
                .bind(i64::try_from(mark.bar).map_err(|_| invalid("the bar does not fit"))?)
                .bind(&mark.marked_time_utc)
                .bind(action_text(mark.action))
                .bind(quality_text(mark.quality))
                .bind(side_text(mark.side))
                .bind(&mark.note)
                .bind(&mark.created_at)
                .execute(&mut *transaction)
                .await
                .map_err(backend)?;
            }
            sqlx::query(
                "UPDATE simulation.runs
                 SET status = CASE WHEN status = 'checked' THEN status ELSE 'reviewed' END
                 WHERE profile_id = $1 AND sim_id = $2",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            transaction.commit().await.map_err(backend)?;
            load_marks(&pool, &profile, &sim_id, Some(MarkOrigin::Oracle)).await
        })
    }

    fn record_check(
        &self,
        profile: ProfileId,
        sim_id: String,
        check: CheckRecord,
    ) -> BoxFuture<Result<(), StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let encoded =
                serde_json::to_string(&check).map_err(|error| invalid(error.to_string()))?;
            let mut transaction = pool.begin().await.map_err(backend)?;
            let _ = locked_status(&mut transaction, &profile, &sim_id).await?;
            sqlx::query(
                "UPDATE simulation.runs
                 SET check_result = $3::jsonb,
                     status = CASE WHEN status = 'decided' THEN status ELSE 'checked' END
                 WHERE profile_id = $1 AND sim_id = $2",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .bind(encoded)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            transaction.commit().await.map_err(backend)?;
            Ok(())
        })
    }

    fn decide(
        &self,
        profile: ProfileId,
        sim_id: String,
        decision: DecisionRecord,
    ) -> BoxFuture<Result<DecisionRecord, StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool.begin().await.map_err(backend)?;
            let status = locked_status(&mut transaction, &profile, &sim_id).await?;
            if let Some(existing) = load_run(&pool, &profile, &sim_id)
                .await?
                .and_then(|run| run.decision)
            {
                transaction.commit().await.map_err(backend)?;
                if existing.decision == decision.decision && existing.note == decision.note {
                    return Ok(existing);
                }
                return Err(StoreError::Locked);
            }
            if status == SimulationStatus::Decided {
                transaction.commit().await.map_err(backend)?;
                return Err(StoreError::Locked);
            }
            sqlx::query(
                "UPDATE simulation.runs
                 SET status = 'decided', decision = $3, decision_note = $4,
                     decided_at = $5::timestamptz
                 WHERE profile_id = $1 AND sim_id = $2",
            )
            .bind(profile.as_str())
            .bind(&sim_id)
            .bind(decision.decision.as_str())
            .bind(&decision.note)
            .bind(&decision.decided_at_utc)
            .execute(&mut *transaction)
            .await
            .map_err(backend)?;
            transaction.commit().await.map_err(backend)?;
            Ok(decision)
        })
    }
}
