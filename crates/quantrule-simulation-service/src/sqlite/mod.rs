//! The SQLite simulation repository.
//!
//! The store implements the existing [`SimulationStore`] trait; no route or use
//! case changes between the server and desktop profiles. Every statement names
//! `simulation__runs` or `simulation__marks`, the double-prefix spelling of the
//! Postgres `simulation.runs` and `simulation.marks` relations, and filters by
//! the injected profile. No statement reads or writes `runs`, `catalog`,
//! `research`, or `gateway`.
//!
//! SQLite has no `FOR UPDATE` and no second connection here, so a mutation that
//! can race a decision opens an immediate transaction instead. `BEGIN
//! IMMEDIATE` takes the write lock before the first read, and the transaction
//! then re-reads the run row; a decision that lands first is visible to the
//! check and the mutation is refused with the same `Locked` error the Postgres
//! row lock produces. The baseline triggers refuse the same transition from a
//! writer that bypasses the repository.
//!
//! Repository work runs on the runtime's blocking pool through
//! [`SqliteDatabase::call`], so the async route shape is unchanged. A JSON
//! document is `TEXT` and an instant is RFC 3339 UTC `TEXT`; the row decoding
//! lives in the private `rows` module.

mod rows;

use quantrule_api::{MarkOrigin, ProfileId, SimulationStatus};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::DatabaseError;
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use quantrule_simulation::{CheckRecord, Selection};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};

use crate::records::{
    DecisionKind, DecisionRecord, MarkRecord, NewSimulation, RunRecord, SimulationSummary,
    action_text, origin_text, parse_status, quality_text, side_text, status_text,
};
use crate::store::{SimulationStore, StoreError};

use rows::{backend, invalid, load_mark, load_marks, load_run, unsigned};

/// The run relation, from the shared `Schema` mapping.
pub(crate) const RUNS: &str = "simulation__runs";

/// The mark relation, from the shared `Schema` mapping.
pub(crate) const MARKS: &str = "simulation__marks";

/// A simulation repository backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteSimulationStore {
    database: SqliteDatabase,
}

impl SqliteSimulationStore {
    /// Build the repository over an open SQLite database.
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

/// Either a SQLite failure or a store outcome inside one transaction.
///
/// The runtime's blocking call carries one error type out, so the transaction
/// body keeps the two apart: a SQLite failure becomes a backend error and a
/// [`StoreError`] keeps its variant (`Locked` and `NotFound` are the public
/// behavior, not a message).
enum TxError {
    Sqlite(rusqlite::Error),
    Store(StoreError),
}

impl From<rusqlite::Error> for TxError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<StoreError> for TxError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

/// Run one immediate transaction and return its result.
///
/// The transaction is committed only when the body succeeds. A [`StoreError`]
/// returned by the body rolls the transaction back and is returned unchanged; a
/// SQLite failure is returned to the caller, which reports it as a backend
/// error. This is the `FOR UPDATE` replacement: the write lock is held before
/// the body reads the run row.
fn immediate<T>(
    connection: &mut Connection,
    body: impl FnOnce(&Transaction<'_>) -> Result<T, TxError>,
) -> rusqlite::Result<Result<T, StoreError>> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    match body(&transaction) {
        Ok(value) => {
            transaction.commit()?;
            Ok(Ok(value))
        }
        Err(TxError::Store(error)) => Ok(Err(error)),
        Err(TxError::Sqlite(error)) => Err(error),
    }
}

/// Map a runtime database failure onto a store failure.
fn database_failure(error: DatabaseError) -> StoreError {
    StoreError::Backend(error.to_string())
}

/// Run one read on the blocking pool.
async fn read<T: Send + 'static>(
    database: SqliteDatabase,
    body: impl FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
) -> Result<T, StoreError> {
    database
        .call(move |connection| -> rusqlite::Result<Result<T, StoreError>> { Ok(body(connection)) })
        .await
        .map_err(database_failure)?
}

/// Run one immediate transaction on the blocking pool.
async fn write<T: Send + 'static>(
    database: SqliteDatabase,
    body: impl FnOnce(&mut Connection) -> rusqlite::Result<Result<T, StoreError>> + Send + 'static,
) -> Result<T, StoreError> {
    database.call(body).await.map_err(database_failure)?
}

/// Read the run status under the transaction's write lock.
///
/// SQLite has no `FOR UPDATE`; the immediate transaction already holds the
/// write lock, so this read is the serialization point the Postgres row lock
/// provides. A missing row is [`StoreError::NotFound`].
fn locked_status(
    transaction: &Transaction<'_>,
    profile: &ProfileId,
    sim_id: &str,
) -> Result<SimulationStatus, StoreError> {
    let sql = format!("SELECT status FROM {RUNS} WHERE profile_id = ?1 AND sim_id = ?2");
    let mut statement = transaction.prepare(&sql).map_err(backend)?;
    let mut rows = statement
        .query(params![profile.as_str(), sim_id])
        .map_err(backend)?;
    let Some(row) = rows.next().map_err(backend)? else {
        return Err(StoreError::NotFound);
    };
    let word: String = row.get("status").map_err(backend)?;
    parse_status(&word).ok_or_else(|| invalid(format!("the stored status {word:?} is unknown")))
}

/// Read the run status and decision under the transaction's write lock.
fn locked_decision(
    transaction: &Transaction<'_>,
    profile: &ProfileId,
    sim_id: &str,
) -> Result<(SimulationStatus, Option<DecisionRecord>), StoreError> {
    let sql = format!(
        "SELECT status, decision, decision_note, decided_at FROM {RUNS}
         WHERE profile_id = ?1 AND sim_id = ?2"
    );
    let mut statement = transaction.prepare(&sql).map_err(backend)?;
    let mut rows = statement
        .query(params![profile.as_str(), sim_id])
        .map_err(backend)?;
    let Some(row) = rows.next().map_err(backend)? else {
        return Err(StoreError::NotFound);
    };
    let word: String = row.get("status").map_err(backend)?;
    let status = parse_status(&word)
        .ok_or_else(|| invalid(format!("the stored status {word:?} is unknown")))?;
    let Some(word) = row.get::<_, Option<String>>("decision").map_err(backend)? else {
        return Ok((status, None));
    };
    let decision = DecisionKind::parse(&word)
        .ok_or_else(|| invalid(format!("the stored decision {word:?} is unknown")))?;
    let note = row
        .get::<_, Option<String>>("decision_note")
        .map_err(backend)?
        .unwrap_or_default();
    let decided_at_utc = row
        .get::<_, Option<String>>("decided_at")
        .map_err(backend)?
        .ok_or_else(|| invalid("a decided run has no decision time"))?;
    Ok((
        status,
        Some(DecisionRecord {
            decision,
            note,
            decided_at_utc,
        }),
    ))
}

impl SimulationStore for SqliteSimulationStore {
    fn create(
        &self,
        profile: ProfileId,
        run: NewSimulation,
    ) -> BoxFuture<Result<RunRecord, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let window_spec = serde_json::to_string(&run.window_spec)
                .map_err(|error| invalid(error.to_string()))?;
            let oracle_spec = serde_json::to_string(&run.oracle_spec)
                .map_err(|error| invalid(error.to_string()))?;
            let check_config = serde_json::to_string(&run.check_config)
                .map_err(|error| invalid(error.to_string()))?;
            let seed = i64::try_from(run.seed).map_err(|_| invalid("the seed does not fit"))?;
            let NewSimulation {
                sim_id,
                premise,
                bundle,
                inputs,
                inputs_digest,
                criteria,
                created_at,
                ..
            } = run;
            write(database, move |connection| {
                let sql = format!(
                    "INSERT INTO {RUNS}
                         (sim_id, profile_id, premise, bundle, inputs, inputs_digest, seed,
                          criteria, window_spec, oracle_spec, check_config, status, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'draft', ?12)
                     ON CONFLICT (sim_id) DO NOTHING"
                );
                connection.execute(
                    &sql,
                    params![
                        &sim_id,
                        profile.as_str(),
                        &premise,
                        bundle.to_string(),
                        &inputs,
                        inputs_digest.to_string(),
                        seed,
                        &criteria,
                        &window_spec,
                        &oracle_spec,
                        &check_config,
                        &created_at,
                    ],
                )?;
                Ok(load_run(connection, &profile, &sim_id).and_then(|found| {
                    found.ok_or_else(|| {
                        StoreError::Conflict(
                            "the simulation id exists in another profile".to_owned(),
                        )
                    })
                }))
            })
            .await
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        status: Option<SimulationStatus>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<SimulationSummary>, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            read(database, move |connection| {
                let limit = i64::try_from(limit).unwrap_or(i64::MAX);
                let sql = match status {
                    Some(_) => format!(
                        "SELECT r.sim_id, r.premise, r.status, r.bundle,
                                COALESCE(json_array_length(r.selection, '$.windows'), 0)
                                    AS windows,
                                (SELECT count(*) FROM {MARKS} m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marks,
                                (SELECT count(DISTINCT m.chart) FROM {MARKS} m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human')
                                    AS marked_windows
                         FROM {RUNS} r
                         WHERE r.profile_id = ?1 AND r.status = ?2
                         ORDER BY r.created_at DESC, r.sim_id
                         LIMIT ?3"
                    ),
                    None => format!(
                        "SELECT r.sim_id, r.premise, r.status, r.bundle,
                                COALESCE(json_array_length(r.selection, '$.windows'), 0)
                                    AS windows,
                                (SELECT count(*) FROM {MARKS} m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marks,
                                (SELECT count(DISTINCT m.chart) FROM {MARKS} m
                                  WHERE m.sim_id = r.sim_id AND m.origin = 'human')
                                    AS marked_windows
                         FROM {RUNS} r
                         WHERE r.profile_id = ?1
                         ORDER BY r.created_at DESC, r.sim_id
                         LIMIT ?2"
                    ),
                };
                let mut statement = connection.prepare(&sql).map_err(backend)?;
                let mut rows = match status {
                    Some(status) => {
                        statement.query(params![profile.as_str(), status_text(status), limit])
                    }
                    None => statement.query(params![profile.as_str(), limit]),
                }
                .map_err(backend)?;
                let mut summaries = Vec::new();
                while let Some(row) = rows.next().map_err(backend)? {
                    summaries.push(summary_from_row(row)?);
                }
                Ok(summaries)
            })
            .await
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        sim_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            read(database, move |connection| {
                load_run(connection, &profile, &sim_id)
            })
            .await
        })
    }

    fn save_windows(
        &self,
        profile: ProfileId,
        sim_id: String,
        selection: Selection,
    ) -> BoxFuture<Result<RunRecord, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let encoded =
                serde_json::to_string(&selection).map_err(|error| invalid(error.to_string()))?;
            write(database, move |connection| {
                immediate(connection, move |transaction| {
                    let status = locked_status(transaction, &profile, &sim_id)?;
                    if status == SimulationStatus::Decided {
                        return Err(StoreError::Locked.into());
                    }
                    let sql = format!(
                        "UPDATE {RUNS}
                         SET selection = ?3,
                             status = CASE WHEN status = 'draft' THEN 'marking' ELSE status END
                         WHERE profile_id = ?1 AND sim_id = ?2"
                    );
                    transaction.execute(&sql, params![profile.as_str(), &sim_id, &encoded])?;
                    let loaded =
                        load_run(transaction, &profile, &sim_id)?.ok_or(StoreError::NotFound)?;
                    Ok(loaded)
                })
            })
            .await
        })
    }

    fn put_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark: MarkRecord,
    ) -> BoxFuture<Result<MarkRecord, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            write(database, move |connection| {
                immediate(connection, move |transaction| {
                    let status = locked_status(transaction, &profile, &sim_id)?;
                    if status == SimulationStatus::Decided {
                        return Err(StoreError::Locked.into());
                    }
                    let chart =
                        i64::try_from(mark.chart).map_err(|_| invalid("the chart does not fit"))?;
                    let bar =
                        i64::try_from(mark.bar).map_err(|_| invalid("the bar does not fit"))?;
                    let sql = format!(
                        "INSERT INTO {MARKS}
                             (mark_id, sim_id, profile_id, chart, bar, marked_time_utc, action,
                              quality, side, origin, note, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                         ON CONFLICT (sim_id, origin, chart, bar, action, side) DO UPDATE SET
                             marked_time_utc = excluded.marked_time_utc,
                             quality = excluded.quality,
                             note = excluded.note,
                             created_at = excluded.created_at"
                    );
                    transaction.execute(
                        &sql,
                        params![
                            &mark.mark_id,
                            &sim_id,
                            profile.as_str(),
                            chart,
                            bar,
                            &mark.marked_time_utc,
                            action_text(mark.action),
                            quality_text(mark.quality),
                            side_text(mark.side),
                            origin_text(mark.origin),
                            &mark.note,
                            &mark.created_at,
                        ],
                    )?;
                    if status == SimulationStatus::Draft {
                        let update = format!(
                            "UPDATE {RUNS} SET status = 'marking'
                             WHERE profile_id = ?1 AND sim_id = ?2"
                        );
                        transaction.execute(&update, params![profile.as_str(), &sim_id])?;
                    }
                    let stored = load_mark(transaction, &profile, &sim_id, &mark)?.unwrap_or(mark);
                    Ok(stored)
                })
            })
            .await
        })
    }

    fn delete_mark(
        &self,
        profile: ProfileId,
        sim_id: String,
        mark_id: String,
    ) -> BoxFuture<Result<bool, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            write(database, move |connection| {
                immediate(connection, move |transaction| {
                    let status = locked_status(transaction, &profile, &sim_id)?;
                    if status == SimulationStatus::Decided {
                        return Err(StoreError::Locked.into());
                    }
                    let sql = format!(
                        "DELETE FROM {MARKS}
                         WHERE profile_id = ?1 AND sim_id = ?2 AND mark_id = ?3"
                    );
                    let deleted =
                        transaction.execute(&sql, params![profile.as_str(), &sim_id, &mark_id])?;
                    Ok(deleted == 1)
                })
            })
            .await
        })
    }

    fn list_marks(
        &self,
        profile: ProfileId,
        sim_id: String,
        origin: Option<MarkOrigin>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            read(database, move |connection| {
                let Some(run) = load_run(connection, &profile, &sim_id)? else {
                    return Err(StoreError::NotFound);
                };
                load_marks(connection, &profile, &run.sim_id, origin)
            })
            .await
        })
    }

    fn replace_oracle(
        &self,
        profile: ProfileId,
        sim_id: String,
        marks: Vec<MarkRecord>,
    ) -> BoxFuture<Result<Vec<MarkRecord>, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            write(database, move |connection| {
                immediate(connection, move |transaction| {
                    let status = locked_status(transaction, &profile, &sim_id)?;
                    if status == SimulationStatus::Decided {
                        return Err(StoreError::Locked.into());
                    }
                    let delete = format!(
                        "DELETE FROM {MARKS}
                         WHERE profile_id = ?1 AND sim_id = ?2 AND origin = 'oracle'"
                    );
                    transaction.execute(&delete, params![profile.as_str(), &sim_id])?;
                    for mark in &marks {
                        let chart = i64::try_from(mark.chart)
                            .map_err(|_| invalid("the chart does not fit"))?;
                        let bar =
                            i64::try_from(mark.bar).map_err(|_| invalid("the bar does not fit"))?;
                        let sql = format!(
                            "INSERT INTO {MARKS}
                                 (mark_id, sim_id, profile_id, chart, bar, marked_time_utc,
                                  action, quality, side, origin, note, created_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'oracle', ?10, ?11)"
                        );
                        transaction.execute(
                            &sql,
                            params![
                                &mark.mark_id,
                                &sim_id,
                                profile.as_str(),
                                chart,
                                bar,
                                &mark.marked_time_utc,
                                action_text(mark.action),
                                quality_text(mark.quality),
                                side_text(mark.side),
                                &mark.note,
                                &mark.created_at,
                            ],
                        )?;
                    }
                    let update = format!(
                        "UPDATE {RUNS}
                         SET status = CASE WHEN status = 'checked' THEN status ELSE 'reviewed' END
                         WHERE profile_id = ?1 AND sim_id = ?2"
                    );
                    transaction.execute(&update, params![profile.as_str(), &sim_id])?;
                    Ok(load_marks(
                        transaction,
                        &profile,
                        &sim_id,
                        Some(MarkOrigin::Oracle),
                    )?)
                })
            })
            .await
        })
    }

    fn record_check(
        &self,
        profile: ProfileId,
        sim_id: String,
        check: CheckRecord,
    ) -> BoxFuture<Result<(), StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let encoded =
                serde_json::to_string(&check).map_err(|error| invalid(error.to_string()))?;
            write(database, move |connection| {
                immediate(connection, move |transaction| {
                    let _ = locked_status(transaction, &profile, &sim_id)?;
                    let sql = format!(
                        "UPDATE {RUNS}
                         SET check_result = ?3,
                             status = CASE WHEN status = 'decided' THEN status ELSE 'checked' END
                         WHERE profile_id = ?1 AND sim_id = ?2"
                    );
                    transaction.execute(&sql, params![profile.as_str(), &sim_id, &encoded])?;
                    Ok(())
                })
            })
            .await
        })
    }

    fn decide(
        &self,
        profile: ProfileId,
        sim_id: String,
        decision: DecisionRecord,
    ) -> BoxFuture<Result<DecisionRecord, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            write(database, move |connection| {
                immediate(connection, move |transaction| {
                    let (status, existing) = locked_decision(transaction, &profile, &sim_id)?;
                    if let Some(existing) = existing {
                        if existing.decision == decision.decision && existing.note == decision.note
                        {
                            return Ok(existing);
                        }
                        return Err(StoreError::Locked.into());
                    }
                    if status == SimulationStatus::Decided {
                        return Err(StoreError::Locked.into());
                    }
                    let sql = format!(
                        "UPDATE {RUNS}
                         SET status = 'decided', decision = ?3, decision_note = ?4,
                             decided_at = ?5
                         WHERE profile_id = ?1 AND sim_id = ?2"
                    );
                    transaction.execute(
                        &sql,
                        params![
                            profile.as_str(),
                            &sim_id,
                            decision.decision.as_str(),
                            &decision.note,
                            &decision.decided_at_utc,
                        ],
                    )?;
                    Ok(decision)
                })
            })
            .await
        })
    }
}

/// Decode one list row.
fn summary_from_row(row: &rusqlite::Row<'_>) -> Result<SimulationSummary, StoreError> {
    let status_word: String = row.get("status").map_err(backend)?;
    let status = parse_status(&status_word)
        .ok_or_else(|| invalid(format!("the stored status {status_word:?} is unknown")))?;
    let bundle: String = row.get("bundle").map_err(backend)?;
    let windows: i64 = row.get("windows").map_err(backend)?;
    Ok(SimulationSummary {
        sim_id: row.get("sim_id").map_err(backend)?,
        premise: row.get("premise").map_err(backend)?,
        status,
        windows: u32::try_from(windows)
            .map_err(|_| invalid("the stored window count does not fit"))?,
        marks: unsigned(row.get("marks").map_err(backend)?, "mark count")?,
        bundle: quantrule_api::Digest::parse(&bundle)
            .map_err(|error| invalid(format!("the stored bundle is malformed: {error}")))?,
        marked_windows: unsigned(
            row.get("marked_windows").map_err(backend)?,
            "marked-window count",
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::{MARKS, RUNS};
    use quantrule_runtime::adapters::{Dialect, Schema};

    #[test]
    fn the_tables_are_the_simulation_prefix() {
        assert_eq!(
            RUNS,
            Dialect::Sqlite
                .table(Schema::Simulation, "runs")
                .expect("the run table name")
        );
        assert_eq!(
            MARKS,
            Dialect::Sqlite
                .table(Schema::Simulation, "marks")
                .expect("the mark table name")
        );
        assert_eq!(
            Dialect::Postgres
                .table(Schema::Simulation, "runs")
                .expect("the Postgres name"),
            "simulation.runs"
        );
    }
}
