//! The first strategy: agreement of Direction, Momentum, and Structure.
//!
//! The rule enters in the direction all three readings agree on and exits when
//! that agreement no longer holds. It is causal and stateless: it reads the bar
//! it is given and the position execution passes in, and it never looks ahead.
//!
//! Two boundary cases are fixed here:
//!
//! - A bar whose timeframe is not the configured strategy timeframe is ignored
//!   and produces [`Decision::Hold`], so a misconfigured runner cannot trade a
//!   series the strategy was not defined for.
//! - An unavailable reading (`None`) while a position is open is not evidence
//!   that the agreement was lost, so the strategy holds. It exits only on a
//!   measured [`Agreement::Neutral`] or on the opposite agreement. This follows
//!   the model rule that a missing measurement is not a neutral one.
//!
//! The strategy never reverses on the bar that closes a position: the exit is
//! that bar's decision, and a new entry can occur only on a later bar.
//!
//! The rule's entry carries a [`TradeIntent`] built from its [`RiskPolicy`].
//! Execution resolves the stop and target; the strategy never chooses a fill
//! price or a bracket distance in price terms.

use crate::intent::{RiskPolicy, StrategyPosition};
use crate::side::Side;

use super::config::StrategyConfig;

use super::decision::{Decision, Entry, Exit, ExitReason, Strategy};
use super::input::{Agreement, StrategyInput};

/// The axis-agreement strategy.
///
/// The strategy is stateless with respect to position. The caller passes the
/// position execution currently holds; the rule enters only from flat and exits
/// only from an open position, so it never pyramids and never holds two
/// positions.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::intent::StrategyPosition;
/// use quantrule_backtest::strategy::{
///     AxisAgreementStrategy, Decision, Strategy, StrategyConfig, StrategyInput,
/// };
///
/// let strategy = AxisAgreementStrategy::new(StrategyConfig::default());
/// let long = StrategyInput::new(900, 0, 0, Some(0.2), Some(0.5), Some(1.0));
///
/// assert!(matches!(
///     strategy.decide(&long, StrategyPosition::flat()),
///     Decision::Enter(_)
/// ));
/// assert!(matches!(
///     strategy.decide(&long, StrategyPosition::with_side(
///         quantrule_backtest::side::Side::Long
///     )),
///     Decision::Hold
/// ));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisAgreementStrategy {
    config: StrategyConfig,
    risk: RiskPolicy,
}

impl AxisAgreementStrategy {
    /// Builds the rule for a configuration with the default risk policy.
    #[must_use]
    pub fn new(config: StrategyConfig) -> Self {
        Self {
            config,
            risk: RiskPolicy::default(),
        }
    }

    /// Builds the rule for a configuration and an explicit risk policy.
    #[must_use]
    pub const fn with_risk(config: StrategyConfig, risk: RiskPolicy) -> Self {
        Self { config, risk }
    }

    /// The configuration the strategy applies.
    #[must_use]
    pub const fn config(&self) -> StrategyConfig {
        self.config
    }

    /// The risk policy the strategy attaches to each entry.
    #[must_use]
    pub const fn risk(&self) -> RiskPolicy {
        self.risk
    }
}

impl Default for AxisAgreementStrategy {
    fn default() -> Self {
        Self::new(StrategyConfig::default())
    }
}

impl Strategy for AxisAgreementStrategy {
    fn decide(&self, input: &StrategyInput, position: StrategyPosition) -> Decision {
        if input.timeframe_secs != self.config.timeframe_secs() {
            return Decision::Hold;
        }

        let agreement = Agreement::classify(input, &self.config);

        match position.side {
            None => match agreement.and_then(Agreement::side) {
                Some(side) => Decision::Enter(Entry {
                    bar: *input,
                    intent: self.risk.intent(side),
                }),
                None => Decision::Hold,
            },
            Some(open) => {
                if holds(open, agreement) {
                    Decision::Hold
                } else {
                    Decision::Exit(Exit {
                        side: open,
                        bar: *input,
                        reason: ExitReason::AgreementLost,
                    })
                }
            }
        }
    }
}

/// Whether the open side is still confirmed by the bar's agreement.
///
/// An unavailable agreement (`None`) confirms nothing and denies nothing, so it
/// holds the position rather than closing it.
const fn holds(open: Side, agreement: Option<Agreement>) -> bool {
    match agreement {
        Some(Agreement::Long) => matches!(open, Side::Long),
        Some(Agreement::Short) => matches!(open, Side::Short),
        Some(Agreement::Neutral) => false,
        None => true,
    }
}
