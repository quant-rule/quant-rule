//! The strategy rule contract.
//!
//! A strategy reads one closed model bar at a time and returns a causal
//! [`Decision`]. It holds no position state between calls: execution passes the
//! position it actually holds on every bar, and the strategy may read only the
//! bar it is given and its own previous inputs. It never creates an order, a
//! fill, a position, or a P&L record.
//!
//! An entry carries a [`TradeIntent`]: the side and the risk specification.
//! Execution resolves the intent. The strategy never chooses a fill price, fee,
//! or slippage.
//!
//! The first strategy is the axis-agreement rule: long when Direction,
//! Momentum force, and Structure score are all strictly above the long
//! threshold, short when all are strictly below the short threshold, and
//! neutral otherwise. Equality at a threshold is neutral. A missing reading is
//! unavailable, not neutral. [`AxisAgreementStrategy`] implements the rule.
//!
//! The rule is defined in
//! `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`.

mod agreement;
mod config;
mod decision;
mod input;
#[cfg(test)]
mod tests;
mod walk;

pub use crate::intent::{
    EntryStyle, RiskPolicy, RiskPolicyError, StopSpec, StrategyPosition, TargetSpec, TradeIntent,
};
pub use crate::side::Side;
pub use agreement::AxisAgreementStrategy;
pub use config::{
    DEFAULT_LONG_THRESHOLD, DEFAULT_SHORT_THRESHOLD, DEFAULT_TIMEFRAME_SECS, StrategyConfig,
    StrategyConfigError,
};
pub use decision::{Decision, Entry, Exit, ExitReason, Strategy};
pub use input::{Agreement, StrategyInput};
pub use walk::{AgreementCounts, StrategyWalk};
