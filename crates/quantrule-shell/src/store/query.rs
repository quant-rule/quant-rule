//! Read-only queries over the stored dataset.
//!
//! The query layer owns table names, primary-key ordering, cell conversion, and
//! the event reference sentinel. It reads stored values; it never computes a
//! reading, metric, resolution, aggregate, event, or leg.

use std::io::{self, Write};

use rusqlite::OptionalExtension;
use rusqlite::params_from_iter;
use rusqlite::types::Value;

use super::{Store, StoreError};

fn strip_internal_columns(rows: &mut TableRows) {
    let mut hidden: Vec<usize> = rows
        .columns
        .iter()
        .enumerate()
        .filter(|(_, name)| matches!(name.as_str(), "dataset_id" | "id"))
        .map(|(index, _)| index)
        .collect();
    hidden.sort_unstable_by(|left, right| right.cmp(left));
    for index in hidden {
        rows.columns.remove(index);
        for row in &mut rows.rows {
            if index < row.len() {
                row.remove(index);
            }
        }
    }
}

/// A stored table that can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Table {
    /// The wide per-bar table.
    Bar,
    /// The per-bar consensus table.
    Consensus,
    /// The materialized Layer 3 table.
    Layer3,
    /// The pivot occurrences.
    Pivot,
    /// The confirmed levels.
    Level,
    /// The event records.
    Event,
    /// The retained progression legs.
    Leg,
}

impl Table {
    /// Resolves the command's table name.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "bar" => Some(Self::Bar),
            "consensus" => Some(Self::Consensus),
            "layer3" => Some(Self::Layer3),
            "pivot" => Some(Self::Pivot),
            "level" => Some(Self::Level),
            "event" => Some(Self::Event),
            "leg" => Some(Self::Leg),
            _ => None,
        }
    }

    pub(crate) const fn sql_name(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Consensus => "consensus",
            Self::Layer3 => "layer3",
            Self::Pivot => "pivot",
            Self::Level => "level",
            Self::Event => "event",
            Self::Leg => "leg",
        }
    }

    const fn order_by(self) -> &'static str {
        match self {
            Self::Bar => "timeframe_secs, bar_index",
            Self::Consensus => "timeframe_secs, bar_index",
            Self::Layer3 => "bar_index",
            Self::Pivot => "timeframe_secs, occurred_bar_index, scale_code, kind_code",
            Self::Level => "timeframe_secs, reference_code",
            Self::Event => {
                "timeframe_secs, available_bar_index, event_type_code, event_subtype_code, \
                 source_code, occurred_bar_index, reference_code, reference_value"
            }
            Self::Leg => "timeframe_secs, leg_id",
        }
    }

    pub(crate) const fn has_timeframe(self) -> bool {
        !matches!(self, Self::Layer3)
    }

    pub(crate) const fn has_bar_index(self) -> bool {
        matches!(self, Self::Bar | Self::Consensus | Self::Layer3)
    }
}

/// One stored cell in a query result.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cell {
    /// SQL NULL.
    Null,
    /// An integer.
    Integer(i64),
    /// A REAL.
    Real(f64),
    /// Text.
    Text(String),
}

impl Cell {
    pub(crate) fn from_value(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Integer(value) => Self::Integer(value),
            Value::Real(value) => Self::Real(value),
            Value::Text(value) => Self::Text(value),
            Value::Blob(value) => Self::Text(format!("<blob:{}>", value.len())),
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => format_float(*value),
            Self::Text(value) => value.clone(),
        }
    }
}

/// A table of stored values in SQL column order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TableRows {
    /// The column names in result order.
    pub(crate) columns: Vec<String>,
    /// The rows in query order.
    pub(crate) rows: Vec<Vec<Cell>>,
}

impl TableRows {
    /// Writes the table as CSV with a header and an empty field for NULL.
    ///
    /// # Errors
    ///
    /// Returns the writer's error.
    pub(crate) fn write_csv(&self, writer: &mut impl Write) -> io::Result<()> {
        for (index, column) in self.columns.iter().enumerate() {
            if index > 0 {
                writer.write_all(b",")?;
            }
            write_csv_field(writer, column)?;
        }
        writer.write_all(b"\n")?;
        for row in &self.rows {
            for (index, cell) in row.iter().enumerate() {
                if index > 0 {
                    writer.write_all(b",")?;
                }
                write_csv_field(writer, &cell.text())?;
            }
            writer.write_all(b"\n")?;
        }
        Ok(())
    }
}

/// One dataset summary from `store list`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasetSummary {
    /// The dataset name.
    pub(crate) name: String,
    /// The caller-supplied data version.
    pub(crate) data_version: String,
    /// The workspace package version.
    pub(crate) model_version: String,
    /// The structure scale code.
    pub(crate) structure_scale_code: i64,
    /// The accepted M1 bar count.
    pub(crate) m1_bars: i64,
    /// Whether Layer 3 is materialized.
    pub(crate) layer3_materialized: i64,
    /// The number of `bar` rows.
    pub(crate) bar_rows: i64,
    /// The number of `consensus` rows.
    pub(crate) consensus_rows: i64,
    /// The number of `layer3` rows.
    pub(crate) layer3_rows: i64,
    /// The number of `pivot` rows.
    pub(crate) pivot_rows: i64,
    /// The number of `level` rows.
    pub(crate) level_rows: i64,
    /// The number of `event` rows.
    pub(crate) event_rows: i64,
    /// The number of `leg` rows.
    pub(crate) leg_rows: i64,
}

/// One candle row for `db series`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SeriesRow {
    /// The source timeframe in seconds.
    pub(crate) timeframe_secs: u64,
    /// The 0-based bar position.
    pub(crate) bar_index: usize,
    /// The bar's open time as Unix seconds.
    pub(crate) open_time: i64,
    /// Open price.
    pub(crate) open: f64,
    /// High price.
    pub(crate) high: f64,
    /// Low price.
    pub(crate) low: f64,
    /// Close price.
    pub(crate) close: f64,
    /// Volume, when present.
    pub(crate) volume: Option<f64>,
}

const METRIC_COLUMNS: [&str; 47] = [
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

const RESOLUTION_COLUMNS: [&str; 12] = [
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
];

impl Store {
    /// Lists every dataset in name order with its row counts.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Query`] when the statement fails.
    pub(crate) fn list_datasets(&self) -> Result<Vec<DatasetSummary>, StoreError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.name, d.data_version, d.model_version, d.structure_scale_code, \
                        d.m1_bars, d.layer3_materialized, \
                        (SELECT count(*) FROM bar b WHERE b.dataset_id = d.id), \
                        (SELECT count(*) FROM consensus c WHERE c.dataset_id = d.id), \
                        (SELECT count(*) FROM layer3 l WHERE l.dataset_id = d.id), \
                        (SELECT count(*) FROM pivot p WHERE p.dataset_id = d.id), \
                        (SELECT count(*) FROM level l WHERE l.dataset_id = d.id), \
                        (SELECT count(*) FROM event e WHERE e.dataset_id = d.id), \
                        (SELECT count(*) FROM leg g WHERE g.dataset_id = d.id) \
                 FROM dataset d ORDER BY d.name",
            )
            .map_err(|source| StoreError::Query { source })?;
        let rows = statement
            .query_map([], |row| {
                Ok(DatasetSummary {
                    name: row.get(0)?,
                    data_version: row.get(1)?,
                    model_version: row.get(2)?,
                    structure_scale_code: row.get(3)?,
                    m1_bars: row.get(4)?,
                    layer3_materialized: row.get(5)?,
                    bar_rows: row.get(6)?,
                    consensus_rows: row.get(7)?,
                    layer3_rows: row.get(8)?,
                    pivot_rows: row.get(9)?,
                    level_rows: row.get(10)?,
                    event_rows: row.get(11)?,
                    leg_rows: row.get(12)?,
                })
            })
            .map_err(|source| StoreError::Query { source })?;
        rows.collect::<Result<_, _>>()
            .map_err(|source| StoreError::Query { source })
    }

    /// Reads every row of a stored table for verification.
    ///
    /// The rows keep the same column order and primary-key order as `show_table`
    /// but have no filters, limit, or display conversion.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Query`] when the statement fails.
    pub(crate) fn raw_table_rows(
        &self,
        dataset_id: i64,
        table: Table,
    ) -> Result<TableRows, StoreError> {
        let sql = format!(
            "SELECT * FROM {} WHERE dataset_id = ? ORDER BY {}",
            table.sql_name(),
            table.order_by()
        );
        self.query_rows(&sql, vec![Value::from(dataset_id)])
    }

    /// Shows a stored table for one dataset.
    ///
    /// The database's internal `dataset_id` and the event row `id` are omitted
    /// from the displayed columns: they are storage identity, not dataset
    /// content, and they change across database histories.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::DatasetNotFound`] when the dataset does not exist,
    /// [`StoreError::InvalidQuery`] when a filter does not apply to the table,
    /// and [`StoreError::Query`] when the statement fails.
    pub(crate) fn show_table(
        &self,
        dataset: &str,
        table: Table,
        timeframe: Option<u64>,
        bar: Option<usize>,
        limit: usize,
    ) -> Result<TableRows, StoreError> {
        if timeframe.is_some() && !table.has_timeframe() {
            return Err(StoreError::InvalidQuery {
                detail: format!("{} has no timeframe_secs column", table.sql_name()),
            });
        }
        if bar.is_some() && !table.has_bar_index() {
            return Err(StoreError::InvalidQuery {
                detail: format!("{} has no bar_index column", table.sql_name()),
            });
        }

        let dataset_id = self.dataset_id(dataset)?;
        let mut sql = format!("SELECT * FROM {} WHERE dataset_id = ?", table.sql_name());
        let mut values = vec![Value::from(dataset_id)];
        if let Some(timeframe) = timeframe {
            sql.push_str(" AND timeframe_secs = ?");
            values.push(Value::from(timeframe as i64));
        }
        if let Some(bar) = bar {
            sql.push_str(" AND bar_index = ?");
            values.push(Value::from(bar as i64));
        }
        sql.push_str(" ORDER BY ");
        sql.push_str(table.order_by());
        sql.push_str(" LIMIT ?");
        values.push(Value::from(limit as i64));

        let mut rows = self.query_rows(&sql, values)?;
        strip_internal_columns(&mut rows);
        if table == Table::Event
            && let Some(index) = rows
                .columns
                .iter()
                .position(|name| name == "reference_code")
        {
            for row in &mut rows.rows {
                if row[index] == Cell::Integer(0) {
                    row[index] = Cell::Null;
                }
            }
        }
        Ok(rows)
    }

    /// Reads the candle-only series, oldest first.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::DatasetNotFound`] when the dataset does not exist
    /// and [`StoreError::Query`] when the statement fails.
    pub(crate) fn series(
        &self,
        dataset: &str,
        timeframe: Option<u64>,
        limit: usize,
    ) -> Result<Vec<SeriesRow>, StoreError> {
        let dataset_id = self.dataset_id(dataset)?;
        let mut sql = String::from(
            "SELECT timeframe_secs, bar_index, open_time, open, high, low, close, volume \
             FROM bar WHERE dataset_id = ?",
        );
        let mut values = vec![Value::from(dataset_id)];
        if let Some(timeframe) = timeframe {
            sql.push_str(" AND timeframe_secs = ?");
            values.push(Value::from(timeframe as i64));
        }
        sql.push_str(" ORDER BY timeframe_secs, bar_index LIMIT ?");
        values.push(Value::from(limit as i64));

        let mut statement = self
            .connection
            .prepare(&sql)
            .map_err(|source| StoreError::Query { source })?;
        let rows = statement
            .query_map(params_from_iter(values), |row| {
                Ok(SeriesRow {
                    timeframe_secs: row.get::<_, i64>(0)? as u64,
                    bar_index: row.get::<_, i64>(1)? as usize,
                    open_time: row.get(2)?,
                    open: row.get(3)?,
                    high: row.get(4)?,
                    low: row.get(5)?,
                    close: row.get(6)?,
                    volume: row.get(7)?,
                })
            })
            .map_err(|source| StoreError::Query { source })?;
        rows.collect::<Result<_, _>>()
            .map_err(|source| StoreError::Query { source })
    }

    /// Reads the 47 metric and 12 resolution columns for one dataset.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::DatasetNotFound`] when the dataset does not exist
    /// and [`StoreError::Query`] when the statement fails.
    pub(crate) fn metrics(
        &self,
        dataset: &str,
        timeframe: Option<u64>,
        bar: Option<usize>,
        limit: usize,
    ) -> Result<TableRows, StoreError> {
        let dataset_id = self.dataset_id(dataset)?;
        let mut select = String::from("timeframe_secs, bar_index");
        for column in METRIC_COLUMNS.iter().chain(RESOLUTION_COLUMNS.iter()) {
            select.push_str(", ");
            select.push_str(column);
        }
        let mut sql = format!("SELECT {select} FROM bar WHERE dataset_id = ?");
        let mut values = vec![Value::from(dataset_id)];
        if let Some(timeframe) = timeframe {
            sql.push_str(" AND timeframe_secs = ?");
            values.push(Value::from(timeframe as i64));
        }
        if let Some(bar) = bar {
            sql.push_str(" AND bar_index = ?");
            values.push(Value::from(bar as i64));
        }
        sql.push_str(" ORDER BY timeframe_secs, bar_index LIMIT ?");
        values.push(Value::from(limit as i64));
        self.query_rows(&sql, values)
    }

    pub(crate) fn dataset_id(&self, name: &str) -> Result<i64, StoreError> {
        let id: Option<i64> = self
            .connection
            .query_row("SELECT id FROM dataset WHERE name = ?1", [name], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|source| StoreError::Query { source })?;
        id.ok_or_else(|| StoreError::DatasetNotFound {
            name: name.to_string(),
        })
    }

    fn query_rows(&self, sql: &str, values: Vec<Value>) -> Result<TableRows, StoreError> {
        let mut statement = self
            .connection
            .prepare(sql)
            .map_err(|source| StoreError::Query { source })?;
        let columns: Vec<String> = statement
            .column_names()
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        let width = statement.column_count();
        let rows = statement
            .query_map(params_from_iter(values), |row| {
                (0..width)
                    .map(|index| row.get::<_, Value>(index).map(Cell::from_value))
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|source| StoreError::Query { source })?;
        let rows = rows
            .collect::<Result<_, _>>()
            .map_err(|source| StoreError::Query { source })?;
        Ok(TableRows { columns, rows })
    }
}

fn write_csv_field(writer: &mut impl Write, text: &str) -> io::Result<()> {
    if text
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        writer.write_all(b"\"")?;
        for character in text.chars() {
            if character == '"' {
                writer.write_all(b"\"\"")?;
            } else {
                writer.write_all(character.encode_utf8(&mut [0; 4]).as_bytes())?;
            }
        }
        writer.write_all(b"\"")
    } else {
        writer.write_all(text.as_bytes())
    }
}

fn format_float(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{Cell, Table, TableRows};
    use crate::store::{Store, StoreError};

    fn store() -> Store {
        let store = Store::open_in_memory().expect("the store opens");
        store
            .connection()
            .execute_batch(
                "INSERT INTO dataset \
                 (name, data_version, model_version, model_contract, research_contract, \
                  structure_scale_code, m1_bars, layer3_materialized) \
                 VALUES ('demo', 'source:1', '0.1.0', 'm', 'r', 2, 2, 1);\
                 INSERT INTO bar \
                 (dataset_id, timeframe_secs, bar_index, open_time, open, high, low, close, \
                  direction_readiness_code, direction_state_age_bars, direction_state_changes_50, \
                  momentum_readiness_code, momentum_state_age_bars, momentum_state_changes_50, \
                  structure_scale_code, structure_readiness_code, structure_state_age_bars, \
                  structure_state_changes_50, progression_readiness_code, \
                  progression_state_age_bars, progression_state_changes_50) \
                 VALUES (1, 60, 1, 1700000060, 10.0, 11.0, 9.0, 10.5, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0),\
                        (1, 60, 0, 1700000000, 10.0, 11.0, 9.0, 10.0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0),\
                        (1, 300, 0, 1699999800, 10.0, 11.0, 9.0, 10.2, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0);\
                 INSERT INTO layer3 (dataset_id, bar_index) VALUES (1, 0), (1, 1);\
                 INSERT INTO event \
                 (dataset_id, timeframe_secs, event_type_code, event_subtype_code, source_code, \
                  occurred_bar_index, available_bar_index, direction_code, reference_code) \
                 VALUES (1, 60, 1, 1, 1, 0, 0, 0, 0);",
            )
            .expect("the fixture rows are inserted");
        store
    }

    #[test]
    fn lists_datasets_in_name_order_with_counts() {
        let store = store();
        let rows = store.list_datasets().expect("the datasets are read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "demo");
        assert_eq!(rows[0].bar_rows, 3);
        assert_eq!(rows[0].layer3_rows, 2);
        assert_eq!(rows[0].event_rows, 1);
        assert_eq!(rows[0].layer3_materialized, 1);
    }

    #[test]
    fn shows_a_table_in_primary_key_order_and_honours_filters() {
        let store = store();
        let rows = store
            .show_table("demo", Table::Bar, Some(60), None, 10)
            .expect("the bars are read");
        assert_eq!(rows.rows.len(), 2);
        assert_eq!(rows.rows[0][1], Cell::Integer(0));
        assert_eq!(rows.rows[1][1], Cell::Integer(1));

        let one = store
            .show_table("demo", Table::Bar, Some(60), Some(1), 10)
            .expect("the bar is read");
        assert_eq!(one.rows.len(), 1);
        assert_eq!(one.rows[0][1], Cell::Integer(1));

        let limited = store
            .show_table("demo", Table::Bar, None, None, 1)
            .expect("the limit is applied");
        assert_eq!(limited.rows.len(), 1);
    }

    #[test]
    fn the_event_reference_sentinel_is_rendered_as_null() {
        let store = store();
        let rows = store
            .show_table("demo", Table::Event, None, None, 10)
            .expect("the events are read");
        let index = rows
            .columns
            .iter()
            .position(|column| column == "reference_code")
            .expect("the reference column");
        assert_eq!(rows.rows[0][index], Cell::Null);
    }

    #[test]
    fn an_unknown_dataset_is_an_error() {
        let store = store();
        assert!(matches!(
            store.show_table("missing", Table::Bar, None, None, 10),
            Err(StoreError::DatasetNotFound { .. })
        ));
    }

    #[test]
    fn an_inapplicable_filter_is_an_error() {
        let store = store();
        assert!(matches!(
            store.show_table("demo", Table::Layer3, Some(60), None, 10),
            Err(StoreError::InvalidQuery { .. })
        ));
        assert!(matches!(
            store.show_table("demo", Table::Event, None, Some(0), 10),
            Err(StoreError::InvalidQuery { .. })
        ));
    }

    #[test]
    fn series_is_oldest_first_and_timeframe_filtered() {
        let store = store();
        let rows = store
            .series("demo", Some(60), 10)
            .expect("the series is read");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].bar_index, 0);
        assert_eq!(rows[1].bar_index, 1);
        assert_eq!(rows[0].open_time, 1_700_000_000);
        assert_eq!(rows[0].close, 10.0);
    }

    #[test]
    fn metrics_returns_the_identity_and_the_full_metric_width() {
        let store = store();
        let rows = store
            .metrics("demo", Some(60), Some(0), 10)
            .expect("the metrics are read");
        assert_eq!(rows.rows.len(), 1);
        assert_eq!(rows.columns.len(), 61);
        assert_eq!(rows.columns[0], "timeframe_secs");
        assert_eq!(rows.columns[1], "bar_index");
        assert_eq!(rows.columns[2], "direction");
        assert_eq!(rows.columns[60], "progression_resolution_reason_code");
    }

    #[test]
    fn csv_uses_empty_null_and_quotes_separators() {
        let table = TableRows {
            columns: vec!["a".to_string(), "b".to_string()],
            rows: vec![
                vec![Cell::Null, Cell::Text(String::from("x,y"))],
                vec![Cell::Integer(1), Cell::Real(-0.0)],
            ],
        };
        let mut output = Vec::new();
        table.write_csv(&mut output).expect("the table writes");
        assert_eq!(
            String::from_utf8(output).expect("the CSV is UTF-8"),
            "a,b\n,\"x,y\"\n1,0\n"
        );
    }

    #[test]
    fn table_names_resolve_exactly() {
        for (name, table) in [
            ("bar", Table::Bar),
            ("consensus", Table::Consensus),
            ("layer3", Table::Layer3),
            ("pivot", Table::Pivot),
            ("level", Table::Level),
            ("event", Table::Event),
            ("leg", Table::Leg),
        ] {
            assert_eq!(Table::from_name(name), Some(table));
        }
        assert_eq!(Table::from_name("Bar"), None);
    }
}
