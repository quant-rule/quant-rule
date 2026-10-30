//! Execution costs and fill mechanics configuration.
//!
//! The configuration records the per-side fee rate and the slippage. It has no
//! default: the caller supplies the costs so a run cannot silently assume zero
//! fees. The portfolio module includes the fill fee in its accounting.
//!
//! Stop and target distances are not execution settings. They arrive in the
//! strategy's [`crate::intent::TradeIntent`] and execution resolves them against
//! the fill. Keeping the distances out of this type is what makes execution
//! mechanics independent of the strategy's risk policy.

use std::fmt;

/// The per-side fee rate and slippage used by execution.
///
/// `fee_rate` is a fraction of the fill notional and `slippage_bps` is in basis
/// points of the reference price. Both values are finite and non-negative. The
/// configuration is a value, not a global setting.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::ExecutionConfig;
///
/// let config = ExecutionConfig::new(0.001, 2.0)?;
/// assert_eq!(config.fee_rate(), 0.001);
/// assert_eq!(config.slippage_bps(), 2.0);
/// assert!((config.slippage_rate() - 0.0002).abs() < 1e-12);
/// # Ok::<(), quantrule_backtest::execution::ExecutionConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutionConfig {
    fee_rate: f64,
    slippage_bps: f64,
}

impl ExecutionConfig {
    /// Builds an execution configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionConfigError`] when either cost is not finite or is
    /// negative.
    pub fn new(fee_rate: f64, slippage_bps: f64) -> Result<Self, ExecutionConfigError> {
        if !fee_rate.is_finite() {
            return Err(ExecutionConfigError::NonFiniteFeeRate);
        }
        if fee_rate < 0.0 {
            return Err(ExecutionConfigError::NegativeFeeRate);
        }
        if !slippage_bps.is_finite() {
            return Err(ExecutionConfigError::NonFiniteSlippage);
        }
        if slippage_bps < 0.0 {
            return Err(ExecutionConfigError::NegativeSlippage);
        }

        Ok(Self {
            fee_rate,
            slippage_bps,
        })
    }

    /// The fee rate as a fraction of the fill notional.
    #[must_use]
    pub const fn fee_rate(&self) -> f64 {
        self.fee_rate
    }

    /// The slippage in basis points of the reference price.
    #[must_use]
    pub const fn slippage_bps(&self) -> f64 {
        self.slippage_bps
    }

    /// The slippage as a fraction of the reference price.
    #[must_use]
    pub fn slippage_rate(&self) -> f64 {
        self.slippage_bps / 10_000.0
    }
}

/// An execution configuration was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionConfigError {
    /// The fee rate is not a finite number.
    NonFiniteFeeRate,
    /// The fee rate is negative.
    NegativeFeeRate,
    /// The slippage is not a finite number.
    NonFiniteSlippage,
    /// The slippage is negative.
    NegativeSlippage,
}

impl fmt::Display for ExecutionConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteFeeRate => write!(formatter, "the fee rate must be finite"),
            Self::NegativeFeeRate => write!(formatter, "the fee rate must not be negative"),
            Self::NonFiniteSlippage => write!(formatter, "the slippage must be finite"),
            Self::NegativeSlippage => write!(formatter, "the slippage must not be negative"),
        }
    }
}

impl std::error::Error for ExecutionConfigError {}
