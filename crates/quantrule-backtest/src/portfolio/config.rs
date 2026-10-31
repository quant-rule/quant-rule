//! Portfolio configuration and its validation.
//!
//! The configuration fixes capital, sizing, costs, and risk limits once, so a
//! run cannot read a value from the environment or change it between bars.
//! Every field is validated before the ledger is built; a configuration that
//! fails validation is never used.
//!
//! The JSON schema is `quantrule.portfolio/v1`, documented in
//! `docs/config/2026-10-30/policy-and-strategy.md`. The shell parses the JSON and
//! builds these values; the engine performs no I/O.

use std::fmt;

use super::sizing::SizingMode;

/// The only leverage accepted in v1.
pub const SUPPORTED_LEVERAGE: f64 = 1.0;

/// The only open-position limit accepted in v1.
pub const SUPPORTED_MAX_OPEN_POSITIONS: u32 = 1;

/// The quote currency of the portfolio.
///
/// Only USD is supported in v1. The enum keeps an unknown currency from
/// reaching the ledger as a string.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::Currency;
///
/// assert_eq!(Currency::parse("USD")?, Currency::Usd);
/// assert!(Currency::parse("EUR").is_err());
/// assert_eq!(Currency::Usd.code(), "USD");
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    /// United States dollar.
    Usd,
}

impl Currency {
    /// Parses the contract's currency code.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::UnsupportedCurrency`] for any code other
    /// than `USD`.
    pub fn parse(code: &str) -> Result<Self, PortfolioConfigError> {
        match code {
            "USD" => Ok(Self::Usd),
            _ => Err(PortfolioConfigError::UnsupportedCurrency),
        }
    }

    /// The contract's currency code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Usd => "USD",
        }
    }
}

/// The base the commission is charged on.
///
/// The product's commission is a percentage of total portfolio equity, so
/// `equity` is the only accepted base in v1.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::CommissionBase;
///
/// assert_eq!(CommissionBase::parse("equity")?, CommissionBase::Equity);
/// assert!(CommissionBase::parse("notional").is_err());
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommissionBase {
    /// The commission is a fraction of total portfolio equity before the
    /// operation.
    Equity,
}

impl CommissionBase {
    /// Parses the contract's commission base.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::UnsupportedCommissionBase`] for any
    /// value other than `equity`.
    pub fn parse(code: &str) -> Result<Self, PortfolioConfigError> {
        match code {
            "equity" => Ok(Self::Equity),
            _ => Err(PortfolioConfigError::UnsupportedCommissionBase),
        }
    }

    /// The contract's commission base code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Equity => "equity",
        }
    }
}

/// How often the commission is charged.
///
/// The product charges per operation, so `operation` is the only accepted
/// value in v1.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::CommissionPer;
///
/// assert_eq!(CommissionPer::parse("operation")?, CommissionPer::Operation);
/// assert!(CommissionPer::parse("bar").is_err());
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommissionPer {
    /// The commission is charged on every fill.
    Operation,
}

impl CommissionPer {
    /// Parses the contract's commission period.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::UnsupportedCommissionPer`] for any value
    /// other than `operation`.
    pub fn parse(code: &str) -> Result<Self, PortfolioConfigError> {
        match code {
            "operation" => Ok(Self::Operation),
            _ => Err(PortfolioConfigError::UnsupportedCommissionPer),
        }
    }

    /// The contract's commission period code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Operation => "operation",
        }
    }
}

/// The commission charged on every fill.
///
/// The rate is a fraction of total portfolio equity before the operation. The
/// execution fee is separate and is recorded on the fill.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::{CommissionBase, CommissionConfig, CommissionPer};
///
/// let commission = CommissionConfig::new(
///     0.001,
///     CommissionBase::Equity,
///     CommissionPer::Operation,
/// )?;
/// assert_eq!(commission.rate(), 0.001);
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommissionConfig {
    rate: f64,
    base: CommissionBase,
    per: CommissionPer,
}

impl CommissionConfig {
    /// Builds a commission configuration.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::NonFiniteCommissionRate`] when the rate
    /// is not finite, and [`PortfolioConfigError::NegativeCommissionRate`] when
    /// it is negative.
    pub fn new(
        rate: f64,
        base: CommissionBase,
        per: CommissionPer,
    ) -> Result<Self, PortfolioConfigError> {
        if !rate.is_finite() {
            return Err(PortfolioConfigError::NonFiniteCommissionRate);
        }
        if rate < 0.0 {
            return Err(PortfolioConfigError::NegativeCommissionRate);
        }

        Ok(Self { rate, base, per })
    }

    /// The commission rate as a fraction of equity.
    #[must_use]
    pub const fn rate(&self) -> f64 {
        self.rate
    }

    /// The commission base.
    #[must_use]
    pub const fn base(&self) -> CommissionBase {
        self.base
    }

    /// The commission period.
    #[must_use]
    pub const fn per(&self) -> CommissionPer {
        self.per
    }
}

/// The funding interval at an aligned UTC boundary.
///
/// Only the intervals the contract lists are accepted. The interval is a
/// property of the instrument's funding schedule, not a free-form duration.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::FundingInterval;
///
/// assert_eq!(FundingInterval::new(8)?.hours(), 8);
/// assert!(FundingInterval::new(3).is_err());
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FundingInterval {
    /// Every hour.
    H1,
    /// Every two hours.
    H2,
    /// Every four hours.
    H4,
    /// Every eight hours.
    H8,
    /// Every twelve hours.
    H12,
    /// Every twenty-four hours.
    H24,
}

impl FundingInterval {
    /// Builds a funding interval from its length in hours.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::UnsupportedFundingInterval`] for any
    /// length outside `1, 2, 4, 8, 12, 24`.
    pub fn new(hours: u32) -> Result<Self, PortfolioConfigError> {
        match hours {
            1 => Ok(Self::H1),
            2 => Ok(Self::H2),
            4 => Ok(Self::H4),
            8 => Ok(Self::H8),
            12 => Ok(Self::H12),
            24 => Ok(Self::H24),
            _ => Err(PortfolioConfigError::UnsupportedFundingInterval),
        }
    }

    /// The interval length in hours.
    #[must_use]
    pub const fn hours(self) -> u32 {
        match self {
            Self::H1 => 1,
            Self::H2 => 2,
            Self::H4 => 4,
            Self::H8 => 8,
            Self::H12 => 12,
            Self::H24 => 24,
        }
    }

    /// The interval length in seconds.
    #[must_use]
    pub const fn seconds(self) -> i64 {
        self.hours() as i64 * 3_600
    }
}

/// The periodic funding charge on an open position.
///
/// A positive rate means a long pays and a short receives; a negative rate
/// reverses the sign. The rate is a fraction of the position notional per
/// interval.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::{FundingConfig, FundingInterval};
///
/// let funding = FundingConfig::new(0.0001, FundingInterval::H8)?;
/// assert_eq!(funding.rate(), 0.0001);
/// assert_eq!(funding.interval().hours(), 8);
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FundingConfig {
    rate: f64,
    interval: FundingInterval,
}

impl FundingConfig {
    /// Builds a funding configuration.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::NonFiniteFundingRate`] when the rate is
    /// not finite. A negative rate is accepted because it reverses the payer.
    pub fn new(rate: f64, interval: FundingInterval) -> Result<Self, PortfolioConfigError> {
        if !rate.is_finite() {
            return Err(PortfolioConfigError::NonFiniteFundingRate);
        }

        Ok(Self { rate, interval })
    }

    /// The funding rate per interval as a fraction of notional.
    #[must_use]
    pub const fn rate(&self) -> f64 {
        self.rate
    }

    /// The funding interval.
    #[must_use]
    pub const fn interval(&self) -> FundingInterval {
        self.interval
    }
}

/// What happens when the drawdown limit is reached.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::DrawdownAction;
///
/// assert_eq!(DrawdownAction::parse("halt")?, DrawdownAction::Halt);
/// assert_eq!(DrawdownAction::parse("none")?, DrawdownAction::None);
/// assert!(DrawdownAction::parse("liquidate").is_err());
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawdownAction {
    /// Latch a halt: no new entries, the existing position continues.
    Halt,
    /// Measure the drawdown without halting.
    None,
}

impl DrawdownAction {
    /// Parses the contract's drawdown action.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::UnsupportedDrawdownAction`] for any value
    /// other than `halt` or `none`.
    pub fn parse(code: &str) -> Result<Self, PortfolioConfigError> {
        match code {
            "halt" => Ok(Self::Halt),
            "none" => Ok(Self::None),
            _ => Err(PortfolioConfigError::UnsupportedDrawdownAction),
        }
    }

    /// The contract's drawdown action code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Halt => "halt",
            Self::None => "none",
        }
    }
}

/// The maximum-drawdown limit and its action.
///
/// A `None` limit means no limit is measured; the action must then be
/// [`DrawdownAction::None`]. A limit is a fraction in `(0, 1]`.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::{DrawdownAction, MaxDrawdownConfig};
///
/// let limit = MaxDrawdownConfig::new(Some(0.2), DrawdownAction::Halt)?;
/// assert_eq!(limit.limit(), Some(0.2));
/// assert!(MaxDrawdownConfig::new(None, DrawdownAction::Halt).is_err());
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaxDrawdownConfig {
    limit: Option<f64>,
    action: DrawdownAction,
}

impl MaxDrawdownConfig {
    /// Builds a drawdown configuration.
    ///
    /// # Errors
    ///
    /// Returns an error when the limit is not finite, is outside `(0, 1]`, or
    /// is absent while the action is `halt`.
    pub fn new(limit: Option<f64>, action: DrawdownAction) -> Result<Self, PortfolioConfigError> {
        if let Some(limit) = limit {
            if !limit.is_finite() {
                return Err(PortfolioConfigError::NonFiniteDrawdownLimit);
            }
            if limit <= 0.0 || limit > 1.0 {
                return Err(PortfolioConfigError::DrawdownLimitOutOfRange);
            }
        } else if matches!(action, DrawdownAction::Halt) {
            return Err(PortfolioConfigError::DrawdownWithoutLimit);
        }

        Ok(Self { limit, action })
    }

    /// The drawdown limit, or `None` when no limit is measured.
    #[must_use]
    pub const fn limit(&self) -> Option<f64> {
        self.limit
    }

    /// The action taken when the limit is reached.
    #[must_use]
    pub const fn action(&self) -> DrawdownAction {
        self.action
    }
}

/// The price used to mark an open position to market.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::MarkToMarket;
///
/// assert_eq!(MarkToMarket::parse("bar_close")?, MarkToMarket::BarClose);
/// assert!(MarkToMarket::parse("bar_open").is_err());
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkToMarket {
    /// Mark at each strategy bar's close.
    BarClose,
}

impl MarkToMarket {
    /// Parses the contract's mark-to-market rule.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioConfigError::UnsupportedMarkToMarket`] for any value
    /// other than `bar_close`.
    pub fn parse(code: &str) -> Result<Self, PortfolioConfigError> {
        match code {
            "bar_close" => Ok(Self::BarClose),
            _ => Err(PortfolioConfigError::UnsupportedMarkToMarket),
        }
    }

    /// The contract's mark-to-market code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BarClose => "bar_close",
        }
    }
}

/// The validated configuration of the portfolio engine.
///
/// The nested cost and risk values are validated when they are built. The
/// top-level numbers and the sizing mode are validated by [`Self::validate`],
/// which the ledger calls before it accepts the first fill.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::{
///     CommissionBase, CommissionConfig, CommissionPer, Currency, DrawdownAction,
///     FundingConfig, FundingInterval, MarkToMarket, MaxDrawdownConfig,
///     PortfolioConfig, SizingMode,
/// };
///
/// let config = PortfolioConfig {
///     initial_cash: 10_000.0,
///     currency: Currency::Usd,
///     sizing: SizingMode::FixedNotional {
///         notional: 10_000.0,
///         quantity_step: 0.0001,
///     },
///     leverage: 1.0,
///     max_open_positions: 1,
///     allow_short: true,
///     commission: CommissionConfig::new(
///         0.001,
///         CommissionBase::Equity,
///         CommissionPer::Operation,
///     )?,
///     funding: FundingConfig::new(0.0001, FundingInterval::H8)?,
///     max_drawdown: MaxDrawdownConfig::new(Some(0.2), DrawdownAction::Halt)?,
///     mark_to_market: MarkToMarket::BarClose,
/// };
/// config.validate()?;
/// assert_eq!(config.initial_cash, 10_000.0);
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortfolioConfig {
    /// The starting cash in the quote currency.
    pub initial_cash: f64,
    /// The quote currency.
    pub currency: Currency,
    /// The position-sizing rule.
    pub sizing: SizingMode,
    /// The declared leverage; exactly `1.0` in v1.
    pub leverage: f64,
    /// The maximum number of open positions; exactly `1` in v1.
    pub max_open_positions: u32,
    /// Whether a short entry is allowed.
    pub allow_short: bool,
    /// The commission charged on every fill.
    pub commission: CommissionConfig,
    /// The periodic funding charge.
    pub funding: FundingConfig,
    /// The maximum-drawdown limit and its action.
    pub max_drawdown: MaxDrawdownConfig,
    /// The price used to mark an open position to market.
    pub mark_to_market: MarkToMarket,
}

impl PortfolioConfig {
    /// Validates the top-level fields and the sizing mode.
    ///
    /// # Errors
    ///
    /// Returns an error when the initial cash is not finite or not positive,
    /// the sizing mode is invalid, the leverage is not exactly `1.0`, or the
    /// position limit is not exactly `1`.
    pub fn validate(&self) -> Result<(), PortfolioConfigError> {
        if !self.initial_cash.is_finite() {
            return Err(PortfolioConfigError::NonFiniteInitialCash);
        }
        if self.initial_cash <= 0.0 {
            return Err(PortfolioConfigError::NonPositiveInitialCash);
        }
        self.sizing.validate()?;
        if !self.leverage.is_finite() {
            return Err(PortfolioConfigError::NonFiniteLeverage);
        }
        if self.leverage != SUPPORTED_LEVERAGE {
            return Err(PortfolioConfigError::UnsupportedLeverage);
        }
        if self.max_open_positions != SUPPORTED_MAX_OPEN_POSITIONS {
            return Err(PortfolioConfigError::UnsupportedPositionLimit);
        }

        Ok(())
    }

    /// Sizes one entry from the decision-bar equity and close.
    ///
    /// This delegates to [`SizingMode::size`]. The quantity is floored to the
    /// configured step; a quantity below one step is a rejection, never a zero.
    ///
    /// # Errors
    ///
    /// Returns a [`super::sizing::SizingRejection`] when the equity or price is
    /// invalid, a short is not allowed, or the result is below one quantity
    /// step.
    pub fn size_entry(
        &self,
        side: crate::strategy::Side,
        equity: f64,
        price: f64,
    ) -> Result<f64, super::sizing::SizingRejection> {
        self.sizing.size(side, equity, price, self.allow_short)
    }
}

/// A portfolio configuration was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioConfigError {
    /// The initial cash is not a finite number.
    NonFiniteInitialCash,
    /// The initial cash is zero or negative.
    NonPositiveInitialCash,
    /// The currency is not supported in v1.
    UnsupportedCurrency,
    /// The fixed notional is not a finite number.
    NonFiniteNotional,
    /// The fixed notional is zero or negative.
    NonPositiveNotional,
    /// The equity fraction is not a finite number.
    NonFiniteFraction,
    /// The equity fraction is outside `(0, 1]`.
    FractionOutOfRange,
    /// The quantity step is not a finite number.
    NonFiniteQuantityStep,
    /// The quantity step is zero or negative.
    NonPositiveQuantityStep,
    /// The leverage is not a finite number.
    NonFiniteLeverage,
    /// The leverage is not exactly `1.0`.
    UnsupportedLeverage,
    /// The open-position limit is not exactly `1`.
    UnsupportedPositionLimit,
    /// The commission rate is not a finite number.
    NonFiniteCommissionRate,
    /// The commission rate is negative.
    NegativeCommissionRate,
    /// The commission base is not `equity`.
    UnsupportedCommissionBase,
    /// The commission period is not `operation`.
    UnsupportedCommissionPer,
    /// The funding rate is not a finite number.
    NonFiniteFundingRate,
    /// The funding interval is not one of `1, 2, 4, 8, 12, 24` hours.
    UnsupportedFundingInterval,
    /// The drawdown limit is not a finite number.
    NonFiniteDrawdownLimit,
    /// The drawdown limit is outside `(0, 1]`.
    DrawdownLimitOutOfRange,
    /// The drawdown action is `halt` while no limit is set.
    DrawdownWithoutLimit,
    /// The drawdown action is not `halt` or `none`.
    UnsupportedDrawdownAction,
    /// The mark-to-market rule is not `bar_close`.
    UnsupportedMarkToMarket,
}

impl fmt::Display for PortfolioConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInitialCash => write!(formatter, "the initial cash must be finite"),
            Self::NonPositiveInitialCash => write!(formatter, "the initial cash must be positive"),
            Self::UnsupportedCurrency => write!(formatter, "the currency must be USD"),
            Self::NonFiniteNotional => write!(formatter, "the fixed notional must be finite"),
            Self::NonPositiveNotional => write!(formatter, "the fixed notional must be positive"),
            Self::NonFiniteFraction => write!(formatter, "the equity fraction must be finite"),
            Self::FractionOutOfRange => {
                write!(formatter, "the equity fraction must be in (0, 1]")
            }
            Self::NonFiniteQuantityStep => {
                write!(formatter, "the quantity step must be finite")
            }
            Self::NonPositiveQuantityStep => {
                write!(formatter, "the quantity step must be positive")
            }
            Self::NonFiniteLeverage => write!(formatter, "the leverage must be finite"),
            Self::UnsupportedLeverage => write!(formatter, "the leverage must be exactly 1.0"),
            Self::UnsupportedPositionLimit => {
                write!(formatter, "the maximum open positions must be exactly 1")
            }
            Self::NonFiniteCommissionRate => {
                write!(formatter, "the commission rate must be finite")
            }
            Self::NegativeCommissionRate => {
                write!(formatter, "the commission rate must not be negative")
            }
            Self::UnsupportedCommissionBase => {
                write!(formatter, "the commission base must be equity")
            }
            Self::UnsupportedCommissionPer => {
                write!(formatter, "the commission period must be operation")
            }
            Self::NonFiniteFundingRate => write!(formatter, "the funding rate must be finite"),
            Self::UnsupportedFundingInterval => {
                write!(
                    formatter,
                    "the funding interval must be one of 1, 2, 4, 8, 12, 24 hours"
                )
            }
            Self::NonFiniteDrawdownLimit => {
                write!(formatter, "the drawdown limit must be finite")
            }
            Self::DrawdownLimitOutOfRange => {
                write!(formatter, "the drawdown limit must be in (0, 1]")
            }
            Self::DrawdownWithoutLimit => {
                write!(formatter, "a halt action requires a drawdown limit")
            }
            Self::UnsupportedDrawdownAction => {
                write!(formatter, "the drawdown action must be halt or none")
            }
            Self::UnsupportedMarkToMarket => {
                write!(formatter, "the mark-to-market rule must be bar_close")
            }
        }
    }
}

impl std::error::Error for PortfolioConfigError {}
