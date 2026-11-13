//! Labelling a mark by the path that followed it.
//!
//! The label does not ask what the trade earned. It asks what the path looked
//! like: how far it went for and against the position, and which level it
//! touched first. Two entries with the same final result can have completely
//! different paths, and the path is what a rule has to survive.

use serde::{Deserialize, Serialize};

use crate::exit::Bar;
use crate::marks::Side;

/// Why a path cannot be labelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// No bars were supplied, or the horizon is zero.
    Empty,
    /// The entry and stop are the same price, or a price is not finite.
    InvalidPrices,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Empty => "a path needs at least one bar and a horizon",
            Self::InvalidPrices => "the entry and the stop must be distinct, finite prices",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PathError {}

/// Which level the path reached first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstTouch {
    /// One R in favour.
    Favorable,
    /// One R against.
    Adverse,
    /// Neither level within the horizon.
    Neither,
}

/// What the bars after an entry did, in R.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Path {
    /// Best excursion in favour.
    pub mfe_r: f64,
    /// Worst excursion against.
    pub mae_r: f64,
    /// Which level was reached first.
    pub first_touch: FirstTouch,
    /// Bars actually observed.
    pub bars: usize,
}

/// Label the path after an entry.
///
/// `horizon` is the number of bars considered, capped by the sample. A bar that
/// reaches both levels counts as adverse, the pessimistic order.
///
/// # Errors
///
/// Returns [`PathError`] when the sample or horizon is empty, or the prices are
/// unusable.
pub fn classify(
    bars: &[Bar],
    entry: f64,
    stop: f64,
    side: Side,
    horizon: usize,
) -> Result<Path, PathError> {
    if bars.is_empty() || horizon == 0 {
        return Err(PathError::Empty);
    }
    if !entry.is_finite() || !stop.is_finite() || entry == stop {
        return Err(PathError::InvalidPrices);
    }

    let risk = (entry - stop).abs();
    let mut mfe = 0.0_f64;
    let mut mae = 0.0_f64;
    let mut first_touch = FirstTouch::Neither;
    let observed = horizon.min(bars.len());

    for bar in &bars[..observed] {
        let (favour, against) = match side {
            Side::Long => ((bar.high - entry) / risk, (entry - bar.low) / risk),
            Side::Short => ((entry - bar.low) / risk, (bar.high - entry) / risk),
        };
        if favour > mfe {
            mfe = favour;
        }
        if against > mae {
            mae = against;
        }
        if first_touch == FirstTouch::Neither {
            if against >= 1.0 {
                first_touch = FirstTouch::Adverse;
            } else if favour >= 1.0 {
                first_touch = FirstTouch::Favorable;
            }
        }
    }

    Ok(Path {
        mfe_r: mfe,
        mae_r: mae,
        first_touch,
        bars: observed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(high: f64, low: f64, close: f64) -> Bar {
        Bar { high, low, close }
    }

    #[test]
    fn a_clean_winner_reaches_one_r_first() {
        let bars = [bar(101.0, 99.5, 100.8), bar(102.0, 100.5, 101.5)];
        let path = classify(&bars, 100.0, 99.0, Side::Long, 50).expect("path");
        assert_eq!(path.first_touch, FirstTouch::Favorable);
        assert!((path.mfe_r - 2.0).abs() < 1e-12);
        assert!((path.mae_r - 0.5).abs() < 1e-12);
    }

    #[test]
    fn a_clean_loser_reaches_one_r_against_first() {
        let bars = [bar(100.2, 98.5, 99.0)];
        let path = classify(&bars, 100.0, 99.0, Side::Long, 50).expect("path");
        assert_eq!(path.first_touch, FirstTouch::Adverse);
        assert!((path.mae_r - 1.5).abs() < 1e-12);
    }

    #[test]
    fn a_bar_touching_both_is_adverse() {
        let bars = [bar(105.0, 95.0, 100.0)];
        let path = classify(&bars, 100.0, 99.0, Side::Long, 50).expect("path");
        assert_eq!(path.first_touch, FirstTouch::Adverse);
    }

    #[test]
    fn a_short_is_mirrored() {
        let bars = [bar(100.5, 98.0, 99.0)];
        let path = classify(&bars, 100.0, 101.0, Side::Short, 50).expect("path");
        assert_eq!(path.first_touch, FirstTouch::Favorable);
        assert!((path.mfe_r - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_horizon_caps_the_observation() {
        let bars = [bar(100.5, 99.5, 100.0), bar(105.0, 99.0, 104.0)];
        let path = classify(&bars, 100.0, 99.0, Side::Long, 1).expect("path");
        assert_eq!(path.bars, 1);
        assert_eq!(path.first_touch, FirstTouch::Neither);
        assert!((path.mfe_r - 0.5).abs() < 1e-12);
    }

    #[test]
    fn invalid_inputs_are_refused() {
        assert_eq!(
            classify(&[], 100.0, 99.0, Side::Long, 50),
            Err(PathError::Empty)
        );
        assert_eq!(
            classify(&[bar(1.0, 1.0, 1.0)], 100.0, 100.0, Side::Long, 50),
            Err(PathError::InvalidPrices)
        );
    }

    #[test]
    fn a_path_round_trips_through_json() {
        let path = classify(&[bar(101.0, 99.5, 100.8)], 100.0, 99.0, Side::Long, 50).expect("path");
        let text = serde_json::to_string(&path).expect("encode");
        let back: Path = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, path);
    }
}
