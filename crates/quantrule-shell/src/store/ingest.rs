//! The dataset writer.
//!
//! `store ingest` builds the layers in memory and hands them here. This module
//! writes one `dataset` row, one wide `bar` row per Layer 2 row, one `pivot` row
//! per pivot occurrence, and one `level` row per confirmed level in a single
//! transaction. It copies the engine's values; it never computes an indicator,
//! a metric, a resolution, a pivot, or a level.
//!
//! The writer is deliberately narrower than the schema: Layer 3, events, legs,
//! and consensus are not materialized until #71–#73. The dataset row is
//! therefore visible before the store is complete, which is why
//! `layer3_materialized` stays zero.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_analytics::layers::{Layer2Row, LevelRow, M1_SECONDS, PivotRow};
use quantrule_analytics::resolution::{AxisResolution, ordinal_bucket};
use quantrule_model::axes::structure::registry::FIRST_REFERENCE_CODE;
use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params, params_from_iter};

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
    /// One row per pivot occurrence.
    pub(crate) pivots: &'a [PivotRow],
    /// One row per confirmed level.
    pub(crate) levels: &'a [LevelRow],
}

impl<'a> IngestRows<'a> {
    /// Bundles the three record sets for one transaction.
    pub(crate) const fn new(
        bars: &'a [Layer2Row],
        pivots: &'a [PivotRow],
        levels: &'a [LevelRow],
    ) -> Self {
        Self {
            bars,
            pivots,
            levels,
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
    /// Inserts one dataset, its Layer 2 bars, and its pivot and level records in
    /// one transaction.
    ///
    /// A dataset with the same name and identity is a no-op. A dataset with the
    /// same name but a different identity is a conflict; nothing is overwritten.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::InvalidDataset`] when the declared M1 count does
    /// not match the rows, exceeds the cap, or a record has an invalid code,
    /// price, bar range, availability bar, resolution, or duplicate key;
    /// returns [`StoreError::Conflict`] for a different identity; and returns
    /// [`StoreError::Write`] when SQLite rejects a statement and the
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

fn validate(dataset: &DatasetRecord, rows: &IngestRows<'_>) -> Result<(), StoreError> {
    if dataset.name.is_empty() {
        return Err(StoreError::InvalidDataset {
            detail: "the dataset name is empty".to_string(),
        });
    }
    if dataset.data_version.is_empty() {
        return Err(StoreError::InvalidDataset {
            detail: "the data version is empty".to_string(),
        });
    }
    if dataset.m1_bars > MAX_M1_BARS {
        return Err(StoreError::InvalidDataset {
            detail: format!(
                "the M1 bar count {} exceeds the {MAX_M1_BARS} cap",
                dataset.m1_bars
            ),
        });
    }

    let mut bar_counts = BTreeMap::new();
    for row in rows.bars {
        *bar_counts
            .entry(row.layer1.timeframe_secs)
            .or_insert(0_usize) += 1;
        validate_resolutions(row)?;
    }
    let m1_rows = bar_counts.get(&M1_SECONDS).copied().unwrap_or(0);
    if m1_rows != dataset.m1_bars {
        return Err(StoreError::InvalidDataset {
            detail: format!(
                "the dataset declares {} M1 bars but the layer has {m1_rows}",
                dataset.m1_bars
            ),
        });
    }

    validate_pivots(rows.pivots, &bar_counts)?;
    validate_levels(rows.levels, &bar_counts)?;
    Ok(())
}

fn validate_pivots(rows: &[PivotRow], bar_counts: &BTreeMap<u64, usize>) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    for row in rows {
        let bar_count = timeframe_bar_count(bar_counts, row.timeframe_secs, "pivot")?;
        if !(1..=2).contains(&row.scale_code) {
            return Err(invalid(format!(
                "the pivot scale code {} is not 1 or 2",
                row.scale_code
            )));
        }
        if !(1..=2).contains(&row.kind_code) {
            return Err(invalid(format!(
                "the pivot kind code {} is not 1 or 2",
                row.kind_code
            )));
        }
        if !row.price.is_finite() {
            return Err(invalid(format!(
                "the pivot at bar {} has a non-finite price",
                row.occurred_bar_index
            )));
        }
        check_bar_range(
            bar_count,
            row.occurred_bar_index,
            row.available_bar_index,
            "pivot",
        )?;
        if row.available_bar_index < row.occurred_bar_index {
            return Err(invalid(format!(
                "the pivot at bar {} is available at bar {}, before it occurred",
                row.occurred_bar_index, row.available_bar_index
            )));
        }
        if row.confirmed_code > 1 {
            return Err(invalid(format!(
                "the pivot at bar {} has confirmed code {}",
                row.occurred_bar_index, row.confirmed_code
            )));
        }
        if row.confirmed_code == 0 && row.available_bar_index != row.occurred_bar_index {
            return Err(invalid(format!(
                "the candidate at bar {} is available at bar {}",
                row.occurred_bar_index, row.available_bar_index
            )));
        }
        let key = (
            row.timeframe_secs,
            row.scale_code,
            row.kind_code,
            row.occurred_bar_index,
        );
        if !seen.insert(key) {
            return Err(invalid(format!(
                "the pivot at timeframe {} bar {} is duplicated",
                row.timeframe_secs, row.occurred_bar_index
            )));
        }
    }
    Ok(())
}

fn validate_levels(rows: &[LevelRow], bar_counts: &BTreeMap<u64, usize>) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    for row in rows {
        let bar_count = timeframe_bar_count(bar_counts, row.timeframe_secs, "level")?;
        if row.reference_code < FIRST_REFERENCE_CODE {
            return Err(invalid(format!(
                "the level reference code {} is below {FIRST_REFERENCE_CODE}",
                row.reference_code
            )));
        }
        if !(1..=2).contains(&row.scale_code) {
            return Err(invalid(format!(
                "the level scale code {} is not 1 or 2",
                row.scale_code
            )));
        }
        if !(1..=2).contains(&row.kind_code) {
            return Err(invalid(format!(
                "the level kind code {} is not 1 or 2",
                row.kind_code
            )));
        }
        if !row.price.is_finite() {
            return Err(invalid(format!(
                "the level at bar {} has a non-finite price",
                row.occurred_bar_index
            )));
        }
        check_bar_range(
            bar_count,
            row.occurred_bar_index,
            row.available_bar_index,
            "level",
        )?;
        if row.available_bar_index < row.occurred_bar_index {
            return Err(invalid(format!(
                "the level at bar {} is available at bar {}, before it occurred",
                row.occurred_bar_index, row.available_bar_index
            )));
        }
        let key = (row.timeframe_secs, row.reference_code);
        if !seen.insert(key) {
            return Err(invalid(format!(
                "the level reference code {} is duplicated for timeframe {}",
                row.reference_code, row.timeframe_secs
            )));
        }
    }
    Ok(())
}

fn timeframe_bar_count(
    bar_counts: &BTreeMap<u64, usize>,
    timeframe_secs: u64,
    record: &str,
) -> Result<usize, StoreError> {
    bar_counts.get(&timeframe_secs).copied().ok_or_else(|| {
        invalid(format!(
            "the {record} timeframe {timeframe_secs} has no bars"
        ))
    })
}

fn check_bar_range(
    bar_count: usize,
    occurred_bar_index: usize,
    available_bar_index: usize,
    record: &str,
) -> Result<(), StoreError> {
    if occurred_bar_index >= bar_count {
        return Err(invalid(format!(
            "the {record} occurrence bar {occurred_bar_index} is outside the {bar_count} bars"
        )));
    }
    if available_bar_index >= bar_count {
        return Err(invalid(format!(
            "the {record} availability bar {available_bar_index} is outside the {bar_count} bars"
        )));
    }
    Ok(())
}

fn invalid(detail: impl Into<String>) -> StoreError {
    StoreError::InvalidDataset {
        detail: detail.into(),
    }
}

fn validate_resolutions(row: &Layer2Row) -> Result<(), StoreError> {
    let layer1 = &row.layer1;
    for (axis, resolution) in [
        ("direction", row.resolution.direction),
        ("momentum", row.resolution.momentum),
        ("structure", row.resolution.structure),
        ("progression", row.resolution.progression),
    ] {
        validate_resolution(axis, layer1.timeframe_secs, layer1.bar_index, resolution)?;
    }
    Ok(())
}

fn validate_resolution(
    axis: &str,
    timeframe_secs: u64,
    bar_index: usize,
    resolution: AxisResolution,
) -> Result<(), StoreError> {
    let location = format!("the {axis} resolution at timeframe {timeframe_secs} bar {bar_index}");
    match resolution.continuous {
        Some(value) => {
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err(invalid(format!("{location} has continuous value {value}")));
            }
            if let Some(reason) = resolution.reason {
                return Err(invalid(format!(
                    "{location} carries reason {reason} with a value"
                )));
            }
            let Some(ordinal) = resolution.ordinal else {
                return Err(invalid(format!(
                    "{location} has a continuous value but no ordinal"
                )));
            };
            if ordinal > 9 {
                return Err(invalid(format!(
                    "{location} has ordinal {ordinal}, outside 0..=9"
                )));
            }
            let expected = ordinal_bucket(value);
            if ordinal != expected {
                return Err(invalid(format!(
                    "{location} has ordinal {ordinal} but bucket {expected}"
                )));
            }
        }
        None => {
            if resolution.ordinal.is_some() {
                return Err(invalid(format!(
                    "{location} has an ordinal without a continuous value"
                )));
            }
            let Some(reason) = resolution.reason else {
                return Err(invalid(format!(
                    "{location} has no continuous value and no reason"
                )));
            };
            if !(1..=6).contains(&reason) {
                return Err(invalid(format!(
                    "{location} has reason {reason}, outside 1..=6"
                )));
            }
        }
    }
    Ok(())
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

fn bar_params(dataset_id: i64, row: &Layer2Row) -> [Value; 74] {
    let resolution = &row.resolution;
    let row = &row.layer1;
    let metrics = &row.metrics;
    let readings = &row.readings;
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(row.bar_index as i64),
        Value::from(row.candle.timestamp()),
        Value::from(row.candle.open()),
        Value::from(row.candle.high()),
        Value::from(row.candle.low()),
        Value::from(row.candle.close()),
        Value::from(row.candle.volume()),
        Value::from(readings.ema20),
        Value::from(readings.ema50),
        Value::from(readings.ema200),
        Value::from(readings.rsi14),
        Value::from(readings.atr14),
        Value::from(readings.rvol20),
        Value::from(metrics.direction.direction),
        Value::from(metrics.direction.state_code.map(i64::from)),
        Value::from(metrics.direction.size_percentile),
        Value::from(i64::from(metrics.direction.readiness_code)),
        Value::from(metrics.direction.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.direction.state_age_bars as i64),
        Value::from(metrics.direction.state_changes_50 as i64),
        Value::from(metrics.momentum.force),
        Value::from(metrics.momentum.acceleration),
        Value::from(metrics.momentum.intensification),
        Value::from(metrics.momentum.state_code.map(i64::from)),
        Value::from(metrics.momentum.size_percentile),
        Value::from(i64::from(metrics.momentum.readiness_code)),
        Value::from(metrics.momentum.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.momentum.state_age_bars as i64),
        Value::from(metrics.momentum.state_changes_50 as i64),
        Value::from(metrics.structure.score),
        Value::from(metrics.structure.state_code.map(i64::from)),
        Value::from(i64::from(metrics.structure.scale_code)),
        Value::from(metrics.structure.high_relation_code.map(i64::from)),
        Value::from(metrics.structure.low_relation_code.map(i64::from)),
        Value::from(metrics.structure.ema_distance_atr_20),
        Value::from(metrics.structure.ema_distance_atr_50),
        Value::from(metrics.structure.ema_distance_atr_200),
        Value::from(metrics.structure.ema_cross_code_20.map(i64::from)),
        Value::from(metrics.structure.ema_cross_code_50.map(i64::from)),
        Value::from(metrics.structure.ema_cross_code_200.map(i64::from)),
        Value::from(metrics.structure.ema_retest_code_20.map(i64::from)),
        Value::from(metrics.structure.ema_retest_code_50.map(i64::from)),
        Value::from(metrics.structure.ema_retest_code_200.map(i64::from)),
        Value::from(metrics.structure.size_percentile),
        Value::from(i64::from(metrics.structure.readiness_code)),
        Value::from(metrics.structure.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.structure.state_age_bars as i64),
        Value::from(metrics.structure.state_changes_50 as i64),
        Value::from(metrics.progression.progress),
        Value::from(metrics.progression.stage_code.map(i64::from)),
        Value::from(metrics.progression.leg_direction.map(i64::from)),
        Value::from(metrics.progression.leg_scale_code.map(i64::from)),
        Value::from(metrics.progression.size_percentile),
        Value::from(i64::from(metrics.progression.readiness_code)),
        Value::from(metrics.progression.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.progression.state_age_bars as i64),
        Value::from(metrics.progression.state_changes_50 as i64),
        Value::from(metrics.context.atr_ratio),
        Value::from(metrics.context.rvol_ratio),
        Value::from(metrics.context.context_code.map(i64::from)),
        Value::from(resolution.direction.continuous),
        Value::from(resolution.direction.ordinal.map(i64::from)),
        Value::from(resolution.direction.reason.map(i64::from)),
        Value::from(resolution.momentum.continuous),
        Value::from(resolution.momentum.ordinal.map(i64::from)),
        Value::from(resolution.momentum.reason.map(i64::from)),
        Value::from(resolution.structure.continuous),
        Value::from(resolution.structure.ordinal.map(i64::from)),
        Value::from(resolution.structure.reason.map(i64::from)),
        Value::from(resolution.progression.continuous),
        Value::from(resolution.progression.ordinal.map(i64::from)),
        Value::from(resolution.progression.reason.map(i64::from)),
    ]
}

const BAR_INSERT: &str = "\
INSERT INTO bar (\
    dataset_id, timeframe_secs, bar_index, open_time, open, high, low, close, volume, \
    ema20, ema50, ema200, rsi14, atr14, rvol20, \
    direction, direction_state_code, direction_size_percentile, direction_readiness_code, \
    direction_unavailable_reason_code, direction_state_age_bars, direction_state_changes_50, \
    momentum_force, momentum_acceleration, momentum_intensification, momentum_state_code, \
    momentum_size_percentile, momentum_readiness_code, momentum_unavailable_reason_code, \
    momentum_state_age_bars, momentum_state_changes_50, \
    structure_score, structure_code, structure_scale_code, high_relation_code, low_relation_code, \
    ema_distance_atr_20, ema_distance_atr_50, ema_distance_atr_200, \
    ema_cross_code_20, ema_cross_code_50, ema_cross_code_200, \
    ema_retest_code_20, ema_retest_code_50, ema_retest_code_200, \
    structure_size_percentile, structure_readiness_code, structure_unavailable_reason_code, \
    structure_state_age_bars, structure_state_changes_50, \
    progress, progression_stage_code, progression_leg_direction, progression_leg_scale_code, \
    progression_size_percentile, progression_readiness_code, progression_unavailable_reason_code, \
    progression_state_age_bars, progression_state_changes_50, \
    atr_ratio, rvol_ratio, context_code, \
    direction_continuous, direction_ordinal, direction_resolution_reason_code, \
    momentum_continuous, momentum_ordinal, momentum_resolution_reason_code, \
    structure_continuous, structure_ordinal, structure_resolution_reason_code, \
    progression_continuous, progression_ordinal, progression_resolution_reason_code\
) VALUES (\
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, \
    ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, \
    ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, \
    ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, \
    ?71, ?72, ?73, ?74\
)";

fn pivot_params(dataset_id: i64, row: &PivotRow) -> [Value; 8] {
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(i64::from(row.scale_code)),
        Value::from(i64::from(row.kind_code)),
        Value::from(row.price),
        Value::from(row.occurred_bar_index as i64),
        Value::from(row.available_bar_index as i64),
        Value::from(i64::from(row.confirmed_code)),
    ]
}

const PIVOT_INSERT: &str = "\
INSERT INTO pivot (\
    dataset_id, timeframe_secs, scale_code, kind_code, price, occurred_bar_index, \
    available_bar_index, confirmed_code\
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

fn level_params(dataset_id: i64, row: &LevelRow) -> [Value; 8] {
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(i64::from(row.reference_code)),
        Value::from(i64::from(row.scale_code)),
        Value::from(i64::from(row.kind_code)),
        Value::from(row.price),
        Value::from(row.occurred_bar_index as i64),
        Value::from(row.available_bar_index as i64),
    ]
}

const LEVEL_INSERT: &str = "\
INSERT INTO level (\
    dataset_id, timeframe_secs, reference_code, scale_code, kind_code, price, \
    occurred_bar_index, available_bar_index\
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

#[cfg(test)]
mod tests;
