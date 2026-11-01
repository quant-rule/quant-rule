//! Writer/reader consistency verification of a stored dataset.
//!
//! The command builds the layers from the source, reads every stored table
//! read-only, and compares the values table by table through the same writer
//! parameter functions. It detects drift between the writer and the reader; it
//! is not an independent oracle for the model or the layer computation. It
//! never writes, migrates, or repairs the database.

use super::ingest::{DatasetRecord, IngestRows, MODEL_CONTRACT, RESEARCH_CONTRACT, expected_rows};
use super::{Cell, Store, StoreError, Table};

/// The result of a successful writer/reader consistency check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VerifyReport {
    /// The number of expected `bar` rows.
    pub(crate) bars: usize,
    /// The number of compared record tables.
    pub(crate) tables: usize,
}

impl Store {
    /// Compares a stored dataset with the values a fresh build would write.
    ///
    /// The check covers the dataset identity, the foreign-key graph, and the
    /// `bar`, `consensus`, `layer3`, `pivot`, `level`, `event`, and `leg`
    /// tables. It uses the writer's parameter functions, so it proves
    /// writer/reader consistency rather than the correctness of an independent
    /// computation. The event `id` is a row identifier and is excluded from the
    /// record comparison; every record column is compared exactly.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::DatasetNotFound`] when the dataset is missing,
    /// [`StoreError::IncompleteDataset`] when `layer3_materialized` is not `1`,
    /// [`StoreError::VerificationMismatch`] for a value, count, or orphan
    /// difference, and [`StoreError::Query`] when a read fails.
    pub(crate) fn verify(
        &self,
        dataset: &DatasetRecord,
        rows: &IngestRows<'_>,
    ) -> Result<VerifyReport, StoreError> {
        let dataset_id = self.dataset_id(&dataset.name)?;
        let stored = self.read_dataset(dataset_id)?;
        check_dataset(dataset, &stored)?;
        if stored.layer3_materialized != 1 {
            return Err(StoreError::IncompleteDataset {
                name: dataset.name.clone(),
            });
        }
        self.check_foreign_keys()?;

        let mut tables = 0;
        for (table, expected) in expected_rows(dataset_id, rows) {
            let mut stored = self.raw_table_rows(dataset_id, table)?;
            if table == Table::Event {
                remove_event_id(&mut stored.columns, &mut stored.rows);
            }
            compare_table(table, &stored.columns, &stored.rows, &expected)?;
            tables += 1;
        }

        Ok(VerifyReport {
            bars: rows.bars.len(),
            tables,
        })
    }

    fn read_dataset(&self, dataset_id: i64) -> Result<StoredDataset, StoreError> {
        self.connection
            .query_row(
                "SELECT data_version, model_version, structure_scale_code, m1_bars, \
                        layer3_materialized, model_contract, research_contract \
                 FROM dataset WHERE id = ?1",
                [dataset_id],
                |row| {
                    Ok(StoredDataset {
                        data_version: row.get(0)?,
                        model_version: row.get(1)?,
                        structure_scale_code: row.get(2)?,
                        m1_bars: row.get(3)?,
                        layer3_materialized: row.get(4)?,
                        model_contract: row.get(5)?,
                        research_contract: row.get(6)?,
                    })
                },
            )
            .map_err(|source| StoreError::Query { source })
    }

    fn check_foreign_keys(&self) -> Result<(), StoreError> {
        let mut statement = self
            .connection
            .prepare("PRAGMA foreign_key_check")
            .map_err(|source| StoreError::Query { source })?;
        let mut rows = statement
            .query([])
            .map_err(|source| StoreError::Query { source })?;
        if rows
            .next()
            .map_err(|source| StoreError::Query { source })?
            .is_some()
        {
            return Err(StoreError::VerificationMismatch {
                detail: String::from("PRAGMA foreign_key_check reported an orphan row"),
            });
        }
        Ok(())
    }
}

struct StoredDataset {
    data_version: String,
    model_version: String,
    structure_scale_code: i64,
    m1_bars: i64,
    layer3_materialized: i64,
    model_contract: String,
    research_contract: String,
}

fn check_dataset(dataset: &DatasetRecord, stored: &StoredDataset) -> Result<(), StoreError> {
    let pairs = [
        (
            "data_version",
            stored.data_version.as_str(),
            dataset.data_version.as_str(),
        ),
        (
            "model_version",
            stored.model_version.as_str(),
            dataset.model_version.as_str(),
        ),
        (
            "model_contract",
            stored.model_contract.as_str(),
            MODEL_CONTRACT,
        ),
        (
            "research_contract",
            stored.research_contract.as_str(),
            RESEARCH_CONTRACT,
        ),
    ];
    for (name, stored, expected) in pairs {
        if stored != expected {
            return Err(StoreError::VerificationMismatch {
                detail: format!("dataset {name}: stored '{stored}', expected '{expected}'"),
            });
        }
    }
    if stored.structure_scale_code != i64::from(dataset.structure_scale_code) {
        return Err(StoreError::VerificationMismatch {
            detail: format!(
                "dataset structure_scale_code: stored {}, expected {}",
                stored.structure_scale_code, dataset.structure_scale_code
            ),
        });
    }
    if stored.m1_bars != dataset.m1_bars as i64 {
        return Err(StoreError::VerificationMismatch {
            detail: format!(
                "dataset m1_bars: stored {}, expected {}",
                stored.m1_bars, dataset.m1_bars
            ),
        });
    }
    Ok(())
}

fn remove_event_id(columns: &mut Vec<String>, rows: &mut [Vec<Cell>]) {
    let Some(index) = columns.iter().position(|column| column == "id") else {
        return;
    };
    columns.remove(index);
    for row in rows {
        if index < row.len() {
            row.remove(index);
        }
    }
}

fn compare_table(
    table: Table,
    stored_columns: &[String],
    stored: &[Vec<Cell>],
    expected: &[Vec<rusqlite::types::Value>],
) -> Result<(), StoreError> {
    if stored.len() != expected.len() {
        return Err(StoreError::VerificationMismatch {
            detail: format!(
                "{} row count: stored {}, expected {}",
                table.sql_name(),
                stored.len(),
                expected.len()
            ),
        });
    }
    for (row_index, (stored_row, expected_row)) in stored.iter().zip(expected).enumerate() {
        if stored_row.len() != expected_row.len() {
            return Err(StoreError::VerificationMismatch {
                detail: format!(
                    "{} row {} width: stored {}, expected {}",
                    table.sql_name(),
                    row_index,
                    stored_row.len(),
                    expected_row.len()
                ),
            });
        }
        for (column_index, (stored, expected)) in stored_row.iter().zip(expected_row).enumerate() {
            let expected = Cell::from_value(expected.clone());
            if stored != &expected {
                let column = stored_columns
                    .get(column_index)
                    .map_or("<unknown>", String::as_str);
                return Err(StoreError::VerificationMismatch {
                    detail: format!(
                        "{} row {} column {column}: stored {:?}, expected {:?}",
                        table.sql_name(),
                        row_index,
                        stored,
                        expected
                    ),
                });
            }
        }
    }
    Ok(())
}
