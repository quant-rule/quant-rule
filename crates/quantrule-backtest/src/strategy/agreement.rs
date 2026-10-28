//! The first strategy: agreement of Direction, Momentum, and Structure.
//!
//! The rule enters in the direction all three readings agree on and exits when
//! that agreement no longer holds. It is causal: it reads the bar it is given
//! and its own position state, and it never looks ahead.
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

use super::config::StrategyConfig;
use super::decision::{Decision, Entry, Exit, ExitReason, Side, Strategy};
use super::input::{Agreement, StrategyInput};

/// The axis-agreement strategy.
///
/// The strategy is flat when constructed. It enters only while flat and exits
/// only while a position is open, so it never pyramids and never holds two
/// positions.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::{
///     AxisAgreementStrategy, Decision, Side, Strategy, StrategyConfig, StrategyInput,
/// };
///
/// let mut strategy = AxisAgreementStrategy::new(StrategyConfig::default());
/// let long = StrategyInput::new(900, 0, 0, Some(0.2), Some(0.5), Some(1.0));
///
/// assert!(matches!(strategy.on_bar(&long), Decision::Enter(_)));
/// assert_eq!(strategy.position(), Some(Side::Long));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisAgreementStrategy {
    config: StrategyConfig,
    position: Option<Side>,
}

impl AxisAgreementStrategy {
    /// Builds a flat strategy for a configuration.
    #[must_use]
    pub const fn new(config: StrategyConfig) -> Self {
        Self {
            config,
            position: None,
        }
    }

    /// The configuration the strategy applies.
    #[must_use]
    pub const fn config(&self) -> StrategyConfig {
        self.config
    }

    /// The side of the open position, or `None` while the strategy is flat.
    #[must_use]
    pub const fn position(&self) -> Option<Side> {
        self.position
    }
}

impl Default for AxisAgreementStrategy {
    fn default() -> Self {
        Self::new(StrategyConfig::default())
    }
}

impl Strategy for AxisAgreementStrategy {
    fn on_bar(&mut self, input: &StrategyInput) -> Decision {
        if input.timeframe_secs != self.config.timeframe_secs() {
            return Decision::Hold;
        }

        let agreement = Agreement::classify(input, &self.config);

        match self.position {
            None => match agreement.and_then(Agreement::side) {
                Some(side) => {
                    self.position = Some(side);
                    Decision::Enter(Entry { side, bar: *input })
                }
                None => Decision::Hold,
            },
            Some(open) => {
                if holds(open, agreement) {
                    Decision::Hold
                } else {
                    self.position = None;
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
