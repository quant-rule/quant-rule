//! The execution-facing order intent.
//!
//! The runner maps a strategy decision onto this type before calling the
//! execution engine. Execution therefore reads its own vocabulary and never
//! imports the strategy module. The entry carries the strategy's
//! [`crate::intent::TradeIntent`]; execution resolves it against the fill.

use crate::intent::TradeIntent;
use crate::side::Side;

/// The execution request for one strategy bar.
///
/// `Hold` is the absence of an order; the engine still returns an empty batch so
/// the runner keeps one batch per bar. `Enter` and `Exit` carry the bar identity
/// needed to reject an out-of-order submission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderIntent {
    /// No order on this bar.
    Hold,
    /// Open a position on the next bar.
    Enter {
        /// The strategy's complete entry request.
        intent: TradeIntent,
        /// The zero-based bar index that produced the decision.
        bar_index: usize,
        /// The decision bar's open time as UTC Unix seconds.
        open_time: i64,
        /// The largest entry notional allowed at the fill, when the caller
        /// enforces a risk envelope. `None` disables the check.
        max_notional: Option<f64>,
    },
    /// Close the open position on the next bar.
    ///
    /// `side` must match the open position; a mismatch is rejected.
    Exit {
        /// The side the strategy believes is open.
        side: Side,
        /// The zero-based bar index that produced the decision.
        bar_index: usize,
        /// The decision bar's open time as UTC Unix seconds.
        open_time: i64,
    },
}
