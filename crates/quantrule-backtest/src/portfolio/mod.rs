//! Portfolio types: configuration, sizing, state, and records.
//!
//! The module owns the capital side of a backtest. It defines the configuration
//! and the sizing rule, the state the ledger maintains, and the records the
//! ledger publishes. It performs no accounting yet: cash, equity, and P&L are
//! updated by the ledger in a later work item. It reads no clock, performs no
//! I/O, and uses no randomness.
//!
//! The module depends on [`crate::strategy::Side`] for the trade direction. It
//! does not depend on [`crate::execution`]: execution supplies fills, and the
//! ledger turns those fills into capital.

mod config;
mod records;
mod sizing;
mod state;

pub use config::{
    CommissionBase, CommissionConfig, CommissionPer, Currency, DrawdownAction, FundingConfig,
    FundingInterval, MarkToMarket, MaxDrawdownConfig, PortfolioConfig, PortfolioConfigError,
    SUPPORTED_LEVERAGE, SUPPORTED_MAX_OPEN_POSITIONS,
};
pub use records::{EquityPoint, ExitReason, FundingCharge, RiskEvent, TradeRecord};
pub use sizing::{SizingMode, SizingRejection};
pub use state::{OpenPosition, OpenPositionError, PortfolioState, PortfolioStateError};
