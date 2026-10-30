//! The records one execution-engine call produces.
//!
//! The engine returns one [`ExecutionBatch`] per call. A report concatenates
//! the batches in call order and keeps the last order record per id. Rejections
//! live here too because they are an outcome of the same decision.

use super::{Fill, OrderRecord, RejectReason};

/// One decision the engine could not execute.
///
/// A rejection is not a fill. The engine records it so a caller can see that a
/// strategy decision had no effect; the strategy itself is not rewritten. The
/// rejection shares its `order_id` with the order record it resolved.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{Rejection, RejectReason};
///
/// let rejection = Rejection {
///     order_id: 3,
///     bar: 3,
///     time: 1_700_002_700,
///     reason: RejectReason::ZeroVolume,
/// };
/// assert_eq!(rejection.reason, RejectReason::ZeroVolume);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejection {
    /// The id of the order the decision became, or would have become.
    pub order_id: u64,
    /// The bar on which the engine refused the order. For
    /// [`RejectReason::NoNextBar`], this is the order's submission bar.
    pub bar: usize,
    /// That bar's open time as UTC Unix seconds.
    pub time: i64,
    /// Why the order was refused.
    pub reason: RejectReason,
}

/// The orders, fills, and rejections produced by one engine call.
///
/// [`ExecutionEngine::advance`], [`ExecutionEngine::submit`], and
/// [`ExecutionEngine::finish`] each return one batch. Concatenating the
/// batches in call order yields the run's records; a report keeps the last
/// order record per id.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::ExecutionBatch;
///
/// let batch = ExecutionBatch::default();
/// assert!(batch.is_empty());
/// assert!(batch.orders.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecutionBatch {
    /// Order records created or resolved by this call.
    pub orders: Vec<OrderRecord>,
    /// Fills produced by this call, in fill-id order.
    pub fills: Vec<Fill>,
    /// Decisions refused by this call, in detection order.
    pub rejections: Vec<Rejection>,
}

impl ExecutionBatch {
    /// Whether the call produced no orders, fills, or rejections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty() && self.fills.is_empty() && self.rejections.is_empty()
    }
}
