//! The dataset writer.
//!
//! `store ingest` builds the layers in memory and hands them here. This module
//! writes one `dataset` row, one wide `bar` row per Layer 2 row, one `consensus`
//! row per closed bar per timeframe, one `layer3` row per M1 bar with the 82
//! cross-timeframe aggregates, one `pivot` row per pivot occurrence, one `level`
//! row per confirmed level, one `event` row per event record, and one `leg` row
//! per retained leg in a single transaction. It copies the engine's values; it
//! never computes an indicator, a metric, a resolution, a pivot, a level, an
//! event, a leg, a consensus count, or an aggregate.
//!
//! The writer is deliberately narrower than the schema: the higher-timeframe
//! blocks in `layer3` are not materialized until #73. `layer3_materialized`
//! therefore stays zero.

mod sql;
mod validate;

use std::collections::BTreeSet;

use quantrule_analytics::layers::{
    ConsensusRow, EventRow, Layer2Row, Layer3Row, LegRow, LevelRow, PivotRow,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params, params_from_iter};

use self::sql::{
    BAR_INSERT, CONSENSUS_INSERT, EVENT_INSERT, LAYER3_AGGREGATE_INSERT, LEG_INSERT, LEVEL_INSERT,
    PIVOT_INSERT, bar_params, consensus_params, event_params, layer3_params, leg_params,
    level_params, pivot_params,
};
use self::validate::validate;
use super::{Store, StoreError};

/// The model contract the stored values follow.
pub(crate) const MODEL_CONTRACT: &str = "2026-10-21";

/// The research contract the column names follow.
pub(crate) const RESEARCH_CONTRACT: &str = "2026-10-24";

/// The hard cap on M1 candles in one dataset.
pub(crate) const MAX_M1_BARS: usize = 100_000;

/// The identity and source facts needed to insert a dataset row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasetRecord {
    /// The unique dataset name.
    pub(crate) name: String,
    /// The caller-supplied identity of the input series.
    pub(crate) data_version: String,
    /// The workspace package version.
    pub(crate) model_version: String,
    /// The structure scale: 1 minor, 2 major.
    pub(crate) structure_scale_code: u8,
    /// The accepted M1 source rows.
    pub(crate) m1_bars: usize,
}

/// The records written with one dataset.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IngestRows<'a> {
    /// One row per Layer 2 bar.
    pub(crate) bars: &'a [Layer2Row],
    /// One row per closed bar per timeframe.
    pub(crate) consensus: &'a [ConsensusRow],
    /// One row per M1 bar, with the 82 aggregate columns.
    pub(crate) layer3: &'a [Layer3Row],
    /// One row per pivot occurrence.
    pub(crate) pivots: &'a [PivotRow],
    /// One row per confirmed level.
    pub(crate) levels: &'a [LevelRow],
    /// One row per event record.
    pub(crate) events: &'a [EventRow],
    /// One row per retained progression leg.
    pub(crate) legs: &'a [LegRow],
}

impl<'a> IngestRows<'a> {
    /// Bundles the record sets for one transaction.
    pub(crate) const fn new(
        bars: &'a [Layer2Row],
        consensus: &'a [ConsensusRow],
        layer3: &'a [Layer3Row],
        pivots: &'a [PivotRow],
        levels: &'a [LevelRow],
        events: &'a [EventRow],
        legs: &'a [LegRow],
    ) -> Self {
        Self {
            bars,
            consensus,
            layer3,
            pivots,
            levels,
            events,
            legs,
        }
    }
}

/// The result of an ingest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IngestReport {
    /// The dataset's primary key.
    pub(crate) dataset_id: i64,
    /// The stored `bar` rows.
    pub(crate) bars: usize,
    /// The distinct timeframes in the stored bars.
    pub(crate) timeframes: usize,
    /// Whether an identical dataset already existed.
    pub(crate) unchanged: bool,
}

impl Store {
    /// Inserts one dataset, its Layer 2 bars, its consensus rows, its Layer 3
    /// aggregate rows, and its pivot, level, event, and leg records in one
    /// transaction.
    ///
    /// A dataset with the same name and identity is a no-op. A dataset with the
    /// same name but a different identity is a conflict; nothing is overwritten.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::InvalidDataset`] when the declared M1 count does
    /// not match the rows, exceeds the cap, or a record has an invalid code,
    /// price, bar range, availability bar, resolution, consensus count or
    /// weakest code, missing or duplicate key, invalid aggregate ladder or
    /// list, non-finite aggregate value, invalid event type or subtype, invalid
    /// event reference, missing threshold value on a Signal threshold crossing,
    /// invalid leg scale or lifecycle, or inconsistent leg
    /// availability; returns [`StoreError::Conflict`] for a different identity;
    /// and returns [`StoreError::Write`] when SQLite rejects a statement and the
    /// transaction rolls back.
    pub(crate) fn ingest(
        &mut self,
        dataset: &DatasetRecord,
        rows: &IngestRows<'_>,
    ) -> Result<IngestReport, StoreError> {
        validate(dataset, rows)?;

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| StoreError::Write { source })?;

        if let Some(existing) = read_identity(&transaction, &dataset.name)? {
            if existing.matches(dataset) {
                let bars = count_bars(&transaction, existing.id)?;
                let timeframes = count_timeframes(&transaction, existing.id)?;
                transaction
                    .commit()
                    .map_err(|source| StoreError::Write { source })?;
                return Ok(IngestReport {
                    dataset_id: existing.id,
                    bars,
                    timeframes,
                    unchanged: true,
                });
            }
            return Err(StoreError::Conflict {
                name: dataset.name.clone(),
            });
        }

        transaction
            .execute(
                "INSERT INTO dataset \
                 (name, data_version, model_version, model_contract, research_contract, \
                  structure_scale_code, m1_bars, layer3_materialized) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
                params![
                    dataset.name,
                    dataset.data_version,
                    dataset.model_version,
                    MODEL_CONTRACT,
                    RESEARCH_CONTRACT,
                    i64::from(dataset.structure_scale_code),
                    dataset.m1_bars as i64,
                ],
            )
            .map_err(|source| StoreError::Write { source })?;
        let dataset_id = transaction.last_insert_rowid();

        {
            let mut statement = transaction
                .prepare(BAR_INSERT)
                .map_err(|source| StoreError::Write { source })?;
            for row in rows.bars {
                statement
                    .execute(params_from_iter(bar_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }
        {
            let mut statement = transaction
                .prepare(CONSENSUS_INSERT)
                .map_err(|source| StoreError::Write { source })?;
            for row in rows.consensus {
                statement
                    .execute(params_from_iter(consensus_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }
        {
            let mut statement = transaction
                .prepare(LAYER3_AGGREGATE_INSERT)
                .map_err(|source| StoreError::Write { source })?;
            for row in rows.layer3 {
                statement
                    .execute(params_from_iter(layer3_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }
        {
            let mut statement = transaction
                .prepare(PIVOT_INSERT)
                .map_err(|source| StoreError::Write { source })?;
            for row in rows.pivots {
                statement
                    .execute(params_from_iter(pivot_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }
        {
            let mut statement = transaction
                .prepare(LEVEL_INSERT)
                .map_err(|source| StoreError::Write { source })?;
            for row in rows.levels {
                statement
                    .execute(params_from_iter(level_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }
        {
            let mut statement = transaction
                .prepare(EVENT_INSERT)
                .map_err(|source| StoreError::Write { source })?;
            for row in rows.events {
                statement
                    .execute(params_from_iter(event_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }
        {
            let mut statement = transaction
                .prepare(LEG_INSERT)
                .map_err(|source| StoreError::Write { source })?;
            for row in rows.legs {
                statement
                    .execute(params_from_iter(leg_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }

        transaction
            .commit()
            .map_err(|source| StoreError::Write { source })?;

        Ok(IngestReport {
            dataset_id,
            bars: rows.bars.len(),
            timeframes: distinct_timeframes(rows.bars),
            unchanged: false,
        })
    }
}

fn distinct_timeframes(rows: &[Layer2Row]) -> usize {
    rows.iter()
        .map(|row| row.layer1.timeframe_secs)
        .collect::<BTreeSet<_>>()
        .len()
}

struct ExistingIdentity {
    id: i64,
    data_version: String,
    model_version: String,
    structure_scale_code: i64,
    m1_bars: i64,
}

impl ExistingIdentity {
    fn matches(&self, dataset: &DatasetRecord) -> bool {
        self.data_version == dataset.data_version
            && self.model_version == dataset.model_version
            && self.structure_scale_code == i64::from(dataset.structure_scale_code)
            && self.m1_bars == dataset.m1_bars as i64
    }
}

fn read_identity(
    connection: &Connection,
    name: &str,
) -> Result<Option<ExistingIdentity>, StoreError> {
    connection
        .query_row(
            "SELECT id, data_version, model_version, structure_scale_code, m1_bars \
             FROM dataset WHERE name = ?1",
            [name],
            |row| {
                Ok(ExistingIdentity {
                    id: row.get(0)?,
                    data_version: row.get(1)?,
                    model_version: row.get(2)?,
                    structure_scale_code: row.get(3)?,
                    m1_bars: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|source| StoreError::Query { source })
}

fn count_bars(connection: &Connection, dataset_id: i64) -> Result<usize, StoreError> {
    let count: i64 = connection
        .query_row(
            "SELECT count(*) FROM bar WHERE dataset_id = ?1",
            [dataset_id],
            |row| row.get(0),
        )
        .map_err(|source| StoreError::Query { source })?;
    Ok(count as usize)
}

fn count_timeframes(connection: &Connection, dataset_id: i64) -> Result<usize, StoreError> {
    let count: i64 = connection
        .query_row(
            "SELECT count(DISTINCT timeframe_secs) FROM bar WHERE dataset_id = ?1",
            [dataset_id],
            |row| row.get(0),
        )
        .map_err(|source| StoreError::Query { source })?;
    Ok(count as usize)
}

#[cfg(test)]
mod aggregate_tests;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod record_tests;
#[cfg(test)]
mod tests;
