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
//! The first backtest fills on the strategy ladder. A decision on bar `N` is
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
//! A gap at the open resolves first: an open at or beyond the target fills the
//! target even if the bar later reaches the stop. If the open is between both
//! levels and the bar reaches both, the stop wins. A fill's
//! [`Fill::reference_price`] is the price selected above; [`Fill::slippage`] is
//! the non-negative per-unit adjustment applied against the position, so a buy
//! pays `reference + slippage` and a sell receives `reference - slippage`.
//! [`Fill::fee`] is the total quote-currency fee for the fill. The policy does
//! not model an order book, partial fills, or intrabar M1 data.
//!
//! # Position bracket
//!
//! An entry carries a [`crate::intent::TradeIntent`]. Execution resolves the
//! stop and target against the fill price and the decision bar's ATR(14):
//!
//! | Stop spec | Long | Short |
//! |---|---|---|
//! | `AtrMultiple { multiple }` | `fill - multiple * ATR` | `fill + multiple * ATR` |
//! | `Price { price }` | `price` | `price` |
//!
//! A `RewardToRisk { multiple }` target is the stop distance multiplied by
//! `multiple`, projected on the profitable side of the fill; a
//! `Price { price }` target is used as given. Execution never chooses a
//! distance. A missing ATR is required only when the stop is ATR-based; an
//! absolute stop and an absolute target need no ATR. The bracket is live from
//! the entry bar onward, so one bar can fill the entry at its open and trigger
//! the stop or target later in the same bar.
//!
//! The entry is rejected with [`RejectReason::UnsupportedEntryStyle`] for an
//! entry style execution does not fill, with [`RejectReason::ExpiredIntent`]
//! when its submission bar is past `valid_until`, with
//! [`RejectReason::SlippageLimit`] when execution's configured slippage exceeds
//! the intent's `max_slippage_bps`, with
//! [`RejectReason::NonContiguousBar`] when the next supplied bar does not follow
//! the decision bar at the execution cadence, and with
//! [`RejectReason::InvalidBracket`] when the resolved levels do not bracket the
//! fill on the correct side. A rejection leaves the position unchanged.
//!
//! A bar with zero volume cannot fill a market order or trigger a bracket. An
//! absent volume is not zero: it is treated as tradable. An order that has no
//! next bar is rejected by [`ExecutionEngine::finish`].
//!
//! Orders that never trigger remain [`OrderState::Pending`]; an order with no
//! next bar or an invalid execution condition is [`OrderState::Rejected`]. The
//! engine publishes each order as an [`OrderRecord`] and returns it in an
//! [`ExecutionBatch`]. Position sizing, cash, equity, and P&L are outside this
//! module.

mod batch;
mod bracket;
mod config;
mod engine;
#[cfg(test)]
mod engine_tests;
mod error;
mod fill;
mod intent;
#[cfg(test)]
mod intent_tests;
mod order;
mod position;
#[cfg(test)]
mod tests;

pub use batch::{ExecutionBatch, Rejection};
pub use config::{ExecutionConfig, ExecutionConfigError};
pub use engine::ExecutionEngine;
pub use error::ExecutionError;
pub use fill::{Fill, FillError, FillReason};
pub use intent::OrderIntent;
pub use order::{Order, OrderError, OrderRecord, OrderSide, OrderState, RejectReason, Trigger};
pub use position::{Position, PositionError, PositionState};
