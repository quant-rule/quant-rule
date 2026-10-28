//! Execution vocabulary and the causal fill simulator.
//!
//! Execution turns a strategy entry or exit into an order and, on a later
//! candle, into a fill. It owns no portfolio accounting: fees and slippage are
//! recorded on the fill, but cash, equity, and P&L belong to the portfolio
//! module. Position sizing is outside this module: the caller supplies the
//! quantity with each entry decision.
//!
//! # Fill policy
//!
//! The first backtest fills on the M15 ladder. A decision on bar `N` is
//! submitted as an order on bar `N`; the order may fill on bar `N + 1`, never
//! on bar `N`. No later bar is read to decide an earlier fill. A bar is
//! processed in two calls: [`ExecutionEngine::advance`] resolves the previous
//! bar's order and the open position's bracket, then
//! [`ExecutionEngine::submit`] schedules the bar's decision for the next bar.
//!
//! | Order | Reference price on the next bar |
//! |---|---|
//! | Market entry or exit | the bar's open |
//! | Stop, long | the open when it gaps at or below the stop, otherwise the stop when the bar's low reaches it |
//! | Stop, short | the open when it gaps at or above the stop, otherwise the stop when the bar's high reaches it |
//! | Target, long | the open when it gaps at or above the target, otherwise the target when the bar's high reaches it |
//! | Target, short | the open when it gaps at or below the target, otherwise the target when the bar's low reaches it |
//!
//! If a bar reaches both the stop and the target, the stop wins. A fill's
//! [`Fill::reference_price`] is the price selected above; [`Fill::slippage`] is
//! the non-negative per-unit adjustment applied against the position, so a buy
//! pays `reference + slippage` and a sell receives `reference - slippage`.
//! [`Fill::fee`] is the total quote-currency fee for the fill. The policy does
//! not model an order book, partial fills, or intrabar M1 data.
//!
//! # Position bracket
//!
//! An entry fill opens a position bracketed by the decision bar's ATR(14),
//! anchored at the fill price: a long stop is `fill - 2 * ATR` and its target
//! is `fill + 3 * ATR`; a short stop is `fill + 2 * ATR` and its target is
//! `fill - 3 * ATR`. The bracket is live from the entry bar onward, so one bar
//! can fill the entry at its open and trigger the stop or target later in the
//! same bar. A missing or non-positive ATR rejects the entry.
//!
//! A bar with zero volume cannot fill a market order or trigger a bracket. An
//! absent volume is not zero: it is treated as tradable. An order that has no
//! next bar is rejected by [`ExecutionEngine::finish`].
//!
//! Orders that never trigger remain [`OrderState::Pending`]; an order with no
//! next bar or an invalid execution condition is [`OrderState::Rejected`].
//! Position sizing, cash, equity, and P&L are outside this module.

mod config;
mod engine;
#[cfg(test)]
mod engine_tests;
mod fill;
mod order;
mod position;
#[cfg(test)]
mod tests;

pub use config::{ExecutionConfig, ExecutionConfigError};
pub use engine::{
    ExecutionEngine, ExecutionError, Rejection, STOP_ATR_MULTIPLE, TARGET_ATR_MULTIPLE,
};
pub use fill::{Fill, FillError, FillReason};
pub use order::{Order, OrderError, OrderSide, OrderState, RejectReason, Trigger};
pub use position::{Position, PositionError, PositionState};
