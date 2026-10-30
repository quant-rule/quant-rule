//! The trade-intent boundary between strategy and execution.
//!
//! The strategy owns the thesis, the direction, and the risk intent. Execution
//! owns fill mechanics and resolves the risk specifications against the fill.
//! This module is the shared vocabulary: it does not read the model, the clock,
//! a source, or any account state.
//!
//! A [`TradeIntent`] is a request, not a fill. It carries no price, quantity,
//! fee, or slippage; those belong to execution. The strategy emits the intent
//! and execution either resolves it into a bracket or records a rejection.
//!
//! The strategy receives a [`StrategyPosition`], which is execution's report of
//! the position it actually holds. The strategy never mutates that position and
//! never infers one from its own decisions, so a rejected entry cannot become a
//! virtual position.

use crate::side::Side;

/// The default stop distance in ATR units.
pub const DEFAULT_STOP_ATR_MULTIPLE: f64 = 2.0;

/// The default target-to-stop reward ratio.
pub const DEFAULT_REWARD_TO_RISK: f64 = 1.5;

/// A complete strategy request for one entry.
///
/// The side and the risk specification belong to the strategy. Execution
/// resolves the stop and target against the entry fill and the decision bar's
/// ATR; it never chooses a distance of its own.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::intent::{EntryStyle, RiskPolicy, StopSpec, TargetSpec};
/// use quantrule_backtest::side::Side;
///
/// let intent = RiskPolicy::default().intent(Side::Long);
/// assert_eq!(intent.side, Side::Long);
/// assert_eq!(intent.entry, EntryStyle::MarketNextOpen);
/// assert_eq!(intent.stop, StopSpec::AtrMultiple { multiple: 2.0 });
/// assert_eq!(intent.target, TargetSpec::RewardToRisk { multiple: 1.5 });
/// assert_eq!(intent.valid_until, None);
/// assert_eq!(intent.max_slippage_bps, None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeIntent {
    /// The side to enter.
    pub side: Side,
    /// How the entry should be filled.
    pub entry: EntryStyle,
    /// The stop policy.
    pub stop: StopSpec,
    /// The target policy.
    pub target: TargetSpec,
    /// The last submission bar on which the intent is valid.
    ///
    /// The bar is the zero-based decision bar. An entry submitted on a later
    /// bar is rejected instead of being deferred.
    pub valid_until: Option<usize>,
    /// The maximum slippage the strategy accepts, in basis points.
    ///
    /// Execution rejects the entry when its configured slippage exceeds this
    /// value. `None` accepts execution's configured slippage.
    pub max_slippage_bps: Option<f64>,
}

/// How an entry should be filled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntryStyle {
    /// Market order at the next bar's open.
    MarketNextOpen,
    /// Limit order, reserved for a later execution policy.
    Limit {
        /// The limit price.
        price: f64,
        /// The number of bars the limit remains valid.
        valid_for_bars: u32,
    },
}

impl EntryStyle {
    /// Whether this is the market-next-open style execution supports.
    #[must_use]
    pub const fn is_market_next_open(self) -> bool {
        matches!(self, Self::MarketNextOpen)
    }
}

/// The stop policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StopSpec {
    /// A stop at a positive ATR multiple from the fill.
    AtrMultiple {
        /// The positive ATR multiple.
        multiple: f64,
    },
    /// An absolute stop price.
    Price {
        /// The stop price.
        price: f64,
    },
}

impl StopSpec {
    /// Whether the specification is finite and positive.
    #[must_use]
    pub fn is_valid(self) -> bool {
        match self {
            Self::AtrMultiple { multiple } => multiple.is_finite() && multiple > 0.0,
            Self::Price { price } => price.is_finite() && price > 0.0,
        }
    }
}

/// The target policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetSpec {
    /// A target at a multiple of the stop distance.
    RewardToRisk {
        /// The positive reward-to-risk multiple.
        multiple: f64,
    },
    /// An absolute target price.
    Price {
        /// The target price.
        price: f64,
    },
}

impl TargetSpec {
    /// Whether the specification is finite and positive.
    #[must_use]
    pub fn is_valid(self) -> bool {
        match self {
            Self::RewardToRisk { multiple } => multiple.is_finite() && multiple > 0.0,
            Self::Price { price } => price.is_finite() && price > 0.0,
        }
    }
}

/// The strategy's view of the position execution currently holds.
///
/// The strategy receives this value but never owns it. Execution and the
/// portfolio are the only owners of the actual position, so a rejected entry
/// leaves this value flat and a bracket exit updates it before the next
/// decision.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::intent::StrategyPosition;
/// use quantrule_backtest::side::Side;
///
/// let flat = StrategyPosition::flat();
/// assert!(flat.is_flat());
/// assert_eq!(flat.side, None);
///
/// let open = StrategyPosition::opened(Side::Long, 7);
/// assert!(open.is_open());
/// assert_eq!(open.side, Some(Side::Long));
/// assert_eq!(open.entry_bar, Some(7));
/// assert_eq!(open.bars_held, 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrategyPosition {
    /// The open side, or `None` when flat.
    pub side: Option<Side>,
    /// The entry fill bar, when a position is open.
    pub entry_bar: Option<usize>,
    /// The number of closed bars since entry, when a position is open.
    pub bars_held: u64,
}

impl Default for StrategyPosition {
    fn default() -> Self {
        Self::flat()
    }
}

impl StrategyPosition {
    /// The flat position.
    #[must_use]
    pub const fn flat() -> Self {
        Self {
            side: None,
            entry_bar: None,
            bars_held: 0,
        }
    }

    /// A position that filled on `entry_bar`.
    #[must_use]
    pub const fn opened(side: Side, entry_bar: usize) -> Self {
        Self {
            side: Some(side),
            entry_bar: Some(entry_bar),
            bars_held: 0,
        }
    }

    /// A position with only a side; used by diagnostic walks and tests.
    #[must_use]
    pub const fn with_side(side: Side) -> Self {
        Self {
            side: Some(side),
            entry_bar: None,
            bars_held: 0,
        }
    }

    /// Whether the position is flat.
    #[must_use]
    pub const fn is_flat(self) -> bool {
        self.side.is_none()
    }

    /// Whether the position is open.
    #[must_use]
    pub const fn is_open(self) -> bool {
        self.side.is_some()
    }
}

/// The human-owned risk policy attached to a strategy entry.
///
/// The policy is expressed as an ATR stop and a reward-to-risk target. The
/// strategy turns it into a [`TradeIntent`]; execution resolves the intent.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::intent::RiskPolicy;
///
/// let policy = RiskPolicy::new(2.0, 1.5)?;
/// assert_eq!(policy.stop_atr_multiple(), 2.0);
/// assert_eq!(policy.reward_to_risk(), 1.5);
/// # Ok::<(), quantrule_backtest::intent::RiskPolicyError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiskPolicy {
    stop_atr_multiple: f64,
    reward_to_risk: f64,
}

impl RiskPolicy {
    /// Builds a risk policy.
    ///
    /// # Errors
    ///
    /// Returns [`RiskPolicyError`] when either value is not finite or not
    /// positive.
    pub fn new(stop_atr_multiple: f64, reward_to_risk: f64) -> Result<Self, RiskPolicyError> {
        if !stop_atr_multiple.is_finite() || stop_atr_multiple <= 0.0 {
            return Err(RiskPolicyError::InvalidStop);
        }
        if !reward_to_risk.is_finite() || reward_to_risk <= 0.0 {
            return Err(RiskPolicyError::InvalidReward);
        }
        Ok(Self {
            stop_atr_multiple,
            reward_to_risk,
        })
    }

    /// The stop distance in ATR units.
    #[must_use]
    pub const fn stop_atr_multiple(self) -> f64 {
        self.stop_atr_multiple
    }

    /// The reward-to-risk multiple.
    #[must_use]
    pub const fn reward_to_risk(self) -> f64 {
        self.reward_to_risk
    }

    /// Builds the entry intent for a side.
    ///
    /// The entry is a market order at the next open, with an ATR stop and a
    /// reward-to-risk target. No intent expiry or slippage limit is attached;
    /// a rule that needs one sets it explicitly on the returned value.
    #[must_use]
    pub const fn intent(self, side: Side) -> TradeIntent {
        TradeIntent {
            side,
            entry: EntryStyle::MarketNextOpen,
            stop: StopSpec::AtrMultiple {
                multiple: self.stop_atr_multiple,
            },
            target: TargetSpec::RewardToRisk {
                multiple: self.reward_to_risk,
            },
            valid_until: None,
            max_slippage_bps: None,
        }
    }
}

impl Default for RiskPolicy {
    fn default() -> Self {
        Self {
            stop_atr_multiple: DEFAULT_STOP_ATR_MULTIPLE,
            reward_to_risk: DEFAULT_REWARD_TO_RISK,
        }
    }
}

/// A risk policy was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskPolicyError {
    /// The stop multiple is not finite or not positive.
    InvalidStop,
    /// The reward multiple is not finite or not positive.
    InvalidReward,
}

impl std::fmt::Display for RiskPolicyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStop => {
                formatter.write_str("the stop multiple must be finite and positive")
            }
            Self::InvalidReward => {
                formatter.write_str("the reward multiple must be finite and positive")
            }
        }
    }
}

impl std::error::Error for RiskPolicyError {}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_REWARD_TO_RISK, DEFAULT_STOP_ATR_MULTIPLE, EntryStyle, RiskPolicy, RiskPolicyError,
        StopSpec, StrategyPosition, TargetSpec, TradeIntent,
    };
    use crate::side::Side;

    #[test]
    fn the_default_policy_is_a_two_atr_stop_and_one_and_a_half_reward() {
        let policy = RiskPolicy::default();
        assert_eq!(policy.stop_atr_multiple(), DEFAULT_STOP_ATR_MULTIPLE);
        assert_eq!(policy.reward_to_risk(), DEFAULT_REWARD_TO_RISK);

        let intent = policy.intent(Side::Short);
        assert_eq!(intent.side, Side::Short);
        assert_eq!(intent.entry, EntryStyle::MarketNextOpen);
        assert_eq!(intent.stop, StopSpec::AtrMultiple { multiple: 2.0 });
        assert_eq!(intent.target, TargetSpec::RewardToRisk { multiple: 1.5 });
        assert_eq!(intent.valid_until, None);
        assert_eq!(intent.max_slippage_bps, None);
    }

    #[test]
    fn risk_values_must_be_finite_and_positive() {
        for stop in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                RiskPolicy::new(stop, 1.5),
                Err(RiskPolicyError::InvalidStop)
            );
        }
        for reward in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                RiskPolicy::new(2.0, reward),
                Err(RiskPolicyError::InvalidReward)
            );
        }
    }

    #[test]
    fn specifications_validate_their_own_values() {
        assert!(StopSpec::AtrMultiple { multiple: 2.0 }.is_valid());
        assert!(!StopSpec::AtrMultiple { multiple: 0.0 }.is_valid());
        assert!(!StopSpec::AtrMultiple { multiple: f64::NAN }.is_valid());
        assert!(StopSpec::Price { price: 95.0 }.is_valid());
        assert!(!StopSpec::Price { price: -1.0 }.is_valid());

        assert!(TargetSpec::RewardToRisk { multiple: 1.5 }.is_valid());
        assert!(!TargetSpec::RewardToRisk { multiple: 0.0 }.is_valid());
        assert!(
            !TargetSpec::RewardToRisk {
                multiple: f64::INFINITY
            }
            .is_valid()
        );
        assert!(TargetSpec::Price { price: 110.0 }.is_valid());
        assert!(!TargetSpec::Price { price: 0.0 }.is_valid());
    }

    #[test]
    fn strategy_positions_report_their_lifecycle() {
        let flat = StrategyPosition::flat();
        assert!(flat.is_flat());
        assert!(!flat.is_open());
        assert_eq!(flat.side, None);
        assert_eq!(flat.entry_bar, None);
        assert_eq!(flat.bars_held, 0);

        let opened = StrategyPosition::opened(Side::Long, 7);
        assert!(opened.is_open());
        assert!(!opened.is_flat());
        assert_eq!(opened.side, Some(Side::Long));
        assert_eq!(opened.entry_bar, Some(7));
        assert_eq!(opened.bars_held, 0);

        let with_side = StrategyPosition::with_side(Side::Short);
        assert_eq!(with_side.side, Some(Side::Short));
        assert_eq!(with_side.entry_bar, None);
    }

    #[test]
    fn a_trade_intent_can_carry_an_expiry_and_a_slippage_limit() {
        let intent = TradeIntent {
            side: Side::Long,
            entry: EntryStyle::Limit {
                price: 99.0,
                valid_for_bars: 3,
            },
            stop: StopSpec::Price { price: 95.0 },
            target: TargetSpec::Price { price: 110.0 },
            valid_until: Some(4),
            max_slippage_bps: Some(5.0),
        };

        assert_eq!(intent.valid_until, Some(4));
        assert_eq!(intent.max_slippage_bps, Some(5.0));
        assert!(!intent.entry.is_market_next_open());
    }
}
