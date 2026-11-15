//! Row decoding for the SQLite simulation repository.
//!
//! SQLite returns a row borrowed from a prepared statement, so every value is
//! read by name and parsed here rather than through a derive. A malformed
//! stored value is [`StoreError::Invalid`] and never a panic, and a value the
//! adapter refuses is [`StoreError::Backend`]. The column list is the same
//! logical shape as the Postgres repository: the JSON documents are `TEXT` and
//! the instants are RFC 3339 UTC `TEXT`.

use quantrule_api::{Digest, MarkOrigin, ProfileId};
use quantrule_simulation::{CheckRecord, Selection, WindowSpec};
use rusqlite::{Connection, Row, params};

use crate::records::{
    DecisionKind, DecisionRecord, MarkRecord, RunRecord, action_text, origin_text, parse_action,
    parse_origin, parse_quality, parse_side, parse_status, side_text,
};
use crate::store::StoreError;

use super::{MARKS, RUNS};

/// The run projection, with the three mark counts as correlated subqueries.
pub(super) const RUN_COLUMNS: &str = "
    r.sim_id, r.premise, r.bundle, r.inputs, r.inputs_digest, r.seed, r.criteria,
    r.window_spec, r.oracle_spec, r.check_config, r.selection,
    r.status, r.check_result, r.decision, r.decision_note,
    r.decided_at, r.created_at,
    (SELECT count(*) FROM simulation__marks m
      WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS human_marks,
    (SELECT count(*) FROM simulation__marks m
      WHERE m.sim_id = r.sim_id AND m.origin = 'oracle') AS oracle_marks,
    (SELECT count(DISTINCT m.chart) FROM simulation__marks m
      WHERE m.sim_id = r.sim_id AND m.origin = 'human') AS marked_windows";

/// The mark projection.
pub(super) const MARK_COLUMNS: &str = "
    mark_id, chart, bar, marked_time_utc, action, quality, side, origin, note,
    created_at";

/// A database failure.
pub(super) fn backend(error: rusqlite::Error) -> StoreError {
    StoreError::Backend(error.to_string())
}

/// A stored value that violates the contract.
pub(super) fn invalid(message: impl Into<String>) -> StoreError {
    StoreError::Invalid(message.into())
}

fn text(row: &Row<'_>, name: &str) -> Result<String, StoreError> {
    row.get::<_, String>(name).map_err(backend)
}

fn optional_text(row: &Row<'_>, name: &str) -> Result<Option<String>, StoreError> {
    row.get::<_, Option<String>>(name).map_err(backend)
}

pub(super) fn unsigned(value: i64, field: &str) -> Result<u64, StoreError> {
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

fn digest_of(row: &Row<'_>, name: &str, field: &str) -> Result<Digest, StoreError> {
    let value = text(row, name)?;
    Digest::parse(&value)
        .map_err(|error| invalid(format!("the stored {field} is malformed: {error}")))
}

fn decision_of(row: &Row<'_>) -> Result<Option<DecisionRecord>, StoreError> {
    let Some(word) = optional_text(row, "decision")? else {
        return Ok(None);
    };
    let decision = DecisionKind::parse(&word)
        .ok_or_else(|| invalid(format!("the stored decision {word:?} is unknown")))?;
    let note = optional_text(row, "decision_note")?.unwrap_or_default();
    let decided_at_utc = optional_text(row, "decided_at")?
        .ok_or_else(|| invalid("a decided run has no decision time"))?;
    Ok(Some(DecisionRecord {
        decision,
        note,
        decided_at_utc,
    }))
}

/// Decode one run row.
pub(super) fn run_from_row(row: &Row<'_>) -> Result<RunRecord, StoreError> {
    let status_word = text(row, "status")?;
    let status = parse_status(&status_word)
        .ok_or_else(|| invalid(format!("the stored status {status_word:?} is unknown")))?;
    let window_spec: String = text(row, "window_spec")?;
    let window_spec: WindowSpec = serde_json::from_str(&window_spec)
        .map_err(|error| invalid(format!("the stored window spec is malformed: {error}")))?;
    let oracle_spec: String = text(row, "oracle_spec")?;
    let oracle_spec = serde_json::from_str(&oracle_spec)
        .map_err(|error| invalid(format!("the stored oracle spec is malformed: {error}")))?;
    let check_config: String = text(row, "check_config")?;
    let check_config = serde_json::from_str(&check_config)
        .map_err(|error| invalid(format!("the stored check config is malformed: {error}")))?;
    let selection: Option<Selection> = parse_json(optional_text(row, "selection")?, "selection")?;
    let check_result: Option<CheckRecord> =
        parse_json(optional_text(row, "check_result")?, "check result")?;
    Ok(RunRecord {
        sim_id: text(row, "sim_id")?,
        premise: text(row, "premise")?,
        bundle: digest_of(row, "bundle", "bundle")?,
        inputs: text(row, "inputs")?,
        inputs_digest: digest_of(row, "inputs_digest", "inputs digest")?,
        seed: unsigned(row.get::<_, i64>("seed").map_err(backend)?, "seed")?,
        criteria: text(row, "criteria")?,
        window_spec,
        oracle_spec,
        check_config,
        selection,
        status,
        check_result,
        decision: decision_of(row)?,
        created_at: text(row, "created_at")?,
        human_marks: unsigned(
            row.get::<_, i64>("human_marks").map_err(backend)?,
            "human count",
        )?,
        oracle_marks: unsigned(
            row.get::<_, i64>("oracle_marks").map_err(backend)?,
            "oracle count",
        )?,
        marked_windows: unsigned(
            row.get::<_, i64>("marked_windows").map_err(backend)?,
            "marked-window count",
        )?,
    })
}

/// Decode one mark row.
pub(super) fn mark_from_row(row: &Row<'_>) -> Result<MarkRecord, StoreError> {
    let action_word = text(row, "action")?;
    let quality_word = text(row, "quality")?;
    let side_word = text(row, "side")?;
    let origin_word = text(row, "origin")?;
    Ok(MarkRecord {
        mark_id: text(row, "mark_id")?,
        chart: unsigned(row.get::<_, i64>("chart").map_err(backend)?, "chart")?,
        bar: unsigned(row.get::<_, i64>("bar").map_err(backend)?, "bar")?,
        marked_time_utc: text(row, "marked_time_utc")?,
        action: parse_action(&action_word)
            .ok_or_else(|| invalid(format!("the stored action {action_word:?} is unknown")))?,
        quality: parse_quality(&quality_word)
            .ok_or_else(|| invalid(format!("the stored quality {quality_word:?} is unknown")))?,
        side: parse_side(&side_word)
            .ok_or_else(|| invalid(format!("the stored side {side_word:?} is unknown")))?,
        origin: parse_origin(&origin_word)
            .ok_or_else(|| invalid(format!("the stored origin {origin_word:?} is unknown")))?,
        note: text(row, "note")?,
        created_at: text(row, "created_at")?,
    })
}

/// Read one run in the profile, or `None`.
pub(super) fn load_run(
    connection: &Connection,
    profile: &ProfileId,
    sim_id: &str,
) -> Result<Option<RunRecord>, StoreError> {
    let sql = format!(
        "SELECT {RUN_COLUMNS} FROM {RUNS} r
         WHERE r.profile_id = ?1 AND r.sim_id = ?2"
    );
    let mut statement = connection.prepare(&sql).map_err(backend)?;
    let mut rows = statement
        .query(params![profile.as_str(), sim_id])
        .map_err(backend)?;
    match rows.next().map_err(backend)? {
        Some(row) => run_from_row(row).map(Some),
        None => Ok(None),
    }
}

/// Read the profile's marks for one run, optionally only one origin.
pub(super) fn load_marks(
    connection: &Connection,
    profile: &ProfileId,
    sim_id: &str,
    origin: Option<MarkOrigin>,
) -> Result<Vec<MarkRecord>, StoreError> {
    let order = "ORDER BY chart, bar, action, side, origin";
    let sql = match origin {
        Some(_) => format!(
            "SELECT {MARK_COLUMNS} FROM {MARKS}
             WHERE profile_id = ?1 AND sim_id = ?2 AND origin = ?3
             {order}"
        ),
        None => format!(
            "SELECT {MARK_COLUMNS} FROM {MARKS}
             WHERE profile_id = ?1 AND sim_id = ?2
             {order}"
        ),
    };
    let mut statement = connection.prepare(&sql).map_err(backend)?;
    let mut rows = match origin {
        Some(origin) => statement.query(params![profile.as_str(), sim_id, origin_text(origin)]),
        None => statement.query(params![profile.as_str(), sim_id]),
    }
    .map_err(backend)?;
    let mut marks = Vec::new();
    while let Some(row) = rows.next().map_err(backend)? {
        marks.push(mark_from_row(row)?);
    }
    Ok(marks)
}

/// Read one mark by its identity `(sim_id, origin, chart, bar, action, side)`.
pub(super) fn load_mark(
    connection: &Connection,
    profile: &ProfileId,
    sim_id: &str,
    mark: &MarkRecord,
) -> Result<Option<MarkRecord>, StoreError> {
    let sql = format!(
        "SELECT {MARK_COLUMNS} FROM {MARKS}
         WHERE profile_id = ?1 AND sim_id = ?2 AND origin = ?3 AND chart = ?4
           AND bar = ?5 AND action = ?6 AND side = ?7"
    );
    let chart = i64::try_from(mark.chart).map_err(|_| invalid("the chart does not fit"))?;
    let bar = i64::try_from(mark.bar).map_err(|_| invalid("the bar does not fit"))?;
    let mut statement = connection.prepare(&sql).map_err(backend)?;
    let mut rows = statement
        .query(params![
            profile.as_str(),
            sim_id,
            origin_text(mark.origin),
            chart,
            bar,
            action_text(mark.action),
            side_text(mark.side),
        ])
        .map_err(backend)?;
    match rows.next().map_err(backend)? {
        Some(row) => mark_from_row(row).map(Some),
        None => Ok(None),
    }
}
