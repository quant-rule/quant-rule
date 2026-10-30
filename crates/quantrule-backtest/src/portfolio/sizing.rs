//! Position sizing and its typed rejection.
//!
//! Sizing turns a decision-bar equity and close into a quantity. It is a pure
//! function of the configuration and the two inputs: no clock, no randomness,
//! no position state. A quantity is floored to the configured step, and a
//! quantity below one step is a rejection rather than a zero.
//!
//! The sizing price is the decision-bar close. The fill happens at the next
//! bar's open, so the realized notional can differ from the sizing notional. The
//! runner separately caps the fill quantity at current equity × leverage × the
//! policy position fraction; this module computes the unconstrained quantity.

use std::fmt;

use crate::side::Side;

use super::config::PortfolioConfigError;

/// How a position is sized.
///
/// Both modes floor the quantity to `quantity_step`. A fixed notional ignores
/// the account equity; an equity fraction scales with it.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::SizingMode;
/// use quantrule_backtest::side::Side;
///
/// let fixed = SizingMode::FixedNotional {
///     notional: 10_000.0,
///     quantity_step: 0.0001,
/// };
/// assert_eq!(fixed.size(Side::Long, 10_000.0, 200.0, true)?, 50.0);
///
/// let fraction = SizingMode::EquityFraction {
///     fraction: 0.5,
///     quantity_step: 1.0,
/// };
/// assert_eq!(fraction.size(Side::Long, 10_000.0, 200.0, true)?, 25.0);
/// # Ok::<(), quantrule_backtest::portfolio::SizingRejection>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizingMode {
    /// Size a fixed quote-currency notional.
    FixedNotional {
        /// The quote-currency notional of one entry.
        notional: f64,
        /// The minimum quantity increment.
        quantity_step: f64,
    },
    /// Size a fraction of the account equity.
    EquityFraction {
        /// The fraction of equity to commit, in `(0, 1]`.
        fraction: f64,
        /// The minimum quantity increment.
        quantity_step: f64,
    },
}

impl SizingMode {
    /// Validates the mode's fields.
    ///
    /// # Errors
    ///
    /// Returns an error when the notional, fraction, or step is not finite,
    /// when the notional or step is not positive, or when the fraction is
    /// outside `(0, 1]`.
    pub fn validate(&self) -> Result<(), PortfolioConfigError> {
        match *self {
            Self::FixedNotional {
                notional,
                quantity_step,
            } => {
                if !notional.is_finite() {
                    return Err(PortfolioConfigError::NonFiniteNotional);
                }
                if notional <= 0.0 {
                    return Err(PortfolioConfigError::NonPositiveNotional);
                }
                validate_step(quantity_step)?;
            }
            Self::EquityFraction {
                fraction,
                quantity_step,
            } => {
                if !fraction.is_finite() {
                    return Err(PortfolioConfigError::NonFiniteFraction);
                }
                if fraction <= 0.0 || fraction > 1.0 {
                    return Err(PortfolioConfigError::FractionOutOfRange);
                }
                validate_step(quantity_step)?;
            }
        }

        Ok(())
    }

    /// The minimum quantity increment.
    #[must_use]
    pub const fn quantity_step(&self) -> f64 {
        match *self {
            Self::FixedNotional { quantity_step, .. }
            | Self::EquityFraction { quantity_step, .. } => quantity_step,
        }
    }

    /// Sizes one entry from the equity and price at the decision bar.
    ///
    /// The quantity is `notional / price` or `(equity * fraction) / price`,
    /// floored to the step. The result is never zero: a quantity below one step
    /// returns [`SizingRejection::BelowOneStep`].
    ///
    /// # Errors
    ///
    /// Returns a rejection when the equity or price is invalid, a short is not
    /// allowed, the mode is invalid, or the result is below one step.
    pub fn size(
        &self,
        side: Side,
        equity: f64,
        price: f64,
        allow_short: bool,
    ) -> Result<f64, SizingRejection> {
        if !equity.is_finite() {
            return Err(SizingRejection::NonFiniteEquity);
        }
        if equity < 0.0 {
            return Err(SizingRejection::NegativeEquity);
        }
        if !price.is_finite() {
            return Err(SizingRejection::NonFinitePrice);
        }
        if price <= 0.0 {
            return Err(SizingRejection::NonPositivePrice);
        }
        if matches!(side, Side::Short) && !allow_short {
            return Err(SizingRejection::ShortNotAllowed);
        }

        let (raw, step) = match *self {
            Self::FixedNotional {
                notional,
                quantity_step,
            } => {
                if !notional.is_finite() {
                    return Err(SizingRejection::NonFiniteNotional);
                }
                if notional <= 0.0 {
                    return Err(SizingRejection::NonPositiveNotional);
                }
                if !quantity_step.is_finite() {
                    return Err(SizingRejection::NonFiniteStep);
                }
                if quantity_step <= 0.0 {
                    return Err(SizingRejection::NonPositiveStep);
                }
                (notional / price, quantity_step)
            }
            Self::EquityFraction {
                fraction,
                quantity_step,
            } => {
                if !fraction.is_finite() {
                    return Err(SizingRejection::NonFiniteFraction);
                }
                if fraction <= 0.0 || fraction > 1.0 {
                    return Err(SizingRejection::FractionOutOfRange);
                }
                if !quantity_step.is_finite() {
                    return Err(SizingRejection::NonFiniteStep);
                }
                if quantity_step <= 0.0 {
                    return Err(SizingRejection::NonPositiveStep);
                }
                ((equity * fraction) / price, quantity_step)
            }
        };

        let steps = floored_steps(raw, step);
        if steps < 1.0 {
            return Err(SizingRejection::BelowOneStep { raw, step });
        }

        let quantity = normalize_quantity(steps * step);
        if !quantity.is_finite() || quantity <= 0.0 {
            return Err(SizingRejection::BelowOneStep { raw, step });
        }

        Ok(quantity)
    }
}

/// The decimal scale used to publish a quantity.
///
/// Quantities are quote-asset units; twelve decimal places cover every
/// configured step in the supported universe and remove binary noise such as
/// `6 * 0.1 = 0.6000000000000001`.
const QUANTITY_SCALE: f64 = 1_000_000_000_000.0;

fn normalize_quantity(quantity: f64) -> f64 {
    if quantity.is_finite() {
        (quantity * QUANTITY_SCALE).round() / QUANTITY_SCALE
    } else {
        quantity
    }
}

/// Floors `raw / step` without losing an exact boundary to binary rounding.
///
/// A decimal boundary such as `0.3 / 0.1` can evaluate to `2.999…`. The
/// tolerance is relative to the ratio and only absorbs representation error;
/// a ratio genuinely below the boundary still floors down.
fn floored_steps(raw: f64, step: f64) -> f64 {
    let ratio = raw / step;
    if !ratio.is_finite() {
        return ratio.floor();
    }
    let nearest = ratio.round();
    let tolerance = 16.0 * f64::EPSILON * ratio.abs().max(1.0);
    if (ratio - nearest).abs() <= tolerance {
        nearest.max(0.0)
    } else {
        ratio.floor()
    }
}

fn validate_step(step: f64) -> Result<(), PortfolioConfigError> {
    if !step.is_finite() {
        return Err(PortfolioConfigError::NonFiniteQuantityStep);
    }
    if step <= 0.0 {
        return Err(PortfolioConfigError::NonPositiveQuantityStep);
    }

    Ok(())
}

/// Why an entry could not be sized.
///
/// A rejection is a typed reason, not a zero quantity. The caller records it
/// and leaves the account flat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizingRejection {
    /// The equity is not a finite number.
    NonFiniteEquity,
    /// The equity is negative.
    NegativeEquity,
    /// The price is not a finite number.
    NonFinitePrice,
    /// The price is zero or negative.
    NonPositivePrice,
    /// A short entry was requested while `allow_short` is false.
    ShortNotAllowed,
    /// The fixed notional is not a finite number.
    NonFiniteNotional,
    /// The fixed notional is zero or negative.
    NonPositiveNotional,
    /// The equity fraction is not a finite number.
    NonFiniteFraction,
    /// The equity fraction is outside `(0, 1]`.
    FractionOutOfRange,
    /// The quantity step is not a finite number.
    NonFiniteStep,
    /// The quantity step is zero or negative.
    NonPositiveStep,
    /// The unfloored quantity is below one step.
    BelowOneStep {
        /// The unfloored quantity.
        raw: f64,
        /// The configured step.
        step: f64,
    },
    /// The portfolio is halted by the drawdown limit.
    Halted,
}

impl fmt::Display for SizingRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteEquity => write!(formatter, "the equity must be finite"),
            Self::NegativeEquity => write!(formatter, "the equity must not be negative"),
            Self::NonFinitePrice => write!(formatter, "the price must be finite"),
            Self::NonPositivePrice => write!(formatter, "the price must be positive"),
            Self::ShortNotAllowed => write!(formatter, "a short entry is not allowed"),
            Self::NonFiniteNotional => write!(formatter, "the fixed notional must be finite"),
            Self::NonPositiveNotional => write!(formatter, "the fixed notional must be positive"),
            Self::NonFiniteFraction => write!(formatter, "the equity fraction must be finite"),
            Self::FractionOutOfRange => {
                write!(formatter, "the equity fraction must be in (0, 1]")
            }
            Self::NonFiniteStep => write!(formatter, "the quantity step must be finite"),
            Self::NonPositiveStep => write!(formatter, "the quantity step must be positive"),
            Self::BelowOneStep { raw, step } => {
                write!(formatter, "the quantity {raw} is below one step of {step}")
            }
            Self::Halted => write!(formatter, "the portfolio is halted"),
        }
    }
}

impl std::error::Error for SizingRejection {}
