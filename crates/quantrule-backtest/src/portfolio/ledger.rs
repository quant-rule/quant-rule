//! The portfolio ledger: cash, positions, trades, costs, and risk.
//!
//! The ledger consumes one strategy bar at a time, in order. It applies the
//! bar's fills, marks the open position to the bar close, and publishes the
//! records a report reads. It is pure: the caller supplies the bar, the fill,
//! and the clock values the fill carries; the ledger reads no file, clock, or
//! random source.
//!
//! The capital model is a cash account. Opening a long spends the notional;
//! opening a short receives it. The position is carried at its marked value, so
//! `equity = cash + position_value` at every point. Leverage above `1.0` is not
//! modelled and is rejected by the configuration.
//!
//! Commission is `rate * max(equity_before_operation, 0)` on every fill. The
//! execution fee is the fill's own fee and is never recomputed. Funding is a
//! separate charge at an aligned boundary; the ledger applies it before the
//! bar's fills.

use std::fmt;

use crate::execution::{Fill, FillReason, OrderSide};
use crate::strategy::Side;

use super::config::{DrawdownAction, PortfolioConfig, PortfolioConfigError};
use super::records::{
    EquityPoint, ExitReason, FundingCharge, MonthlyReturn, PortfolioMetrics, RiskEvent, TradeRecord,
};
use super::sizing::SizingRejection;
use super::state::{OpenPosition, OpenPositionError, PortfolioState, PortfolioStateError};

const SECONDS_PER_DAY: i64 = 86_400;

/// The accounting state machine of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioLedger {
    config: PortfolioConfig,
    state: PortfolioState,
    trades: Vec<TradeRecord>,
    equity_points: Vec<EquityPoint>,
    funding_charges: Vec<FundingCharge>,
    risk_events: Vec<RiskEvent>,
    execution_fees: f64,
    commission: f64,
    funding_paid: f64,
    funding_received: f64,
    gross_realized: f64,
    net_realized: f64,
    wins: usize,
    losses: usize,
    profit_from_wins: f64,
    loss_from_losses: f64,
    hold_bars_total: usize,
    max_drawdown: f64,
    current_drawdown_bars: usize,
    max_drawdown_duration: usize,
    mark_count: usize,
    sum_position_value: f64,
    sum_equity: f64,
    last_price: Option<f64>,
}

impl PortfolioLedger {
    /// Builds a ledger from a validated portfolio configuration.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioLedgerError::Config`] when the configuration is
    /// invalid, and [`PortfolioLedgerError::State`] when the initial cash
    /// cannot build the starting state.
    pub fn new(config: PortfolioConfig) -> Result<Self, PortfolioLedgerError> {
        config.validate().map_err(PortfolioLedgerError::Config)?;
        let state =
            PortfolioState::new(config.initial_cash).map_err(PortfolioLedgerError::State)?;

        Ok(Self {
            config,
            state,
            trades: Vec::new(),
            equity_points: Vec::new(),
            funding_charges: Vec::new(),
            risk_events: Vec::new(),
            execution_fees: 0.0,
            commission: 0.0,
            funding_paid: 0.0,
            funding_received: 0.0,
            gross_realized: 0.0,
            net_realized: 0.0,
            wins: 0,
            losses: 0,
            profit_from_wins: 0.0,
            loss_from_losses: 0.0,
            hold_bars_total: 0,
            max_drawdown: 0.0,
            current_drawdown_bars: 0,
            max_drawdown_duration: 0,
            mark_count: 0,
            sum_position_value: 0.0,
            sum_equity: 0.0,
            last_price: None,
        })
    }

    /// The capital state after the last fill, funding charge, or mark.
    #[must_use]
    pub const fn state(&self) -> PortfolioState {
        self.state
    }

    /// The closed trades, in close order.
    #[must_use]
    pub fn trades(&self) -> &[TradeRecord] {
        &self.trades
    }

    /// One point per UTC date with at least one mark, at the last mark close.
    #[must_use]
    pub fn equity_points(&self) -> &[EquityPoint] {
        &self.equity_points
    }

    /// The funding charges, in charge order.
    #[must_use]
    pub fn funding_charges(&self) -> &[FundingCharge] {
        &self.funding_charges
    }

    /// The drawdown-halt events; there is at most one per run.
    #[must_use]
    pub fn risk_events(&self) -> &[RiskEvent] {
        &self.risk_events
    }

    /// Sizes one entry at the current equity and the supplied price.
    ///
    /// # Errors
    ///
    /// Returns a [`SizingRejection`] when the ledger is halted or the sizing
    /// rule rejects the entry.
    pub fn size_entry(&self, side: Side, price: f64) -> Result<f64, SizingRejection> {
        if self.state.halted {
            return Err(SizingRejection::Halted);
        }
        self.config.size_entry(side, self.state.equity, price)
    }

    /// Applies one fill to cash and the open position.
    ///
    /// An entry opens a position; an exit, stop, or target closes it and
    /// publishes a [`TradeRecord`]. The commission uses the equity before the
    /// operation; the fill's own fee is added, never recomputed.
    ///
    /// # Errors
    ///
    /// Returns an error when the fill is malformed, an entry arrives while a
    /// position is open, an exit arrives while flat, or the exit side does not
    /// close the position.
    pub fn apply_fill(&mut self, fill: &Fill) -> Result<(), PortfolioLedgerError> {
        validate_fill(fill)?;
        match fill.reason {
            FillReason::Entry => self.open(fill),
            FillReason::Exit | FillReason::StopLoss | FillReason::TakeProfit => self.close(fill),
        }
    }

    /// Applies funding at an aligned UTC boundary, before the bar's fills.
    ///
    /// A positive rate means a long pays and a short receives. The charge uses
    /// the bar open as the mark price. A zero-volume bar and a non-aligned bar
    /// produce no charge and no record. A missing boundary bar is simply never
    /// passed here, so the boundary is skipped.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioLedgerError::NonFinitePrice`] when a charge is due
    /// and the price is not finite or positive.
    pub fn apply_funding(
        &mut self,
        bar: usize,
        time: i64,
        price: f64,
        volume: Option<f64>,
    ) -> Result<Option<FundingCharge>, PortfolioLedgerError> {
        if volume == Some(0.0) {
            return Ok(None);
        }
        let interval = self.config.funding.interval().seconds();
        if time.rem_euclid(interval) != 0 {
            return Ok(None);
        }
        let Some(mut position) = self.state.position else {
            return Ok(None);
        };
        if !price.is_finite() || price <= 0.0 {
            return Err(PortfolioLedgerError::NonFinitePrice);
        }

        let amount = self.config.funding.rate() * position.quantity.abs() * price;
        let signed = match position.side {
            Side::Long => amount,
            Side::Short => -amount,
        };
        self.state.cash -= signed;
        self.state.equity -= signed;
        if signed > 0.0 {
            self.funding_paid += signed;
            position.funding_paid += signed;
        } else if signed < 0.0 {
            self.funding_received += -signed;
            position.funding_received += -signed;
        }
        self.state.position = Some(position);

        let charge = FundingCharge {
            bar,
            time,
            rate: self.config.funding.rate(),
            price,
            quantity: position.quantity,
            amount: signed,
        };
        self.funding_charges.push(charge);
        Ok(Some(charge))
    }

    /// Marks the open position to the bar close and records the equity point.
    ///
    /// The peak equity, drawdown, maximum drawdown, and the longest underwater
    /// run are updated here. The halt latches once the configured limit is
    /// reached; it is recorded once and never clears.
    ///
    /// # Errors
    ///
    /// Returns [`PortfolioLedgerError::NonFinitePrice`] when the close is not
    /// finite or positive.
    pub fn mark(&mut self, bar: usize, time: i64, close: f64) -> Result<(), PortfolioLedgerError> {
        if !close.is_finite() || close <= 0.0 {
            return Err(PortfolioLedgerError::NonFinitePrice);
        }

        let position_value = self.position_value(close);
        let equity = self.state.cash + position_value;
        self.state.equity = equity;
        if equity > self.state.peak_equity {
            self.state.peak_equity = equity;
        }
        let drawdown = 1.0 - equity / self.state.peak_equity;
        self.state.drawdown = drawdown;
        if drawdown > self.max_drawdown {
            self.max_drawdown = drawdown;
        }
        if drawdown > 0.0 {
            self.current_drawdown_bars += 1;
            if self.current_drawdown_bars > self.max_drawdown_duration {
                self.max_drawdown_duration = self.current_drawdown_bars;
            }
        } else {
            self.current_drawdown_bars = 0;
        }

        self.check_halt(bar, time, equity, drawdown);

        self.sum_position_value += position_value;
        self.sum_equity += equity;
        self.mark_count += 1;
        self.record_equity_point(time, position_value, equity, drawdown);
        self.last_price = Some(close);
        Ok(())
    }

    /// The summary metrics of the run so far.
    #[must_use]
    pub fn metrics(&self) -> PortfolioMetrics {
        let trades = self.trades.len();
        let win_rate = (trades > 0).then(|| self.wins as f64 / trades as f64);
        let profit_factor = if trades == 0 || self.loss_from_losses == 0.0 {
            None
        } else {
            Some(self.profit_from_wins / -self.loss_from_losses)
        };
        let average_trade = (trades > 0).then(|| self.net_realized / trades as f64);
        let average_hold_bars = (trades > 0).then(|| self.hold_bars_total as f64 / trades as f64);
        let exposure = (self.mark_count > 0 && self.sum_equity != 0.0)
            .then(|| self.sum_position_value / self.sum_equity);

        let (unrealized, open_gross) = self.open_pnl();
        PortfolioMetrics {
            initial_cash: self.config.initial_cash,
            final_equity: self.state.equity,
            total_return: self.state.equity / self.config.initial_cash - 1.0,
            realized_pnl: self.net_realized,
            unrealized_pnl: unrealized,
            gross_pnl: self.gross_realized + open_gross,
            net_pnl: self.net_realized + unrealized,
            execution_fees: self.execution_fees,
            commission: self.commission,
            funding_paid: self.funding_paid,
            funding_received: self.funding_received,
            max_drawdown: self.max_drawdown,
            max_drawdown_duration_bars: self.max_drawdown_duration,
            trades,
            wins: self.wins,
            losses: self.losses,
            win_rate,
            profit_factor,
            average_trade,
            average_hold_bars,
            exposure,
            halted: self.state.halted,
            open_position: self.state.position,
            monthly_returns: self.monthly_returns(),
        }
    }

    fn open(&mut self, fill: &Fill) -> Result<(), PortfolioLedgerError> {
        if self.state.position.is_some() {
            return Err(PortfolioLedgerError::PositionAlreadyOpen);
        }

        let side = match fill.side {
            OrderSide::Buy => Side::Long,
            OrderSide::Sell => Side::Short,
        };
        let equity = self.state.equity;
        let commission = self.commission_on(equity);
        let notional = fill.price * fill.quantity;
        let moved = match fill.side {
            OrderSide::Buy => self.state.cash - notional,
            OrderSide::Sell => self.state.cash + notional,
        };
        let cash = moved - fill.fee - commission;

        let position = OpenPosition::new(
            side,
            fill.quantity,
            fill.price,
            fill.bar,
            fill.time,
            equity,
            fill.fee,
            commission,
        )
        .map_err(PortfolioLedgerError::Position)?;
        let position_value = match side {
            Side::Long => notional,
            Side::Short => -notional,
        };

        self.state.cash = cash;
        self.state.position = Some(position);
        self.state.equity = cash + position_value;
        self.execution_fees += fill.fee;
        self.commission += commission;
        self.last_price = Some(fill.price);
        Ok(())
    }

    fn close(&mut self, fill: &Fill) -> Result<(), PortfolioLedgerError> {
        let Some(position) = self.state.position else {
            return Err(PortfolioLedgerError::NoPosition);
        };
        let closing_side = match position.side {
            Side::Long => OrderSide::Sell,
            Side::Short => OrderSide::Buy,
        };
        if fill.side != closing_side {
            return Err(PortfolioLedgerError::WrongSide);
        }

        let equity = self.state.equity;
        let commission = self.commission_on(equity);
        let notional = fill.price * fill.quantity;
        let moved = match fill.side {
            OrderSide::Buy => self.state.cash - notional,
            OrderSide::Sell => self.state.cash + notional,
        };
        let cash = moved - fill.fee - commission;

        let gross = (fill.price - position.entry_price)
            * position.quantity
            * f64::from(position.side.code());
        let execution_fees = position.entry_fees + fill.fee;
        let total_commission = position.entry_commission + commission;
        let funding = position.funding_paid - position.funding_received;
        let net = gross - execution_fees - total_commission - funding;
        let hold_bars = fill.bar.saturating_sub(position.entry_bar);

        let trade = TradeRecord {
            trade_id: self.trades.len() as u64 + 1,
            side: position.side,
            quantity: position.quantity,
            entry_bar: position.entry_bar,
            entry_time: position.entry_time,
            entry_price: position.entry_price,
            exit_bar: fill.bar,
            exit_time: fill.time,
            exit_price: fill.price,
            gross_pnl: gross,
            execution_fees,
            commission: total_commission,
            funding,
            net_pnl: net,
            return_fraction: (position.entry_equity > 0.0).then(|| net / position.entry_equity),
            hold_bars,
            exit_reason: exit_reason(fill.reason)?,
        };

        self.trades.push(trade);
        self.gross_realized += gross;
        self.net_realized += net;
        if net > 0.0 {
            self.wins += 1;
            self.profit_from_wins += net;
        } else if net < 0.0 {
            self.losses += 1;
            self.loss_from_losses += net;
        }
        self.hold_bars_total += hold_bars;
        self.state.cash = cash;
        self.state.position = None;
        self.state.equity = cash;
        self.execution_fees += fill.fee;
        self.commission += commission;
        self.last_price = None;
        Ok(())
    }

    fn commission_on(&self, equity: f64) -> f64 {
        self.config.commission.rate() * equity.max(0.0)
    }

    fn position_value(&self, price: f64) -> f64 {
        self.state.position.map_or(0.0, |position| {
            position.quantity * price * f64::from(position.side.code())
        })
    }

    fn check_halt(&mut self, bar: usize, time: i64, equity: f64, drawdown: f64) {
        if self.state.halted || self.config.max_drawdown.action() != DrawdownAction::Halt {
            return;
        }
        let Some(limit) = self.config.max_drawdown.limit() else {
            return;
        };
        if drawdown < limit {
            return;
        }
        self.state.halted = true;
        self.risk_events.push(RiskEvent {
            bar,
            time,
            drawdown,
            equity,
            peak_equity: self.state.peak_equity,
            limit,
        });
    }

    fn record_equity_point(&mut self, time: i64, position_value: f64, equity: f64, drawdown: f64) {
        let date = time.div_euclid(SECONDS_PER_DAY) * SECONDS_PER_DAY;
        let point = EquityPoint {
            date,
            cash: self.state.cash,
            position_value,
            equity,
            peak_equity: self.state.peak_equity,
            drawdown,
        };
        if let Some(last) = self.equity_points.last_mut()
            && last.date == date
        {
            *last = point;
        } else {
            self.equity_points.push(point);
        }
    }

    /// The open position's net and gross unrealized P&L.
    fn open_pnl(&self) -> (f64, f64) {
        let Some(position) = self.state.position else {
            return (0.0, 0.0);
        };
        let Some(price) = self.last_price else {
            return (0.0, 0.0);
        };
        let gross =
            (price - position.entry_price) * position.quantity * f64::from(position.side.code());
        let funding = position.funding_paid - position.funding_received;
        let net = gross - position.entry_fees - position.entry_commission - funding;
        (net, gross)
    }

    fn monthly_returns(&self) -> Vec<MonthlyReturn> {
        let Some(first) = self.equity_points.first() else {
            return Vec::new();
        };
        let Some(last) = self.equity_points.last() else {
            return Vec::new();
        };

        let (mut year, mut month) = month_of(first.date);
        let (last_year, last_month) = month_of(last.date);
        let mut previous = Some(self.config.initial_cash);
        let mut index = 0;
        let mut returns = Vec::new();

        loop {
            let mut month_last = None;
            while let Some(point) = self.equity_points.get(index) {
                if month_of(point.date) != (year, month) {
                    break;
                }
                month_last = Some(point);
                index += 1;
            }

            let return_fraction = match (month_last, previous) {
                (Some(point), Some(prior)) if prior > 0.0 => Some(point.equity / prior - 1.0),
                _ => None,
            };
            returns.push(MonthlyReturn {
                year,
                month,
                return_fraction,
            });

            previous = month_last.map(|point| point.equity);
            if (year, month) == (last_year, last_month) {
                break;
            }
            (year, month) = next_month(year, month);
        }

        returns
    }
}

fn validate_fill(fill: &Fill) -> Result<(), PortfolioLedgerError> {
    if !fill.price.is_finite() || fill.price <= 0.0 {
        return Err(PortfolioLedgerError::InvalidFill);
    }
    if !fill.quantity.is_finite() || fill.quantity <= 0.0 {
        return Err(PortfolioLedgerError::InvalidFill);
    }
    if !fill.fee.is_finite() || fill.fee < 0.0 {
        return Err(PortfolioLedgerError::InvalidFill);
    }
    Ok(())
}

fn exit_reason(reason: FillReason) -> Result<ExitReason, PortfolioLedgerError> {
    match reason {
        FillReason::Exit => Ok(ExitReason::AgreementLost),
        FillReason::StopLoss => Ok(ExitReason::Stop),
        FillReason::TakeProfit => Ok(ExitReason::Target),
        FillReason::Entry => Err(PortfolioLedgerError::UnexpectedFillReason),
    }
}

/// The UTC month of a Unix-second timestamp.
pub(super) fn month_of(timestamp: i64) -> (i32, u8) {
    let (year, month, _) = civil_from_days(timestamp.div_euclid(SECONDS_PER_DAY));
    (year, month as u8)
}

/// The next calendar month.
pub(super) fn next_month(year: i32, month: u8) -> (i32, u8) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

/// Converts days since 1970-01-01 to a proleptic Gregorian date.
///
/// This is Howard Hinnant's civil-from-days algorithm. It is arithmetic only,
/// so the ledger needs no calendar dependency and no clock.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year as i32, month as u32, day as u32)
}

/// A ledger operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioLedgerError {
    /// The configuration is invalid.
    Config(PortfolioConfigError),
    /// The initial state could not be built.
    State(PortfolioStateError),
    /// The open position could not be built.
    Position(OpenPositionError),
    /// An entry arrived while a position was open.
    PositionAlreadyOpen,
    /// An exit arrived while no position was open.
    NoPosition,
    /// The exit side does not close the open position.
    WrongSide,
    /// A fill price, quantity, or fee is invalid.
    InvalidFill,
    /// A fill reason cannot close a position.
    UnexpectedFillReason,
    /// A price is not finite or not positive.
    NonFinitePrice,
}

impl fmt::Display for PortfolioLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(source) => source.fmt(formatter),
            Self::State(source) => source.fmt(formatter),
            Self::Position(source) => source.fmt(formatter),
            Self::PositionAlreadyOpen => formatter.write_str("a position is already open"),
            Self::NoPosition => formatter.write_str("no position is open"),
            Self::WrongSide => {
                formatter.write_str("the fill side does not close the open position")
            }
            Self::InvalidFill => formatter.write_str("the fill values are invalid"),
            Self::UnexpectedFillReason => {
                formatter.write_str("the fill reason cannot close a position")
            }
            Self::NonFinitePrice => formatter.write_str("the price must be finite and positive"),
        }
    }
}

impl std::error::Error for PortfolioLedgerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(source) => Some(source),
            Self::State(source) => Some(source),
            Self::Position(source) => Some(source),
            _ => None,
        }
    }
}
