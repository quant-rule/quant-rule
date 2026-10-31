//! The records a completed portfolio run publishes.
//!
//! A record is data, not a decision. It carries the numbers the ledger measured
//! and no model label, axis word, or strategy threshold. The shell renders it;
//! the report contract fixes the field names and the row order.

use super::state::OpenPosition;

/// Why a position was closed.
///
/// The portfolio's exit reason is the fill's reason, not the strategy's. A
/// strategy exit becomes [`Self::StrategyExit`]; a bracket exit becomes
/// [`Self::Stop`] or [`Self::Target`]. The strategy report carries the specific
/// context or structure reason.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::ExitReason;
///
/// assert_eq!(ExitReason::StrategyExit.code(), "strategy");
/// assert_eq!(ExitReason::Stop.code(), "stop");
/// assert_eq!(ExitReason::Target.code(), "target");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// The strategy requested the close.
    StrategyExit,
    /// The position's stop was reached.
    Stop,
    /// The position's target was reached.
    Target,
}

impl ExitReason {
    /// The contract's exit-reason code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::StrategyExit => "strategy",
            Self::Stop => "stop",
            Self::Target => "target",
        }
    }
}

/// One closed trade.
///
/// `gross_pnl` is the price move times quantity, before costs. `net_pnl` is
/// gross minus execution fees, commission, and funding. `return_fraction` is
/// `net_pnl / entry_equity`; it is `None` when the entry equity was zero.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::{ExitReason, TradeRecord};
/// use quantrule_backtest::side::Side;
///
/// let trade = TradeRecord {
///     trade_id: 1,
///     side: Side::Long,
///     quantity: 2.0,
///     entry_bar: 10,
///     entry_time: 1_700_000_000,
///     entry_price: 100.0,
///     exit_bar: 20,
///     exit_time: 1_700_000_900,
///     exit_price: 110.0,
///     gross_pnl: 20.0,
///     execution_fees: 1.0,
///     commission: 2.0,
///     funding: 0.5,
///     net_pnl: 16.5,
///     return_fraction: Some(0.00165),
///     hold_bars: 10,
///     exit_reason: ExitReason::Target,
/// };
/// assert_eq!(trade.net_pnl, 16.5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeRecord {
    /// The one-based trade identifier.
    pub trade_id: u64,
    /// The position's direction.
    pub side: crate::side::Side,
    /// The quantity traded.
    pub quantity: f64,
    /// The zero-based strategy bar of the entry fill.
    pub entry_bar: usize,
    /// The entry bar's open time as UTC Unix seconds.
    pub entry_time: i64,
    /// The entry fill price.
    pub entry_price: f64,
    /// The zero-based strategy bar of the exit fill.
    pub exit_bar: usize,
    /// The exit bar's open time as UTC Unix seconds.
    pub exit_time: i64,
    /// The exit fill price.
    pub exit_price: f64,
    /// The price move times quantity, before costs.
    pub gross_pnl: f64,
    /// The execution fees paid on both fills.
    pub execution_fees: f64,
    /// The commission paid on both fills.
    pub commission: f64,
    /// The funding paid minus funding received while the position was open.
    pub funding: f64,
    /// Gross P&L minus fees, commission, and funding.
    pub net_pnl: f64,
    /// `net_pnl / entry_equity`, or `None` when the entry equity was zero.
    pub return_fraction: Option<f64>,
    /// The number of strategy bars the position was held.
    pub hold_bars: usize,
    /// Why the position was closed.
    pub exit_reason: ExitReason,
}

/// The portfolio's capital at the last strategy bar of one UTC date.
///
/// `drawdown` is `1 - equity / peak_equity`. A date with no strategy bar has no
/// point; it is absent, never zero.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::EquityPoint;
///
/// let point = EquityPoint {
///     date: 1_735_689_600,
///     cash: 9_000.0,
///     position_value: 1_100.0,
///     equity: 10_100.0,
///     peak_equity: 10_200.0,
///     drawdown: 1.0 - 10_100.0 / 10_200.0,
/// };
/// assert_eq!(point.equity, 10_100.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquityPoint {
    /// The UTC date's start as Unix seconds.
    pub date: i64,
    /// The cash balance at the date's last bar close.
    pub cash: f64,
    /// The marked position value at the date's last bar close.
    pub position_value: f64,
    /// Cash plus the position value.
    pub equity: f64,
    /// The highest equity seen so far.
    pub peak_equity: f64,
    /// `1 - equity / peak_equity`.
    pub drawdown: f64,
}

/// One funding charge at an aligned UTC boundary.
///
/// `amount` is signed from the portfolio's point of view: a positive amount was
/// paid, a negative amount was received. A zero-volume or missing boundary bar
/// produces no charge and therefore no record.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::FundingCharge;
///
/// let charge = FundingCharge {
///     bar: 96,
///     time: 1_700_028_800,
///     rate: 0.0001,
///     price: 100.0,
///     quantity: 2.0,
///     amount: 0.02,
/// };
/// assert_eq!(charge.amount, 0.02);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FundingCharge {
    /// The zero-based strategy bar at the boundary.
    pub bar: usize,
    /// The bar's open time as UTC Unix seconds.
    pub time: i64,
    /// The funding rate for the interval.
    pub rate: f64,
    /// The mark price used for the charge: the bar open.
    pub price: f64,
    /// The open position's quantity.
    pub quantity: f64,
    /// The signed amount: positive paid, negative received.
    pub amount: f64,
}

/// A drawdown-halt risk event.
///
/// The event is recorded once, at the bar the drawdown first reached the
/// configured limit. The halt then latches for the rest of the run.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::RiskEvent;
///
/// let event = RiskEvent {
///     bar: 500,
///     time: 1_700_045_000,
///     drawdown: 0.2,
///     equity: 8_000.0,
///     peak_equity: 10_000.0,
///     limit: 0.2,
/// };
/// assert_eq!(event.limit, 0.2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskEvent {
    /// The zero-based strategy bar of the breach.
    pub bar: usize,
    /// The bar's open time as UTC Unix seconds.
    pub time: i64,
    /// The drawdown at the breach.
    pub drawdown: f64,
    /// The equity at the breach.
    pub equity: f64,
    /// The peak equity at the breach.
    pub peak_equity: f64,
    /// The configured limit.
    pub limit: f64,
}

/// One month's return in the actual period.
///
/// The month is numeric so the engine publishes no label; the shell renders
/// `YYYY-MM`. A month with no equity point, or whose prior equity is zero, has
/// no return and is never written as zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonthlyReturn {
    /// The UTC year.
    pub year: i32,
    /// The UTC month, `1..=12`.
    pub month: u8,
    /// The month's return, or `None` when it is undefined.
    pub return_fraction: Option<f64>,
}

/// The portfolio summary metrics of one run.
///
/// `realized_pnl` is the sum of closed-trade net P&L. `unrealized_pnl` is the
/// open position's gross mark-to-market P&L minus its entry costs and accrued
/// funding; it is zero when flat. `net_pnl` is their sum, and the ledger
/// invariant is `final_equity = initial_cash + net_pnl`.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioMetrics {
    /// The starting cash.
    pub initial_cash: f64,
    /// The equity at the last mark, fill, or the initial cash.
    pub final_equity: f64,
    /// `final_equity / initial_cash - 1`.
    pub total_return: f64,
    /// Closed-trade net P&L.
    pub realized_pnl: f64,
    /// Open-position net P&L, or zero when flat.
    pub unrealized_pnl: f64,
    /// Closed gross P&L plus the open position's gross P&L.
    pub gross_pnl: f64,
    /// `realized_pnl + unrealized_pnl`.
    pub net_pnl: f64,
    /// Execution fees paid on every fill.
    pub execution_fees: f64,
    /// Commission paid on every fill.
    pub commission: f64,
    /// Funding paid at aligned boundaries.
    pub funding_paid: f64,
    /// Funding received at aligned boundaries.
    pub funding_received: f64,
    /// The maximum drawdown observed at a mark.
    pub max_drawdown: f64,
    /// The longest run of marked bars with a positive drawdown.
    pub max_drawdown_duration_bars: usize,
    /// The number of closed trades.
    pub trades: usize,
    /// Closed trades with a positive net P&L.
    pub wins: usize,
    /// Closed trades with a negative net P&L.
    pub losses: usize,
    /// `wins / trades`, or `None` with no trades.
    pub win_rate: Option<f64>,
    /// Winning net P&L divided by the absolute losing net P&L, or `None` when
    /// there are no trades or no losses.
    pub profit_factor: Option<f64>,
    /// Mean closed-trade net P&L, or `None` with no trades.
    pub average_trade: Option<f64>,
    /// Mean closed-trade hold in strategy bars, or `None` with no trades.
    pub average_hold_bars: Option<f64>,
    /// `sum(position_value) / sum(equity)` over marks, or `None` when no mark
    /// exists or the equity sum is zero.
    pub exposure: Option<f64>,
    /// Whether the drawdown halt has latched.
    pub halted: bool,
    /// The open position at the last update, or `None` when flat.
    pub open_position: Option<OpenPosition>,
    /// One entry per month in the actual period.
    pub monthly_returns: Vec<MonthlyReturn>,
}
