//! Execution costs and the bracket multiples.
//!
//! The configuration records the per-side fee rate, the slippage, and the two
//! ATR multiples that execution applies to a fill and to a position's bracket.
//! It has no default: the caller supplies the costs so a run cannot silently
//! assume zero fees. The portfolio module includes the fill fee in its
//! accounting.

use std::fmt;

/// The default stop distance in ATR units.
pub const DEFAULT_STOP_ATR_MULTIPLE: f64 = 2.0;

/// The default target distance in ATR units.
pub const DEFAULT_TARGET_ATR_MULTIPLE: f64 = 3.0;

/// The per-side fee rate, slippage, and bracket multiples used by execution.
///
/// `fee_rate` is a fraction of the fill notional, `slippage_bps` is in basis
/// points of the reference price, and the two multiples are positive ATR
/// distances. All values are finite. The configuration is a value, not a global
/// setting.
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
/// assert_eq!(config.stop_atr_multiple(), 2.0);
/// assert_eq!(config.target_atr_multiple(), 3.0);
/// # Ok::<(), quantrule_backtest::execution::ExecutionConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutionConfig {
    fee_rate: f64,
    slippage_bps: f64,
    stop_atr_multiple: f64,
    target_atr_multiple: f64,
}

impl ExecutionConfig {
    /// Builds an execution configuration with the default 2/3 bracket.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionConfigError`] when either cost is not finite or is
    /// negative.
    pub fn new(fee_rate: f64, slippage_bps: f64) -> Result<Self, ExecutionConfigError> {
        Self::with_bracket(
            fee_rate,
            slippage_bps,
            DEFAULT_STOP_ATR_MULTIPLE,
            DEFAULT_TARGET_ATR_MULTIPLE,
        )
    }

    /// Builds an execution configuration with explicit bracket multiples.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionConfigError`] when a cost is not finite or is
    /// negative, or when a multiple is not finite or not positive.
    pub fn with_bracket(
        fee_rate: f64,
        slippage_bps: f64,
        stop_atr_multiple: f64,
        target_atr_multiple: f64,
    ) -> Result<Self, ExecutionConfigError> {
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
        if !stop_atr_multiple.is_finite() {
            return Err(ExecutionConfigError::NonFiniteStopMultiple);
        }
        if stop_atr_multiple <= 0.0 {
            return Err(ExecutionConfigError::NonPositiveStopMultiple);
        }
        if !target_atr_multiple.is_finite() {
            return Err(ExecutionConfigError::NonFiniteTargetMultiple);
        }
        if target_atr_multiple <= 0.0 {
            return Err(ExecutionConfigError::NonPositiveTargetMultiple);
        }

        Ok(Self {
            fee_rate,
            slippage_bps,
            stop_atr_multiple,
            target_atr_multiple,
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

    /// The stop distance in ATR units.
    #[must_use]
    pub const fn stop_atr_multiple(&self) -> f64 {
        self.stop_atr_multiple
    }

    /// The target distance in ATR units.
    #[must_use]
    pub const fn target_atr_multiple(&self) -> f64 {
        self.target_atr_multiple
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
    /// The stop multiple is not a finite number.
    NonFiniteStopMultiple,
    /// The stop multiple is zero or negative.
    NonPositiveStopMultiple,
    /// The target multiple is not a finite number.
    NonFiniteTargetMultiple,
    /// The target multiple is zero or negative.
    NonPositiveTargetMultiple,
}

impl fmt::Display for ExecutionConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteFeeRate => write!(formatter, "the fee rate must be finite"),
            Self::NegativeFeeRate => write!(formatter, "the fee rate must not be negative"),
            Self::NonFiniteSlippage => write!(formatter, "the slippage must be finite"),
            Self::NegativeSlippage => write!(formatter, "the slippage must not be negative"),
            Self::NonFiniteStopMultiple => {
                write!(formatter, "the stop ATR multiple must be finite")
            }
            Self::NonPositiveStopMultiple => {
                write!(formatter, "the stop ATR multiple must be positive")
            }
            Self::NonFiniteTargetMultiple => {
                write!(formatter, "the target ATR multiple must be finite")
            }
            Self::NonPositiveTargetMultiple => {
                write!(formatter, "the target ATR multiple must be positive")
            }
        }
    }
}

impl std::error::Error for ExecutionConfigError {}
