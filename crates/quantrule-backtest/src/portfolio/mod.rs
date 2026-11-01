//! Portfolio types: configuration, sizing, state, records, and the ledger.
//!
//! The module owns the capital side of a backtest. It defines the configuration
//! and the sizing rule, the state the ledger maintains, the records the ledger
//! publishes, and the accounting state machine that turns fills into cash,
//! equity, trades, funding charges, and drawdown.
//!
//! The ledger reads no clock, performs no I/O, and uses no randomness. It takes
//! execution's [`crate::execution::Fill`] as its input; execution does not know
//! the portfolio exists.

mod config;
mod ledger;
#[cfg(test)]
mod ledger_tests;
mod records;
mod sizing;
mod state;
#[cfg(test)]
mod tests;

pub use config::{
    CommissionBase, CommissionConfig, CommissionPer, Currency, DrawdownAction, FundingConfig,
    FundingInterval, MarkToMarket, MaxDrawdownConfig, PortfolioConfig, PortfolioConfigError,
    SUPPORTED_LEVERAGE, SUPPORTED_MAX_OPEN_POSITIONS,
};
pub use ledger::{PortfolioLedger, PortfolioLedgerError};
pub use records::{
    EquityPoint, ExitReason, FundingCharge, MonthlyReturn, PortfolioMetrics, RiskEvent, TradeRecord,
};
pub(crate) use sizing::floored_steps;
pub use sizing::{SizingMode, SizingRejection};
pub use state::{OpenPosition, OpenPositionError, PortfolioState, PortfolioStateError};
