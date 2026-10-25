//! Dataset lifecycle operations.
//!
//! Removing a dataset is explicit and cascades through every record table. The
//! database file is never deleted; a caller can remove one dataset and keep the
//! rest of the evidence.

use rusqlite::{TransactionBehavior, params};

use super::{Store, StoreError};

/// The result of removing one dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoveReport {
    /// The removed dataset name.
    pub(crate) name: String,
    /// The number of stored bar rows that were removed.
    pub(crate) bars: usize,
}

impl Store {
    /// Removes one dataset and every row that references it.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::DatasetNotFound`] when the name does not exist,
    /// and [`StoreError::Write`] when the delete fails and rolls back.
    pub(crate) fn remove(&mut self, name: &str) -> Result<RemoveReport, StoreError> {
        let dataset_id = self.dataset_id(name)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|source| StoreError::Write { source })?;

        let bars: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM bar WHERE dataset_id = ?1",
                [dataset_id],
                |row| row.get(0),
            )
            .map_err(|source| StoreError::Query { source })?;
        transaction
            .execute("DELETE FROM dataset WHERE id = ?1", params![dataset_id])
            .map_err(|source| StoreError::Write { source })?;
        transaction
            .commit()
            .map_err(|source| StoreError::Write { source })?;

        Ok(RemoveReport {
            name: name.to_owned(),
            bars: usize::try_from(bars).unwrap_or(usize::MAX),
        })
    }
}
