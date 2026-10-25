//! Bounded publication and full event batches.
//!
//! The model keeps two event views:
//!
//! - an [`EventBatch`] with every event computed in the run, for `events`, the
//!   store, and research;
//! - an [`EventHistory`] with the newest [`EVENT_SNAPSHOT_LIMIT`] records per
//!   timeframe, for the latest snapshot and the five-bar aggregates.
//!
//! A bar can produce several events, so the history bound counts records, not
//! candles. The batch is never truncated; the bound is a retention rule for the
//! published history only.

use crate::events::EventRecord;

/// The number of event records retained in a published per-timeframe history.
pub const EVENT_SNAPSHOT_LIMIT: usize = 1_024;

/// The full event record set for one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct EventBatch {
    records: Vec<EventRecord>,
}

impl EventBatch {
    /// Builds a batch from records already in availability order.
    #[must_use]
    pub fn new(records: Vec<EventRecord>) -> Self {
        Self { records }
    }

    /// Every record, in availability order.
    #[must_use]
    pub fn as_slice(&self) -> &[EventRecord] {
        &self.records
    }

    /// The number of records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the batch is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The newest record, or `None` when empty.
    #[must_use]
    pub fn latest(&self) -> Option<&EventRecord> {
        self.records.last()
    }
}

/// The newest bounded event records for one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct EventHistory {
    records: Vec<EventRecord>,
}

impl EventHistory {
    /// Keeps the newest [`EVENT_SNAPSHOT_LIMIT`] records from a batch.
    ///
    /// The batch is expected in availability order. The newest records are
    /// copied in their original order, so the published history is deterministic.
    #[must_use]
    pub fn from_batch(batch: &EventBatch) -> Self {
        let records = batch.as_slice();
        let start = records.len().saturating_sub(EVENT_SNAPSHOT_LIMIT);
        Self {
            records: records[start..].to_vec(),
        }
    }

    /// The retained records, oldest first.
    #[must_use]
    pub fn as_slice(&self) -> &[EventRecord] {
        &self.records
    }

    /// The number of retained records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the history is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The newest retained record, or `None` when empty.
    #[must_use]
    pub fn latest(&self) -> Option<&EventRecord> {
        self.records.last()
    }
}

#[cfg(test)]
mod tests {
    use super::{EVENT_SNAPSHOT_LIMIT, EventBatch, EventHistory};
    use crate::events::{EventRecord, EventType};

    fn record(index: usize) -> EventRecord {
        EventRecord {
            event_type: EventType::Signal,
            event_subtype_code: 1,
            source_code: 4,
            timeframe_secs: 60,
            occurred_bar_index: index,
            available_bar_index: index,
            direction_code: 1,
            reference_code: None,
            primary_value: Some(index as f64),
            reference_value: Some(30.0),
        }
    }

    #[test]
    fn a_short_batch_is_retained_whole() {
        let batch = EventBatch::new((0..10).map(record).collect());
        let history = EventHistory::from_batch(&batch);
        assert_eq!(history.len(), 10);
        assert_eq!(history.as_slice(), batch.as_slice());
    }

    #[test]
    fn a_long_batch_retains_the_newest_records_in_order() {
        let batch = EventBatch::new((0..(EVENT_SNAPSHOT_LIMIT + 10)).map(record).collect());
        let history = EventHistory::from_batch(&batch);
        assert_eq!(history.len(), EVENT_SNAPSHOT_LIMIT);
        assert_eq!(
            history.as_slice().first().map(|event| event.occurred_bar_index),
            Some(10)
        );
        assert_eq!(
            history.as_slice().last().map(|event| event.occurred_bar_index),
            Some(EVENT_SNAPSHOT_LIMIT + 9)
        );
        assert_eq!(history.as_slice(), &batch.as_slice()[10..]);
    }
}
