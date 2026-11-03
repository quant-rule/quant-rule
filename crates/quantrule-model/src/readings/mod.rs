//! The model's indicator readings.
//!
//! The readings are the model's calculation inputs: EMA 20/50/200, RSI 14,
//! ATR 14, RVOL 20, and confirmed swing pivots at a minor and a major scale.
//! Every reading is a pure function of the closed candles passed in. Scalar
//! readings publish one value per input candle; a value is `None` until the
//! required history exists, and a measured zero is `Some(0.0)`.
//!
//! A reading is not owned by an axis. The [`roles`] module records each use of
//! a reading, the consumer of that use, and the definition of that use. The
//! axes, event detectors, and context diagnostic consume the same readings
//! through those definitions.
//!
//! This module computes no axis, event, aggregate, or market-state value. The
//! [`roles`] registry names those consumers and their contract codes, but the
//! axis and event calculations live in their own modules. This module performs
//! no I/O, reads no clock, and uses no randomness.

mod adx;
mod atr;
mod ema;
mod macd;
mod pivots;
mod rsi;
mod rvol;
mod series;

pub mod roles;

pub use adx::{AdxSeries, AdxState};
pub use atr::atr;
pub use ema::ema;
pub use macd::MacdSeries;
pub use pivots::{Pivot, PivotCandidate, PivotKind, PivotScale, PivotSeries};
pub use rsi::rsi;
pub use rvol::rvol;
pub use series::ReadingSeries;

use quantrule_data::Candle;

/// The configurable periods of the scalar readings.
///
/// The field names on [`Readings`] identify the reading's role: `ema20`,
/// `ema50`, and `ema200` are the fast, middle, and slow EMA roles, and `rsi14`,
/// `atr14`, and `rvol20` are the RSI, ATR, and relative-volume roles. The
/// configured period may differ from the role label; the model config records
/// the period that produced the value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadingsConfig {
    /// The fast EMA period.
    pub ema_fast: usize,
    /// The middle EMA period.
    pub ema_mid: usize,
    /// The slow EMA period.
    pub ema_slow: usize,
    /// The RSI period.
    pub rsi: usize,
    /// The ATR period.
    pub atr: usize,
    /// The relative-volume baseline period.
    pub rvol: usize,
    /// The MACD fast period.
    pub macd_fast: usize,
    /// The MACD slow period.
    pub macd_slow: usize,
    /// The MACD signal period.
    pub macd_signal: usize,
    /// The ADX period.
    pub adx: usize,
    /// The ADX trend-strength threshold.
    pub adx_threshold: f64,
    /// The ADX slope lookback in bars.
    pub adx_slope_bars: usize,
    /// Whether the ADX/DI family is active for this timeframe.
    pub adx_active: bool,
    /// Whether the EMA family is active.
    pub ema_active: bool,
    /// Whether RSI is active.
    pub rsi_active: bool,
    /// Whether ATR is active.
    pub atr_active: bool,
    /// Whether RVOL is active.
    pub rvol_active: bool,
}

impl ReadingsConfig {
    /// The default periods: EMA 20/50/200, RSI 14, ATR 14, RVOL 20.
    pub const DEFAULT: Self = Self {
        ema_fast: 20,
        ema_mid: 50,
        ema_slow: 200,
        rsi: 14,
        atr: 14,
        rvol: 20,
        macd_fast: 12,
        macd_slow: 26,
        macd_signal: 9,
        adx: 14,
        adx_threshold: 25.0,
        adx_slope_bars: 3,
        adx_active: true,
        ema_active: true,
        rsi_active: true,
        atr_active: true,
        rvol_active: true,
    };

    /// Builds a reading configuration.
    ///
    /// The caller validates the periods; a zero period produces an unavailable
    /// series, and a non-increasing EMA triple is rejected by the config loader.
    #[must_use]
    pub const fn new(
        ema_fast: usize,
        ema_mid: usize,
        ema_slow: usize,
        rsi: usize,
        atr: usize,
        rvol: usize,
    ) -> Self {
        Self {
            ema_fast,
            ema_mid,
            ema_slow,
            rsi,
            atr,
            rvol,
            macd_fast: 12,
            macd_slow: 26,
            macd_signal: 9,
            adx: 14,
            adx_threshold: 25.0,
            adx_slope_bars: 3,
            adx_active: true,
            ema_active: true,
            rsi_active: true,
            atr_active: true,
            rvol_active: true,
        }
    }

    /// Sets the ADX parameters.
    #[must_use]
    pub const fn with_adx(mut self, period: usize, threshold: f64, slope_bars: usize) -> Self {
        self.adx = period;
        self.adx_threshold = threshold;
        self.adx_slope_bars = slope_bars;
        self
    }

    /// Enables or disables the ADX/DI family.
    #[must_use]
    pub const fn with_adx_active(mut self, active: bool) -> Self {
        self.adx_active = active;
        self
    }

    /// Sets the MACD periods.
    #[must_use]
    pub const fn with_macd(mut self, fast: usize, slow: usize, signal: usize) -> Self {
        self.macd_fast = fast;
        self.macd_slow = slow;
        self.macd_signal = signal;
        self
    }

    /// Whether the periods are positive and the EMA periods are strictly
    /// increasing.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.ema_fast > 0
            && self.ema_fast < self.ema_mid
            && self.ema_mid < self.ema_slow
            && self.rsi > 0
            && self.atr > 0
            && self.rvol > 0
            && self.macd_fast > 0
            && self.macd_fast < self.macd_slow
            && self.macd_signal > 0
            && self.adx > 0
            && self.adx_threshold > 0.0
            && self.adx_slope_bars > 0
    }
}

impl Default for ReadingsConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The complete set of indicator readings for one candle series.
///
/// The scalar series and the pivot candidate series have exactly one entry per
/// input candle. The confirmed pivots are stored by availability bar inside
/// [`PivotSeries`], so a consumer cannot read a pivot before it is knowable.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::readings::Readings;
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let readings = Readings::compute(&[candle]);
///
/// assert_eq!(readings.len(), 1);
/// assert_eq!(readings.ema20.get(0), None);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Readings {
    /// EMA 20.
    pub ema20: ReadingSeries,
    /// EMA 50.
    pub ema50: ReadingSeries,
    /// EMA 200.
    pub ema200: ReadingSeries,
    /// RSI 14.
    pub rsi14: ReadingSeries,
    /// ATR 14.
    pub atr14: ReadingSeries,
    /// RVOL 20.
    pub rvol20: ReadingSeries,
    /// Confirmed pivots and candidates at both scales.
    pub pivots: PivotSeries,
}

impl Readings {
    /// Computes every reading with the default periods.
    ///
    /// The input is not modified. The output length equals the input length for
    /// every scalar series and for the candidate series.
    #[must_use]
    pub fn compute(candles: &[Candle]) -> Self {
        Self::compute_with_config(candles, &ReadingsConfig::default())
    }

    /// Computes every reading with an explicit period configuration.
    ///
    /// The input is not modified. The output length equals the input length for
    /// every scalar series and for the candidate series.
    #[must_use]
    pub fn compute_with_config(candles: &[Candle], config: &ReadingsConfig) -> Self {
        let empty = || ReadingSeries::new(vec![None; candles.len()]);
        Self {
            ema20: if config.ema_active {
                ema(candles, config.ema_fast)
            } else {
                empty()
            },
            ema50: if config.ema_active {
                ema(candles, config.ema_mid)
            } else {
                empty()
            },
            ema200: if config.ema_active {
                ema(candles, config.ema_slow)
            } else {
                empty()
            },
            rsi14: if config.rsi_active {
                rsi(candles, config.rsi)
            } else {
                empty()
            },
            atr14: if config.atr_active {
                atr(candles, config.atr)
            } else {
                empty()
            },
            rvol20: if config.rvol_active {
                rvol(candles, config.rvol)
            } else {
                empty()
            },
            pivots: PivotSeries::compute(candles),
        }
    }

    /// The number of input bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ema20.len()
    }

    /// Whether the input series was empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ema20.is_empty()
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use quantrule_data::Candle;

    pub(crate) fn candle(close: f64) -> Candle {
        candle_ohlcv(close, close, close, None)
    }

    pub(crate) fn candle_with_volume(close: f64, volume: Option<f64>) -> Candle {
        candle_ohlcv(close, close, close, volume)
    }

    pub(crate) fn candle_ohlcv(high: f64, low: f64, close: f64, volume: Option<f64>) -> Candle {
        Candle::new(0, 0, close, high, low, close, volume).expect("valid test candle")
    }

    pub(crate) fn candles_from_closes(closes: &[f64]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(index, close)| {
                Candle::new(
                    index as u64,
                    index as i64,
                    *close,
                    *close,
                    *close,
                    *close,
                    None,
                )
                .expect("valid test candle")
            })
            .collect()
    }

    pub(crate) fn candles_from_ohlcv(rows: &[(f64, f64, f64)]) -> Vec<Candle> {
        rows.iter()
            .enumerate()
            .map(|(index, (high, low, close))| {
                Candle::new(
                    index as u64,
                    index as i64,
                    *close,
                    *high,
                    *low,
                    *close,
                    None,
                )
                .expect("valid test candle")
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::candles_from_closes;
    use super::{PivotScale, Readings};

    #[test]
    fn compute_with_config_uses_the_configured_periods() {
        use super::ReadingsConfig;
        use super::test_support::candle_with_volume;

        let candles: Vec<_> = (0..25)
            .map(|index| candle_with_volume(100.0 + f64::from(index), Some(10.0)))
            .collect();
        let config = ReadingsConfig::new(3, 5, 8, 2, 3, 4);
        let readings = Readings::compute_with_config(&candles, &config);

        assert_eq!(readings.ema20.get(1), None);
        assert!(readings.ema20.get(2).is_some());
        assert_eq!(readings.ema50.get(3), None);
        assert!(readings.ema50.get(4).is_some());
        assert_eq!(readings.ema200.get(6), None);
        assert!(readings.ema200.get(7).is_some());
        assert_eq!(readings.rsi14.get(1), None);
        assert!(readings.rsi14.get(2).is_some());
        assert_eq!(readings.atr14.get(2), None);
        assert!(readings.atr14.get(3).is_some());
        assert_eq!(readings.rvol20.get(2), None);
        assert!(readings.rvol20.get(3).is_some());
    }

    #[test]
    fn compute_aligns_every_series_to_the_input_length() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0, 4.0]);
        let readings = Readings::compute(&candles);

        assert_eq!(readings.len(), candles.len());
        assert_eq!(readings.ema20.len(), candles.len());
        assert_eq!(readings.ema50.len(), candles.len());
        assert_eq!(readings.ema200.len(), candles.len());
        assert_eq!(readings.rsi14.len(), candles.len());
        assert_eq!(readings.atr14.len(), candles.len());
        assert_eq!(readings.rvol20.len(), candles.len());
        assert_eq!(readings.pivots.len(), candles.len());
        assert_eq!(
            readings.pivots.candidates(PivotScale::Minor).len(),
            candles.len()
        );
        assert_eq!(
            readings.pivots.candidates(PivotScale::Major).len(),
            candles.len()
        );
    }

    #[test]
    fn every_input_length_keeps_every_series_aligned() {
        let closes: Vec<f64> = (0..40)
            .map(|index| 100.0 + f64::from(index) * 0.5)
            .collect();
        let candles = candles_from_closes(&closes);

        for length in 0..=candles.len() {
            let readings = Readings::compute(&candles[..length]);
            assert_eq!(readings.len(), length);
            for series in [
                &readings.ema20,
                &readings.ema50,
                &readings.ema200,
                &readings.rsi14,
                &readings.atr14,
                &readings.rvol20,
            ] {
                assert_eq!(series.len(), length);
            }
            assert_eq!(readings.pivots.len(), length);
            for scale in PivotScale::ALL {
                assert_eq!(readings.pivots.candidates(scale).len(), length);
            }
        }
    }

    #[test]
    fn an_empty_series_produces_empty_readings() {
        let readings = Readings::compute(&[]);

        assert!(readings.is_empty());
        assert_eq!(readings.len(), 0);
        assert_eq!(readings.ema20.get(0), None);
        assert_eq!(readings.rsi14.get(0), None);
        assert_eq!(readings.atr14.get(0), None);
        assert_eq!(readings.rvol20.get(0), None);
        assert_eq!(readings.pivots.len(), 0);
    }
}
