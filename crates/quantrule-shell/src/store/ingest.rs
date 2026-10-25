//! The per-bar writer.
//!
//! `store ingest` builds the layers in memory and hands them here. This module
//! writes one `dataset` row and one wide `bar` row per Layer 1 row in a single
//! transaction. It copies the engine's values; it never computes an indicator,
//! a metric, or a resolution.
//!
//! The writer is deliberately narrower than the schema: the 12 resolution
//! columns stay null until #70, and Layer 3 is not materialized until #73. The
//! dataset row is therefore visible before the store is complete, which is why
//! `layer3_materialized` stays zero.

use std::collections::BTreeSet;

use quantrule_analytics::layers::{Layer1Row, M1_SECONDS};
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
    /// Inserts one dataset and its Layer 1 bars in one transaction.
    ///
    /// A dataset with the same name and identity is a no-op. A dataset with the
    /// same name but a different identity is a conflict; nothing is overwritten.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::InvalidDataset`] when the declared M1 count does
    /// not match the rows or exceeds the cap, [`StoreError::Conflict`] for a
    /// different identity, and [`StoreError::Write`] when SQLite rejects a
    /// statement and the transaction rolls back.
    pub(crate) fn ingest(
        &mut self,
        dataset: &DatasetRecord,
        rows: &[Layer1Row],
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
            for row in rows {
                statement
                    .execute(params_from_iter(bar_params(dataset_id, row)))
                    .map_err(|source| StoreError::Write { source })?;
            }
        }

        transaction
            .commit()
            .map_err(|source| StoreError::Write { source })?;

        Ok(IngestReport {
            dataset_id,
            bars: rows.len(),
            timeframes: distinct_timeframes(rows),
            unchanged: false,
        })
    }
}

fn validate(dataset: &DatasetRecord, rows: &[Layer1Row]) -> Result<(), StoreError> {
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

    let m1_rows = rows
        .iter()
        .filter(|row| row.timeframe_secs == M1_SECONDS)
        .count();
    if m1_rows != dataset.m1_bars {
        return Err(StoreError::InvalidDataset {
            detail: format!(
                "the dataset declares {} M1 bars but the layer has {m1_rows}",
                dataset.m1_bars
            ),
        });
    }

    Ok(())
}

fn distinct_timeframes(rows: &[Layer1Row]) -> usize {
    rows.iter()
        .map(|row| row.timeframe_secs)
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

fn bar_params(dataset_id: i64, row: &Layer1Row) -> [Value; 62] {
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
    atr_ratio, rvol_ratio, context_code\
) VALUES (\
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, \
    ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, \
    ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, \
    ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62\
)";

#[cfg(test)]
mod tests {
    use super::{BAR_INSERT, DatasetRecord, MAX_M1_BARS};
    use crate::store::{Store, StoreError};
    use quantrule_analytics::layers::{
        ContextMetrics, DirectionMetrics, Layer1Row, Metrics, MomentumMetrics, ProgressionMetrics,
        ReadingsSnapshot,
    };
    use quantrule_data::Candle;
    use rusqlite::types::Value;

    /// The 62 columns the writer binds, in `BAR_INSERT` order.
    const COLUMNS: [&str; 62] = [
        "dataset_id",
        "timeframe_secs",
        "bar_index",
        "open_time",
        "open",
        "high",
        "low",
        "close",
        "volume",
        "ema20",
        "ema50",
        "ema200",
        "rsi14",
        "atr14",
        "rvol20",
        "direction",
        "direction_state_code",
        "direction_size_percentile",
        "direction_readiness_code",
        "direction_unavailable_reason_code",
        "direction_state_age_bars",
        "direction_state_changes_50",
        "momentum_force",
        "momentum_acceleration",
        "momentum_intensification",
        "momentum_state_code",
        "momentum_size_percentile",
        "momentum_readiness_code",
        "momentum_unavailable_reason_code",
        "momentum_state_age_bars",
        "momentum_state_changes_50",
        "structure_score",
        "structure_code",
        "structure_scale_code",
        "high_relation_code",
        "low_relation_code",
        "ema_distance_atr_20",
        "ema_distance_atr_50",
        "ema_distance_atr_200",
        "ema_cross_code_20",
        "ema_cross_code_50",
        "ema_cross_code_200",
        "ema_retest_code_20",
        "ema_retest_code_50",
        "ema_retest_code_200",
        "structure_size_percentile",
        "structure_readiness_code",
        "structure_unavailable_reason_code",
        "structure_state_age_bars",
        "structure_state_changes_50",
        "progress",
        "progression_stage_code",
        "progression_leg_direction",
        "progression_leg_scale_code",
        "progression_size_percentile",
        "progression_readiness_code",
        "progression_unavailable_reason_code",
        "progression_state_age_bars",
        "progression_state_changes_50",
        "atr_ratio",
        "rvol_ratio",
        "context_code",
    ];

    fn candle(timestamp: i64, volume: Option<f64>) -> Candle {
        Candle::new(0, timestamp, 10.0, 11.0, 9.0, 10.5, volume).expect("the test candle is valid")
    }

    fn record(name: &str, data_version: &str, m1_bars: usize) -> DatasetRecord {
        DatasetRecord {
            name: name.to_string(),
            data_version: data_version.to_string(),
            model_version: "0.1.0".to_string(),
            structure_scale_code: 2,
            m1_bars,
        }
    }

    /// Every field carries a distinct value, so a swapped column is visible.
    fn unique_row(timeframe_secs: u64, bar_index: usize) -> Layer1Row {
        Layer1Row {
            timeframe_secs,
            bar_index,
            candle: candle(1_700_000_000, Some(123.0)),
            readings: ReadingsSnapshot {
                ema20: Some(20.1),
                ema50: Some(50.2),
                ema200: Some(200.3),
                rsi14: Some(55.4),
                atr14: Some(1.5),
                rvol20: Some(0.6),
            },
            metrics: Metrics {
                direction: DirectionMetrics {
                    direction: Some(0.11),
                    state_code: Some(1),
                    size_percentile: Some(12.0),
                    readiness_code: 2,
                    unavailable_reason_code: None,
                    state_age_bars: 3,
                    state_changes_50: 4,
                },
                momentum: MomentumMetrics {
                    force: Some(-0.21),
                    acceleration: Some(0.22),
                    intensification: Some(-0.23),
                    state_code: Some(-1),
                    size_percentile: Some(24.0),
                    readiness_code: 1,
                    unavailable_reason_code: Some(7),
                    state_age_bars: 5,
                    state_changes_50: 6,
                },
                structure: StructureFields::values(),
                progression: ProgressionMetrics {
                    progress: Some(0.41),
                    stage_code: Some(3),
                    leg_direction: Some(-1),
                    leg_scale_code: Some(1),
                    size_percentile: Some(42.0),
                    readiness_code: 0,
                    unavailable_reason_code: Some(9),
                    state_age_bars: 9,
                    state_changes_50: 10,
                },
                context: ContextMetrics {
                    atr_ratio: Some(0.51),
                    rvol_ratio: Some(0.52),
                    context_code: Some(4),
                },
            },
        }
    }

    /// A row whose optional values are all null and whose zeros are valid.
    fn sparse_row() -> Layer1Row {
        Layer1Row {
            timeframe_secs: 60,
            bar_index: 0,
            candle: candle(1_700_000_000, Some(0.0)),
            readings: ReadingsSnapshot {
                ema20: None,
                ema50: None,
                ema200: None,
                rsi14: None,
                atr14: None,
                rvol20: None,
            },
            metrics: Metrics {
                direction: DirectionMetrics {
                    direction: None,
                    state_code: None,
                    size_percentile: None,
                    readiness_code: 0,
                    unavailable_reason_code: None,
                    state_age_bars: 0,
                    state_changes_50: 0,
                },
                momentum: MomentumMetrics {
                    force: None,
                    acceleration: None,
                    intensification: None,
                    state_code: None,
                    size_percentile: None,
                    readiness_code: 0,
                    unavailable_reason_code: None,
                    state_age_bars: 0,
                    state_changes_50: 0,
                },
                structure: StructureFields::sparse(),
                progression: ProgressionMetrics {
                    progress: Some(0.0),
                    stage_code: None,
                    leg_direction: None,
                    leg_scale_code: None,
                    size_percentile: None,
                    readiness_code: 0,
                    unavailable_reason_code: None,
                    state_age_bars: 0,
                    state_changes_50: 0,
                },
                context: ContextMetrics {
                    atr_ratio: None,
                    rvol_ratio: None,
                    context_code: None,
                },
            },
        }
    }

    struct StructureFields;

    impl StructureFields {
        fn values() -> quantrule_analytics::layers::StructureMetrics {
            use quantrule_analytics::layers::StructureMetrics;
            StructureMetrics {
                score: Some(0.31),
                state_code: Some(2),
                scale_code: 2,
                high_relation_code: Some(1),
                low_relation_code: Some(-1),
                ema_distance_atr_20: Some(0.32),
                ema_distance_atr_50: Some(0.33),
                ema_distance_atr_200: Some(0.34),
                ema_cross_code_20: Some(1),
                ema_cross_code_50: Some(2),
                ema_cross_code_200: Some(3),
                ema_retest_code_20: Some(4),
                ema_retest_code_50: Some(5),
                ema_retest_code_200: Some(6),
                size_percentile: Some(36.0),
                readiness_code: 2,
                unavailable_reason_code: None,
                state_age_bars: 7,
                state_changes_50: 8,
            }
        }

        fn sparse() -> quantrule_analytics::layers::StructureMetrics {
            use quantrule_analytics::layers::StructureMetrics;
            StructureMetrics {
                score: None,
                state_code: None,
                scale_code: 2,
                high_relation_code: None,
                low_relation_code: None,
                ema_distance_atr_20: None,
                ema_distance_atr_50: None,
                ema_distance_atr_200: None,
                ema_cross_code_20: None,
                ema_cross_code_50: None,
                ema_cross_code_200: None,
                ema_retest_code_20: None,
                ema_retest_code_50: None,
                ema_retest_code_200: None,
                size_percentile: None,
                readiness_code: 0,
                unavailable_reason_code: None,
                state_age_bars: 0,
                state_changes_50: 0,
            }
        }
    }

    fn stored_row(store: &Store, dataset_id: i64) -> Vec<Value> {
        let columns = COLUMNS.join(", ");
        let sql = format!(
            "SELECT {columns} FROM bar WHERE dataset_id = ?1 ORDER BY timeframe_secs, bar_index"
        );
        store
            .connection()
            .query_row(&sql, [dataset_id], |row| {
                (0..COLUMNS.len())
                    .map(|index| row.get(index))
                    .collect::<Result<Vec<Value>, _>>()
            })
            .expect("the stored row is read")
    }

    fn count(store: &Store, table: &str) -> i64 {
        store
            .connection()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("the count is read")
    }

    #[test]
    fn every_written_column_round_trips() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let row = unique_row(60, 7);
        let report = store
            .ingest(&record("demo", "source:1", 1), &[row])
            .expect("the dataset is ingested");
        assert_eq!(report.bars, 1);
        assert_eq!(report.timeframes, 1);

        let expected = vec![
            Value::from(report.dataset_id),
            Value::from(60_i64),
            Value::from(7_i64),
            Value::from(1_700_000_000_i64),
            Value::from(10.0),
            Value::from(11.0),
            Value::from(9.0),
            Value::from(10.5),
            Value::from(Some(123.0)),
            Value::from(Some(20.1)),
            Value::from(Some(50.2)),
            Value::from(Some(200.3)),
            Value::from(Some(55.4)),
            Value::from(Some(1.5)),
            Value::from(Some(0.6)),
            Value::from(Some(0.11)),
            Value::from(Some(1_i64)),
            Value::from(Some(12.0)),
            Value::from(2_i64),
            Value::from(None::<i64>),
            Value::from(3_i64),
            Value::from(4_i64),
            Value::from(Some(-0.21)),
            Value::from(Some(0.22)),
            Value::from(Some(-0.23)),
            Value::from(Some(-1_i64)),
            Value::from(Some(24.0)),
            Value::from(1_i64),
            Value::from(Some(7_i64)),
            Value::from(5_i64),
            Value::from(6_i64),
            Value::from(Some(0.31)),
            Value::from(Some(2_i64)),
            Value::from(2_i64),
            Value::from(Some(1_i64)),
            Value::from(Some(-1_i64)),
            Value::from(Some(0.32)),
            Value::from(Some(0.33)),
            Value::from(Some(0.34)),
            Value::from(Some(1_i64)),
            Value::from(Some(2_i64)),
            Value::from(Some(3_i64)),
            Value::from(Some(4_i64)),
            Value::from(Some(5_i64)),
            Value::from(Some(6_i64)),
            Value::from(Some(36.0)),
            Value::from(2_i64),
            Value::from(None::<i64>),
            Value::from(7_i64),
            Value::from(8_i64),
            Value::from(Some(0.41)),
            Value::from(Some(3_i64)),
            Value::from(Some(-1_i64)),
            Value::from(Some(1_i64)),
            Value::from(Some(42.0)),
            Value::from(0_i64),
            Value::from(Some(9_i64)),
            Value::from(9_i64),
            Value::from(10_i64),
            Value::from(Some(0.51)),
            Value::from(Some(0.52)),
            Value::from(Some(4_i64)),
        ];
        assert_eq!(expected.len(), COLUMNS.len());

        let stored = stored_row(&store, report.dataset_id);
        assert_eq!(stored.len(), COLUMNS.len());
        for ((name, stored), expected) in COLUMNS.iter().zip(&stored).zip(&expected) {
            assert_eq!(stored, expected, "column {name}");
        }
    }

    #[test]
    fn nulls_stay_null_and_zeros_stay_zero() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let report = store
            .ingest(&record("demo", "source:1", 1), &[sparse_row()])
            .expect("the dataset is ingested");

        let stored = stored_row(&store, report.dataset_id);
        let by_name = |name: &str| {
            let index = COLUMNS
                .iter()
                .position(|column| *column == name)
                .expect("the column exists");
            stored[index].clone()
        };

        assert_eq!(by_name("volume"), Value::Real(0.0));
        assert_eq!(by_name("ema20"), Value::Null);
        assert_eq!(by_name("direction"), Value::Null);
        assert_eq!(by_name("direction_readiness_code"), Value::Integer(0));
        assert_eq!(by_name("direction_unavailable_reason_code"), Value::Null);
        assert_eq!(by_name("progress"), Value::Real(0.0));
        assert_eq!(by_name("structure_code"), Value::Null);
        assert_eq!(by_name("context_code"), Value::Null);
        assert_eq!(by_name("structure_scale_code"), Value::Integer(2));

        for column in [
            "direction_continuous",
            "direction_ordinal",
            "direction_resolution_reason_code",
            "momentum_continuous",
            "momentum_ordinal",
            "momentum_resolution_reason_code",
            "structure_continuous",
            "structure_ordinal",
            "structure_resolution_reason_code",
            "progression_continuous",
            "progression_ordinal",
            "progression_resolution_reason_code",
        ] {
            let value: Value = store
                .connection()
                .query_row(
                    &format!("SELECT {column} FROM bar WHERE dataset_id = ?1"),
                    [report.dataset_id],
                    |row| row.get(0),
                )
                .expect("the resolution column is read");
            assert_eq!(value, Value::Null, "column {column}");
        }
    }

    #[test]
    fn the_dataset_row_records_the_identity_and_contracts() {
        let mut store = Store::open_in_memory().expect("the store opens");
        store
            .ingest(&record("demo", "source:1", 1), &[unique_row(60, 0)])
            .expect("the dataset is ingested");

        let (data_version, model_contract, research_contract, scale, m1_bars, materialized) = store
            .connection()
            .query_row(
                "SELECT data_version, model_contract, research_contract, structure_scale_code, \
                 m1_bars, layer3_materialized FROM dataset WHERE name = 'demo'",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .expect("the dataset row is read");
        assert_eq!(data_version, "source:1");
        assert_eq!(model_contract, "2026-10-21");
        assert_eq!(research_contract, "2026-10-24");
        assert_eq!(scale, 2);
        assert_eq!(m1_bars, 1);
        assert_eq!(materialized, 0);
    }

    #[test]
    fn the_same_identity_is_a_no_op() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let dataset = record("demo", "source:1", 1);
        let rows = [unique_row(60, 0)];
        let first = store.ingest(&dataset, &rows).expect("the first ingest");
        assert!(!first.unchanged);

        let second = store.ingest(&dataset, &rows).expect("the second ingest");
        assert!(second.unchanged);
        assert_eq!(second.dataset_id, first.dataset_id);
        assert_eq!(second.bars, first.bars);
        assert_eq!(second.timeframes, first.timeframes);
        assert_eq!(count(&store, "dataset"), 1);
        assert_eq!(count(&store, "bar"), 1);
    }

    #[test]
    fn a_different_identity_is_a_conflict_and_changes_nothing() {
        let mut store = Store::open_in_memory().expect("the store opens");
        store
            .ingest(&record("demo", "source:1", 1), &[unique_row(60, 0)])
            .expect("the first ingest");

        let error = store
            .ingest(&record("demo", "source:2", 1), &[unique_row(60, 0)])
            .expect_err("a different identity is rejected");
        assert!(matches!(error, StoreError::Conflict { name } if name == "demo"));
        assert_eq!(count(&store, "dataset"), 1);
        assert_eq!(count(&store, "bar"), 1);

        let data_version: String = store
            .connection()
            .query_row(
                "SELECT data_version FROM dataset WHERE name = 'demo'",
                [],
                |row| row.get(0),
            )
            .expect("the identity is read");
        assert_eq!(data_version, "source:1");
    }

    #[test]
    fn a_failed_insert_rolls_back_the_dataset_and_every_bar() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let first = unique_row(60, 0);
        let mut duplicate = unique_row(60, 0);
        duplicate.candle = candle(1_700_000_060, Some(1.0));

        let error = store
            .ingest(&record("demo", "source:1", 2), &[first, duplicate])
            .expect_err("the duplicate primary key fails the transaction");
        assert!(matches!(error, StoreError::Write { .. }));
        assert_eq!(count(&store, "dataset"), 0);
        assert_eq!(count(&store, "bar"), 0);
    }

    #[test]
    fn the_m1_cap_is_enforced_before_writing() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let error = store
            .ingest(&record("demo", "source:1", MAX_M1_BARS + 1), &[])
            .expect_err("the cap is enforced");
        assert!(matches!(error, StoreError::InvalidDataset { .. }));
        assert_eq!(count(&store, "dataset"), 0);
    }

    #[test]
    fn the_m1_count_must_match_the_rows() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let error = store
            .ingest(&record("demo", "source:1", 1), &[])
            .expect_err("a missing M1 row is rejected");
        assert!(matches!(error, StoreError::InvalidDataset { .. }));

        let error = store
            .ingest(&record("demo", "source:1", 0), &[unique_row(60, 0)])
            .expect_err("an undeclared M1 row is rejected");
        assert!(matches!(error, StoreError::InvalidDataset { .. }));
        assert_eq!(count(&store, "dataset"), 0);
    }

    #[test]
    fn an_empty_dataset_is_accepted() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let report = store
            .ingest(&record("empty", "source:0", 0), &[])
            .expect("an empty dataset is accepted");
        assert!(!report.unchanged);
        assert_eq!(report.bars, 0);
        assert_eq!(report.timeframes, 0);
        assert_eq!(count(&store, "dataset"), 1);
        assert_eq!(count(&store, "bar"), 0);
    }

    #[test]
    fn the_row_order_does_not_change_the_stored_set() {
        let mut first = Store::open_in_memory().expect("the first store opens");
        let mut second = Store::open_in_memory().expect("the second store opens");
        let rows = [unique_row(60, 0), unique_row(300, 0), unique_row(60, 1)];
        first
            .ingest(&record("demo", "source:1", 2), &rows)
            .expect("the first ingest");
        second
            .ingest(&record("demo", "source:1", 2), &[rows[2], rows[0], rows[1]])
            .expect("the reordered ingest");

        let read = |store: &Store| -> Vec<(i64, i64)> {
            let mut statement = store
                .connection()
                .prepare(
                    "SELECT timeframe_secs, bar_index FROM bar ORDER BY timeframe_secs, bar_index",
                )
                .expect("the query prepares");
            statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .expect("the rows are read")
                .collect::<Result<_, _>>()
                .expect("the rows are valid")
        };
        assert_eq!(read(&first), read(&second));
    }

    #[test]
    fn the_report_counts_distinct_timeframes() {
        let mut store = Store::open_in_memory().expect("the store opens");
        let rows = [unique_row(60, 0), unique_row(60, 1), unique_row(300, 0)];
        let report = store
            .ingest(&record("demo", "source:1", 2), &rows)
            .expect("the dataset is ingested");
        assert_eq!(report.bars, 3);
        assert_eq!(report.timeframes, 2);
    }

    #[test]
    fn the_insert_binds_62_parameters() {
        assert_eq!(BAR_INSERT.matches('?').count(), 62);
    }

    #[test]
    fn an_empty_name_or_data_version_is_rejected() {
        let mut store = Store::open_in_memory().expect("the store opens");
        for dataset in [record("", "source:1", 0), record("demo", "", 0)] {
            let error = store
                .ingest(&dataset, &[])
                .expect_err("an empty identity is rejected");
            assert!(matches!(error, StoreError::InvalidDataset { .. }));
        }
        assert_eq!(count(&store, "dataset"), 0);
    }
}
