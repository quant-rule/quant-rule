//! The deterministic reference position for signal mode.
//!
//! Signal mode has no execution and no portfolio. The strategy still needs a
//! position to decide from, so this module owns one fixed, deterministic
//! reference position: a market entry scheduled on bar `N` opens at bar `N + 1`
//! at that bar's open; a two-sided bracket is live on the next bar and the
//! first boundary reached chooses the side; an open position carries the pack's
//! ATR stop and reward target, evaluated from the entry bar onward with the
//! stop-first tie rule; an exit scheduled on bar `N` closes at bar `N + 1` at
//! that bar's open.
//!
//! The reference is deliberately not a fill simulator. It records no price,
//! quantity, fee, or P&L, and it never reads an execution or portfolio pack.
//! A reference entry or bracket requires a tradable, contiguous bar; an exit is
//! a state transition and is applied at the next open even when that bar cannot
//! trade. That difference keeps the signal series complete when the market is
//! closed while still refusing to invent an entry on a zero-volume or gapped
//! bar.
//!
//! The module is pure: it reads the published model bar and the strategy
//! snapshot and returns one [`SignalRow`]. It performs no I/O, reads no clock,
//! and uses no randomness.

use quantrule_backtest::intent::{
    BracketIntent, StopSpec, StrategyPosition, TargetSpec, TradeIntent,
};
use quantrule_backtest::side::Side;
use quantrule_backtest::strategy::{Decision, EventStrategyInput, ExitReason};
use quantrule_data::Candle;
use serde_json::Value;

use crate::config::StrategyConfig;
use crate::stages::strategy::{BarRow, decision_evidence};

use super::artifact::{SignalAction, SignalIntent, SignalReason, SignalRow, SignalSide};
use super::error::SignalError;

/// The open reference position.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ReferencePosition {
    side: Side,
    entry_position: usize,
    entry_price: f64,
    entry_atr: Option<f64>,
    stop_price: f64,
    target_price: f64,
}

/// An action scheduled by the previous bar.
#[derive(Debug, Clone)]
enum PendingAction {
    Enter {
        side: Side,
        intent: TradeIntent,
        atr: Option<f64>,
    },
    Exit,
    Bracket {
        intent: BracketIntent,
        atr: Option<f64>,
        evidence: Vec<Value>,
        remaining_bars: u32,
    },
}

/// A reference event detected on one bar.
#[derive(Debug, Default)]
struct BarEvents {
    entry_rejected: bool,
    bracket_rejected: bool,
    bracket_expired: bool,
    bracket_fill: Option<BracketFill>,
    reference_exit: Option<ReferenceExit>,
}

#[derive(Debug)]
struct BracketFill {
    side: Side,
    intent: BracketIntent,
    evidence: Vec<Value>,
}

#[derive(Debug)]
struct ReferenceExit {
    side: Side,
    reason: SignalReason,
}

enum BracketResolution {
    Fill { side: Side, price: f64 },
    Ambiguous,
    None,
}

/// The reference-position state machine for one signal-mode pack.
pub(crate) struct ReferenceMachine<'a> {
    config: &'a StrategyConfig,
    position: Option<ReferencePosition>,
    pending: Option<PendingAction>,
}

impl<'a> ReferenceMachine<'a> {
    /// Starts flat with no pending action.
    pub(crate) const fn new(config: &'a StrategyConfig) -> Self {
        Self {
            config,
            position: None,
            pending: None,
        }
    }

    /// Processes one published execution bar and returns its signal row.
    ///
    /// # Errors
    ///
    /// Returns [`SignalError::GivebackUnsupported`] if the engine produces the
    /// fold-only giveback exit. The signal pack parser already refuses a
    /// non-null giveback, so this is a defensive check rather than a reachable
    /// configuration.
    pub(crate) fn step(
        &mut self,
        position: usize,
        row: &BarRow,
        contiguous: bool,
        input: &EventStrategyInput,
    ) -> Result<SignalRow, SignalError> {
        let mut events = BarEvents::default();
        self.advance(position, &row.candle, contiguous, &mut events);
        let strategy_position = self.strategy_position(position, &row.candle);
        let decision = self.config.decide(input, strategy_position);
        let evidence = decision_evidence(self.config, input, decision);

        // A pending order from a previous bar is resolved before the decision.
        // The current pack schema limits a bracket to one bar, so this branch
        // is unreachable today; it keeps a future wider window deterministic
        // instead of silently replacing the live bracket.
        let decision = if self.pending.is_some() && !matches!(decision, Decision::Hold) {
            Decision::Hold
        } else {
            decision
        };

        let (action, side, intent, reason, row_evidence) = match decision {
            Decision::Enter(entry) => {
                self.pending = Some(PendingAction::Enter {
                    side: entry.side(),
                    intent: entry.intent,
                    atr: row.atr,
                });
                (
                    SignalAction::Enter,
                    Some(SignalSide::from_side(entry.side())),
                    Some(SignalIntent::Trade(entry.intent)),
                    Some(SignalReason::Entry),
                    evidence,
                )
            }
            Decision::Exit(exit) => {
                let reason = match exit.reason {
                    ExitReason::EventCondition => SignalReason::EventCondition,
                    ExitReason::TimeStop => SignalReason::TimeStop,
                    ExitReason::Giveback => return Err(SignalError::GivebackUnsupported),
                };
                self.pending = Some(PendingAction::Exit);
                (
                    SignalAction::Exit,
                    Some(SignalSide::from_side(exit.side)),
                    Some(SignalIntent::Exit),
                    Some(reason),
                    evidence,
                )
            }
            Decision::EnterBracket(entry) => {
                self.pending = Some(PendingAction::Bracket {
                    intent: entry.intent,
                    atr: row.atr,
                    evidence: evidence.clone(),
                    remaining_bars: entry.intent.valid_for_bars,
                });
                (
                    SignalAction::Bracket,
                    None,
                    Some(SignalIntent::Bracket(entry.intent)),
                    Some(SignalReason::BracketEntry),
                    evidence,
                )
            }
            Decision::Hold => hold_row(events),
        };

        Ok(SignalRow {
            bar_index: row.bar,
            open_time: row.timestamp,
            available_bar_index: row.bar,
            action,
            side,
            intent,
            reason,
            evidence: row_evidence,
        })
    }

    fn advance(
        &mut self,
        position: usize,
        candle: &Candle,
        contiguous: bool,
        events: &mut BarEvents,
    ) {
        if let Some(pending) = self.pending.take() {
            match pending {
                PendingAction::Enter { side, intent, atr } => {
                    if !contiguous || !tradable(candle) || !market_fill_allowed(&intent, position) {
                        events.entry_rejected = true;
                    } else if let Some((stop_price, target_price)) =
                        reference_levels(side, candle.open(), atr, intent.stop, intent.target)
                    {
                        self.position = Some(ReferencePosition {
                            side,
                            entry_position: position,
                            entry_price: candle.open(),
                            entry_atr: atr,
                            stop_price,
                            target_price,
                        });
                    } else {
                        events.entry_rejected = true;
                    }
                }
                PendingAction::Exit => {
                    // A reference exit is a target state, not a fill. It is
                    // applied even on an untradable bar so the signal is flat
                    // when the rule says flat.
                    self.position = None;
                }
                PendingAction::Bracket {
                    intent,
                    atr,
                    evidence,
                    remaining_bars,
                } => {
                    if !contiguous || !tradable(candle) {
                        events.bracket_rejected = true;
                    } else {
                        match bracket_trigger(&intent, candle) {
                            BracketResolution::Fill { side, price } => {
                                if let Some((stop_price, target_price)) =
                                    reference_levels(side, price, atr, intent.stop, intent.target)
                                {
                                    self.position = Some(ReferencePosition {
                                        side,
                                        entry_position: position,
                                        entry_price: price,
                                        entry_atr: atr,
                                        stop_price,
                                        target_price,
                                    });
                                    events.bracket_fill = Some(BracketFill {
                                        side,
                                        intent,
                                        evidence,
                                    });
                                } else {
                                    events.bracket_rejected = true;
                                }
                            }
                            BracketResolution::Ambiguous => {
                                // OHLC cannot order two intrabar triggers from
                                // an open between them; the fold rejects the
                                // bracket for the same reason.
                                events.bracket_rejected = true;
                            }
                            BracketResolution::None => {
                                if remaining_bars <= 1 {
                                    events.bracket_expired = true;
                                } else {
                                    self.pending = Some(PendingAction::Bracket {
                                        intent,
                                        atr,
                                        evidence,
                                        remaining_bars: remaining_bars - 1,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(open) = &self.position
            && tradable(candle)
            && let Some(reason) = bracket_hit(open, position, candle)
        {
            events.reference_exit = Some(ReferenceExit {
                side: open.side,
                reason,
            });
            self.position = None;
        }
    }

    fn strategy_position(&self, position: usize, candle: &Candle) -> StrategyPosition {
        let Some(open) = &self.position else {
            return StrategyPosition::flat();
        };
        let (unrealized_atr, mfe_atr, mae_atr) = excursions(open, candle);
        StrategyPosition {
            side: Some(open.side),
            entry_bar: Some(open.entry_position),
            bars_held: position.saturating_sub(open.entry_position) as u64,
            entry_price: Some(open.entry_price),
            entry_atr: open.entry_atr,
            unrealized_atr,
            mfe_atr,
            mae_atr,
        }
    }
}

fn hold_row(
    events: BarEvents,
) -> (
    SignalAction,
    Option<SignalSide>,
    Option<SignalIntent>,
    Option<SignalReason>,
    Vec<Value>,
) {
    if let Some(exit) = events.reference_exit {
        return (
            SignalAction::Exit,
            Some(SignalSide::from_side(exit.side)),
            Some(SignalIntent::Exit),
            Some(exit.reason),
            Vec::new(),
        );
    }
    if let Some(fill) = events.bracket_fill {
        return (
            SignalAction::Enter,
            Some(SignalSide::from_side(fill.side)),
            Some(SignalIntent::Bracket(fill.intent)),
            Some(SignalReason::BracketFill),
            fill.evidence,
        );
    }
    if events.bracket_rejected {
        return (
            SignalAction::Exit,
            None,
            Some(SignalIntent::Exit),
            Some(SignalReason::BracketRejected),
            Vec::new(),
        );
    }
    if events.bracket_expired {
        return (
            SignalAction::Exit,
            None,
            Some(SignalIntent::Exit),
            Some(SignalReason::BracketExpired),
            Vec::new(),
        );
    }
    if events.entry_rejected {
        return (
            SignalAction::Exit,
            None,
            Some(SignalIntent::Exit),
            Some(SignalReason::EntryRejected),
            Vec::new(),
        );
    }
    (SignalAction::Hold, None, None, None, Vec::new())
}

fn market_fill_allowed(intent: &TradeIntent, position: usize) -> bool {
    intent.entry.is_market_next_open()
        && intent
            .valid_until
            .is_none_or(|valid_until| position <= valid_until)
}

/// Resolves the ATR stop and reward target the pack's risk policy produces.
///
/// Only the ATR-multiple stop and the reward-to-risk target are supported in
/// signal mode. A price stop or price target has no signal-mode caller; the
/// reference rejects it rather than inventing a fill the signal cannot express.
fn reference_levels(
    side: Side,
    entry: f64,
    atr: Option<f64>,
    stop: StopSpec,
    target: TargetSpec,
) -> Option<(f64, f64)> {
    let (StopSpec::AtrMultiple { multiple }, TargetSpec::RewardToRisk { multiple: reward }) =
        (stop, target)
    else {
        return None;
    };
    if !entry.is_finite() || entry <= 0.0 {
        return None;
    }
    let atr = atr.filter(|value| value.is_finite() && *value > 0.0)?;
    if !multiple.is_finite() || multiple <= 0.0 || !reward.is_finite() || reward <= 0.0 {
        return None;
    }
    let stop_price = match side {
        Side::Long => entry - multiple * atr,
        Side::Short => entry + multiple * atr,
    };
    let stop_distance = (entry - stop_price).abs();
    let target_price = match side {
        Side::Long => entry + stop_distance * reward,
        Side::Short => entry - stop_distance * reward,
    };
    if !stop_price.is_finite()
        || stop_price <= 0.0
        || !target_price.is_finite()
        || target_price <= 0.0
    {
        return None;
    }
    let valid = match side {
        Side::Long => stop_price < entry && entry < target_price,
        Side::Short => target_price < entry && entry < stop_price,
    };
    valid.then_some((stop_price, target_price))
}

fn bracket_trigger(intent: &BracketIntent, candle: &Candle) -> BracketResolution {
    let upper = intent.upper_price;
    let lower = intent.lower_price;
    let open = candle.open();
    if open >= upper {
        return BracketResolution::Fill {
            side: Side::Long,
            price: open,
        };
    }
    if open <= lower {
        return BracketResolution::Fill {
            side: Side::Short,
            price: open,
        };
    }
    let long_hit = candle.high() >= upper;
    let short_hit = candle.low() <= lower;
    match (long_hit, short_hit) {
        (true, true) => BracketResolution::Ambiguous,
        (true, false) => BracketResolution::Fill {
            side: Side::Long,
            price: upper,
        },
        (false, true) => BracketResolution::Fill {
            side: Side::Short,
            price: lower,
        },
        (false, false) => BracketResolution::None,
    }
}

fn bracket_hit(
    position: &ReferencePosition,
    bar_position: usize,
    candle: &Candle,
) -> Option<SignalReason> {
    let entry_bar = bar_position == position.entry_position;
    match position.side {
        Side::Long => {
            if entry_bar {
                if candle.low() <= position.stop_price {
                    return Some(SignalReason::StopLoss);
                }
                if candle.high() >= position.target_price {
                    return Some(SignalReason::TakeProfit);
                }
                return None;
            }
            if candle.open() <= position.stop_price {
                return Some(SignalReason::StopLoss);
            }
            if candle.open() >= position.target_price {
                return Some(SignalReason::TakeProfit);
            }
            if candle.low() <= position.stop_price {
                return Some(SignalReason::StopLoss);
            }
            if candle.high() >= position.target_price {
                return Some(SignalReason::TakeProfit);
            }
            None
        }
        Side::Short => {
            if entry_bar {
                if candle.high() >= position.stop_price {
                    return Some(SignalReason::StopLoss);
                }
                if candle.low() <= position.target_price {
                    return Some(SignalReason::TakeProfit);
                }
                return None;
            }
            if candle.open() >= position.stop_price {
                return Some(SignalReason::StopLoss);
            }
            if candle.open() <= position.target_price {
                return Some(SignalReason::TakeProfit);
            }
            if candle.high() >= position.stop_price {
                return Some(SignalReason::StopLoss);
            }
            if candle.low() <= position.target_price {
                return Some(SignalReason::TakeProfit);
            }
            None
        }
    }
}

fn excursions(
    position: &ReferencePosition,
    candle: &Candle,
) -> (Option<f64>, Option<f64>, Option<f64>) {
    let Some(atr) = position
        .entry_atr
        .filter(|value| value.is_finite() && *value > 0.0)
    else {
        return (None, Some(0.0), Some(0.0));
    };
    let (favorable, adverse) = match position.side {
        Side::Long => (
            (candle.high() - position.entry_price) / atr,
            (position.entry_price - candle.low()) / atr,
        ),
        Side::Short => (
            (position.entry_price - candle.low()) / atr,
            (candle.high() - position.entry_price) / atr,
        ),
    };
    let unrealized =
        (candle.close() - position.entry_price) / atr * f64::from(position.side.code());
    (
        Some(unrealized),
        Some(favorable.max(0.0)),
        Some(adverse.max(0.0)),
    )
}

fn tradable(candle: &Candle) -> bool {
    // An absent volume is not zero: the source did not supply it, so the bar is
    // treated as tradable rather than as an empty one.
    candle.volume() != Some(0.0)
}
