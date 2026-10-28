//! Strategy configuration and its validation.
//!
//! The configuration fixes the timeframe and the two thresholds once, so a rule
//! cannot read a threshold from the environment or change it between bars. The
//! thresholds must straddle zero: a Long classification must mean a strictly
//! positive reading, and a Short classification a strictly negative one.

use std::fmt;

/// The default timeframe of the first strategy: 15 minutes.
pub const DEFAULT_TIMEFRAME_SECS: u64 = 900;

/// The default long threshold: `+0.1`.
pub const DEFAULT_LONG_THRESHOLD: f64 = 0.1;

/// The default short threshold: `-0.1`.
pub const DEFAULT_SHORT_THRESHOLD: f64 = -0.1;

/// The validated configuration of a strategy.
///
/// The fields are private so that an invalid configuration cannot be built by
/// struct literal. Use [`StrategyConfig::new`] or [`StrategyConfig::default`].
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::StrategyConfig;
///
/// let config = StrategyConfig::new(900, 0.1, -0.1)?;
/// assert_eq!(config.timeframe_secs(), 900);
/// assert_eq!(config.long_threshold(), 0.1);
/// assert_eq!(config.short_threshold(), -0.1);
/// # Ok::<(), quantrule_backtest::strategy::StrategyConfigError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrategyConfig {
    timeframe_secs: u64,
    long_threshold: f64,
    short_threshold: f64,
}

impl StrategyConfig {
    /// Builds a configuration from a timeframe and two directional thresholds.
    ///
    /// # Errors
    ///
    /// Returns an error when the timeframe is zero, either threshold is not
    /// finite, the long threshold is not greater than the short threshold, the
    /// long threshold is negative, or the short threshold is positive.
    pub fn new(
        timeframe_secs: u64,
        long_threshold: f64,
        short_threshold: f64,
    ) -> Result<Self, StrategyConfigError> {
        if timeframe_secs == 0 {
            return Err(StrategyConfigError::ZeroTimeframe);
        }
        if !long_threshold.is_finite() {
            return Err(StrategyConfigError::NonFiniteLongThreshold);
        }
        if !short_threshold.is_finite() {
            return Err(StrategyConfigError::NonFiniteShortThreshold);
        }
        if long_threshold <= short_threshold {
            return Err(StrategyConfigError::LongNotAboveShort);
        }
        if long_threshold < 0.0 {
            return Err(StrategyConfigError::NegativeLongThreshold);
        }
        if short_threshold > 0.0 {
            return Err(StrategyConfigError::PositiveShortThreshold);
        }

        Ok(Self {
            timeframe_secs,
            long_threshold,
            short_threshold,
        })
    }

    /// The strategy timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(&self) -> u64 {
        self.timeframe_secs
    }

    /// The threshold all three readings must strictly exceed for a long
    /// agreement.
    #[must_use]
    pub const fn long_threshold(&self) -> f64 {
        self.long_threshold
    }

    /// The threshold all three readings must strictly fall below for a short
    /// agreement.
    #[must_use]
    pub const fn short_threshold(&self) -> f64 {
        self.short_threshold
    }
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            timeframe_secs: DEFAULT_TIMEFRAME_SECS,
            long_threshold: DEFAULT_LONG_THRESHOLD,
            short_threshold: DEFAULT_SHORT_THRESHOLD,
        }
    }
}

/// A strategy configuration was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyConfigError {
    /// The timeframe is zero.
    ZeroTimeframe,
    /// The long threshold is not finite.
    NonFiniteLongThreshold,
    /// The short threshold is not finite.
    NonFiniteShortThreshold,
    /// The long threshold is negative.
    NegativeLongThreshold,
    /// The short threshold is positive.
    PositiveShortThreshold,
    /// The long threshold is not greater than the short threshold.
    LongNotAboveShort,
}

impl fmt::Display for StrategyConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTimeframe => {
                write!(formatter, "the strategy timeframe must be non-zero")
            }
            Self::NonFiniteLongThreshold => {
                write!(formatter, "the long threshold must be finite")
            }
            Self::NonFiniteShortThreshold => {
                write!(formatter, "the short threshold must be finite")
            }
            Self::NegativeLongThreshold => {
                write!(formatter, "the long threshold must be at least zero")
            }
            Self::PositiveShortThreshold => {
                write!(formatter, "the short threshold must be at most zero")
            }
            Self::LongNotAboveShort => {
                write!(
                    formatter,
                    "the long threshold must be greater than the short threshold"
                )
            }
        }
    }
}

impl std::error::Error for StrategyConfigError {}
