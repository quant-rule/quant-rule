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

mod atr;
mod ema;
mod pivots;
mod rsi;
mod rvol;
mod series;

pub mod roles;

pub use atr::atr;
pub use ema::ema;
pub use pivots::{Pivot, PivotCandidate, PivotKind, PivotScale, PivotSeries};
pub use rsi::rsi;
pub use rvol::rvol;
pub use series::ReadingSeries;

use quantrule_data::Candle;

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
    /// Computes every reading from the closed candles.
    ///
    /// The input is not modified. The output length equals the input length for
    /// every scalar series and for the candidate series.
    #[must_use]
    pub fn compute(candles: &[Candle]) -> Self {
        Self {
            ema20: ema(candles, 20),
            ema50: ema(candles, 50),
            ema200: ema(candles, 200),
            rsi14: rsi(candles, 14),
            atr14: atr(candles, 14),
            rvol20: rvol(candles, 20),
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
