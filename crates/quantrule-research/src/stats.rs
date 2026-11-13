//! The measures one hypothesis is judged by.
//!
//! Expectancy is the average result per trade in R. Significance asks whether
//! that average could plausibly be zero. Both are reported with the sample size
//! and the drawdown, because an edge with a 40% drawdown is not the same finding
//! as one with 5%.

use serde::{Deserialize, Serialize};

use crate::rng::Pcg32;

/// How many bootstrap resamples one evaluation uses.
pub const RESAMPLES: u32 = 5_000;

/// The two-sided confidence level reported for the mean.
pub const CONFIDENCE: f64 = 0.95;

/// The threshold below which a result is called significant.
pub const ALPHA: f64 = 0.05;

/// Why a sample cannot be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatsError {
    /// No trades were supplied.
    Empty,
    /// A return was not a finite number.
    NotFinite,
}

impl std::fmt::Display for StatsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a hypothesis needs at least one trade"),
            Self::NotFinite => formatter.write_str("a return is not a finite number"),
        }
    }
}

impl std::error::Error for StatsError {}

/// What one hypothesis measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statistics {
    /// Trades in the sample.
    pub trades: usize,
    /// Average result per trade, in R.
    pub expectancy_r: f64,
    /// Fraction of trades with a positive result.
    pub hit_rate: f64,
    /// Largest peak-to-trough fall of the cumulative result, in R.
    pub max_drawdown_r: f64,
    /// One-sided bootstrap probability of a mean at or below zero.
    pub p_value: f64,
    /// Lower bound of the confidence interval for the mean, in R.
    pub ci_low_r: f64,
    /// Upper bound of the confidence interval for the mean, in R.
    pub ci_high_r: f64,
    /// The confidence level behind the interval.
    pub confidence: f64,
    /// Whether `p_value` is below [`ALPHA`].
    pub significant: bool,
}

/// Evaluate one sample of trade returns.
///
/// # Errors
///
/// Returns [`StatsError::Empty`] for an empty sample and
/// [`StatsError::NotFinite`] when a return is NaN or infinite.
pub fn evaluate(returns: &[f64], seed: u64) -> Result<Statistics, StatsError> {
    if returns.is_empty() {
        return Err(StatsError::Empty);
    }
    if returns.iter().any(|value| !value.is_finite()) {
        return Err(StatsError::NotFinite);
    }

    let trades = returns.len();
    let mean = returns.iter().sum::<f64>() / trades as f64;
    let wins = returns.iter().filter(|value| **value > 0.0).count();
    let hit_rate = wins as f64 / trades as f64;
    let max_drawdown_r = drawdown(returns);

    let mut generator = Pcg32::new(seed);
    let mut means = Vec::with_capacity(RESAMPLES as usize);
    for _ in 0..RESAMPLES {
        let mut sum = 0.0;
        for _ in 0..trades {
            sum += returns[generator.below(trades)];
        }
        means.push(sum / trades as f64);
    }

    let at_or_below_zero = means.iter().filter(|value| **value <= 0.0).count();
    let p_value = (at_or_below_zero as f64 + 1.0) / (f64::from(RESAMPLES) + 1.0);

    means.sort_by(f64::total_cmp);
    let tail = ((1.0 - CONFIDENCE) / 2.0 * f64::from(RESAMPLES)) as usize;
    let ci_low_r = means[tail.min(means.len() - 1)];
    let ci_high_r = means[means.len() - 1 - tail];

    Ok(Statistics {
        trades,
        expectancy_r: mean,
        hit_rate,
        max_drawdown_r,
        p_value,
        ci_low_r,
        ci_high_r,
        confidence: CONFIDENCE,
        significant: p_value < ALPHA,
    })
}

/// The largest peak-to-trough fall of the cumulative result.
fn drawdown(returns: &[f64]) -> f64 {
    let mut cumulative = 0.0;
    let mut peak = 0.0;
    let mut worst = 0.0;
    for value in returns {
        cumulative += value;
        if cumulative > peak {
            peak = cumulative;
        }
        let fall = peak - cumulative;
        if fall > worst {
            worst = fall;
        }
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(winning: usize, losing: usize) -> Vec<f64> {
        let mut returns = vec![3.0; winning];
        returns.extend(vec![-1.0; losing]);
        returns
    }

    #[test]
    fn the_worked_example_is_point_eight_r() {
        let stats = evaluate(&sample(45, 55), 7).expect("evaluate");
        assert!(
            (stats.expectancy_r - 0.8).abs() < 1e-12,
            "{}",
            stats.expectancy_r
        );
        assert!((stats.hit_rate - 0.45).abs() < 1e-12);
        assert_eq!(stats.trades, 100);
    }

    #[test]
    fn a_zero_mean_sample_is_not_significant() {
        // 25 wins at +3R and 75 losses at -1R average exactly zero.
        let stats = evaluate(&sample(25, 75), 7).expect("evaluate");
        assert!(stats.expectancy_r.abs() < 1e-12);
        assert!(!stats.significant, "p={}", stats.p_value);
    }

    #[test]
    fn a_clear_edge_is_significant() {
        let stats = evaluate(&sample(60, 40), 7).expect("evaluate");
        assert!(stats.significant, "p={}", stats.p_value);
        assert!(stats.ci_low_r > 0.0, "ci_low={}", stats.ci_low_r);
    }

    #[test]
    fn drawdown_is_peak_to_trough() {
        let stats = evaluate(&[1.0, -3.0, 1.0], 7).expect("evaluate");
        assert!((stats.max_drawdown_r - 3.0).abs() < 1e-12);
    }

    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let first = evaluate(&sample(45, 55), 11).expect("evaluate");
        let second = evaluate(&sample(45, 55), 11).expect("evaluate");
        assert_eq!(first, second);
    }

    #[test]
    fn an_empty_sample_is_refused() {
        assert_eq!(evaluate(&[], 1), Err(StatsError::Empty));
    }

    #[test]
    fn a_non_finite_return_is_refused() {
        assert_eq!(evaluate(&[1.0, f64::NAN], 1), Err(StatsError::NotFinite));
    }

    #[test]
    fn the_confidence_interval_brackets_the_mean() {
        let stats = evaluate(&sample(55, 45), 3).expect("evaluate");
        assert!(stats.ci_low_r <= stats.expectancy_r);
        assert!(stats.ci_high_r >= stats.expectancy_r);
    }
}
