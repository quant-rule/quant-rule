//! Comparing exit policies on the same entries.
//!
//! The entry is not the interesting decision; the exit is. This module runs
//! several policies over one entry and one stop, so the comparison isolates the
//! exit rule. Everything is in R: the risk is the entry-to-stop distance.
//!
//! The stop is assumed to fill before the target within the same bar. That is
//! the pessimistic order, and it never flatters a policy.

use serde::{Deserialize, Serialize};

use crate::marks::Side;

/// Why an exit cannot be evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitError {
    /// No bars were supplied.
    Empty,
    /// The entry and stop are the same price, or a price is not finite.
    InvalidPrices,
    /// A policy parameter is unusable.
    InvalidPolicy,
}

impl std::fmt::Display for ExitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Empty => "at least one bar is required",
            Self::InvalidPrices => "the entry and the stop must be distinct, finite prices",
            Self::InvalidPolicy => "the policy parameters are invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExitError {}

/// One bar, reduced to what an exit needs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    /// High.
    pub high: f64,
    /// Low.
    pub low: f64,
    /// Close.
    pub close: f64,
}

/// The policies the lab compares.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitPolicy {
    /// Exit at a fixed multiple of the risk.
    Fixed {
        /// The multiple, for example 2.0 for 2R.
        target_r: f64,
    },
    /// Exit at the close of a fixed bar count.
    Time {
        /// Bars held.
        bars: usize,
    },
    /// Take half at the first multiple, the rest at the second.
    Partial {
        /// The first multiple.
        first_r: f64,
        /// The second multiple.
        final_r: f64,
    },
    /// Exit when the close crosses a moving average.
    TrailingEma {
        /// The smoothing period.
        period: usize,
    },
}

/// What one policy returned for one entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExitResult {
    /// The policy name.
    pub policy: String,
    /// The result in R.
    pub r: f64,
    /// Bars held.
    pub bars: usize,
}

/// Run one policy over one entry.
///
/// # Errors
///
/// Returns [`ExitError`] when the bars are empty, the prices are unusable, or a
/// policy parameter is invalid.
pub fn evaluate(
    bars: &[Bar],
    entry: f64,
    stop: f64,
    side: Side,
    policy: ExitPolicy,
) -> Result<ExitResult, ExitError> {
    if bars.is_empty() {
        return Err(ExitError::Empty);
    }
    if !entry.is_finite() || !stop.is_finite() || entry == stop {
        return Err(ExitError::InvalidPrices);
    }
    let risk = (entry - stop).abs();

    match policy {
        ExitPolicy::Fixed { target_r } => {
            if !target_r.is_finite() || target_r <= 0.0 {
                return Err(ExitError::InvalidPolicy);
            }
            let target = if side == Side::Long {
                entry + target_r * risk
            } else {
                entry - target_r * risk
            };
            for (index, bar) in bars.iter().enumerate() {
                if hit_stop(bar, stop, side) {
                    return Ok(result("fixed", -1.0, index + 1));
                }
                if hit_target(bar, target, side) {
                    return Ok(result("fixed", target_r, index + 1));
                }
            }
            Ok(result(
                "fixed",
                close_r(bars.last().expect("not empty"), entry, risk, side),
                bars.len(),
            ))
        }
        ExitPolicy::Time { bars: held } => {
            if held == 0 || held > bars.len() {
                return Err(ExitError::InvalidPolicy);
            }
            let last = &bars[held - 1];
            if hit_stop(last, stop, side) {
                return Ok(result("time", -1.0, held));
            }
            Ok(result("time", close_r(last, entry, risk, side), held))
        }
        ExitPolicy::Partial { first_r, final_r } => {
            if !first_r.is_finite() || !final_r.is_finite() || first_r <= 0.0 || final_r <= first_r
            {
                return Err(ExitError::InvalidPolicy);
            }
            let first = price(entry, risk, first_r, side);
            let last = price(entry, risk, final_r, side);
            let mut booked = false;
            for (index, bar) in bars.iter().enumerate() {
                if hit_stop(bar, stop, side) {
                    return Ok(result(
                        "partial",
                        if booked { -0.5 } else { -1.0 },
                        index + 1,
                    ));
                }
                if !booked && hit_target(bar, first, side) {
                    booked = true;
                }
                if hit_target(bar, last, side) {
                    return Ok(result(
                        "partial",
                        if booked {
                            0.5 * first_r + 0.5 * final_r
                        } else {
                            final_r
                        },
                        index + 1,
                    ));
                }
            }
            let close = close_r(bars.last().expect("not empty"), entry, risk, side);
            Ok(result(
                "partial",
                if booked {
                    0.5 * first_r + 0.5 * close
                } else {
                    close
                },
                bars.len(),
            ))
        }
        ExitPolicy::TrailingEma { period } => {
            if period < 2 || period > bars.len() {
                return Err(ExitError::InvalidPolicy);
            }
            let mut ema = bars[0].close;
            let smoothing = 2.0 / (period as f64 + 1.0);
            for (index, bar) in bars.iter().enumerate() {
                ema = bar.close.mul_add(smoothing, ema * (1.0 - smoothing));
                if index == 0 {
                    continue;
                }
                if hit_stop(bar, stop, side) {
                    return Ok(result("trailing_ema", -1.0, index + 1));
                }
                let crossed = match side {
                    Side::Long => bar.close < ema,
                    Side::Short => bar.close > ema,
                };
                if crossed {
                    return Ok(result(
                        "trailing_ema",
                        close_r(bar, entry, risk, side),
                        index + 1,
                    ));
                }
            }
            Ok(result(
                "trailing_ema",
                close_r(bars.last().expect("not empty"), entry, risk, side),
                bars.len(),
            ))
        }
    }
}

/// Run every policy over the same entry and stop.
///
/// # Errors
///
/// Returns the first [`ExitError`] any policy reports.
pub fn lab(
    bars: &[Bar],
    entry: f64,
    stop: f64,
    side: Side,
    policies: &[ExitPolicy],
) -> Result<Vec<ExitResult>, ExitError> {
    policies
        .iter()
        .map(|policy| evaluate(bars, entry, stop, side, *policy))
        .collect()
}

fn result(policy: &str, r: f64, bars: usize) -> ExitResult {
    ExitResult {
        policy: policy.to_owned(),
        r,
        bars,
    }
}

fn price(entry: f64, risk: f64, multiple: f64, side: Side) -> f64 {
    match side {
        Side::Long => entry + multiple * risk,
        Side::Short => entry - multiple * risk,
    }
}

fn hit_target(bar: &Bar, target: f64, side: Side) -> bool {
    match side {
        Side::Long => bar.high >= target,
        Side::Short => bar.low <= target,
    }
}

fn hit_stop(bar: &Bar, stop: f64, side: Side) -> bool {
    match side {
        Side::Long => bar.low <= stop,
        Side::Short => bar.high >= stop,
    }
}

fn close_r(bar: &Bar, entry: f64, risk: f64, side: Side) -> f64 {
    match side {
        Side::Long => (bar.close - entry) / risk,
        Side::Short => (entry - bar.close) / risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars() -> Vec<Bar> {
        vec![
            Bar {
                high: 101.0,
                low: 99.5,
                close: 100.5,
            },
            Bar {
                high: 103.0,
                low: 100.0,
                close: 102.0,
            },
            Bar {
                high: 105.0,
                low: 102.0,
                close: 104.0,
            },
            Bar {
                high: 106.0,
                low: 103.0,
                close: 105.0,
            },
        ]
    }

    #[test]
    fn a_fixed_target_reaches_two_r() {
        let result = evaluate(
            &bars(),
            100.0,
            99.0,
            Side::Long,
            ExitPolicy::Fixed { target_r: 2.0 },
        )
        .expect("evaluate");
        assert!((result.r - 2.0).abs() < 1e-12);
        assert_eq!(result.bars, 2);
    }

    #[test]
    fn a_stop_is_minus_one_r() {
        let bars = vec![Bar {
            high: 100.5,
            low: 98.5,
            close: 99.0,
        }];
        let result = evaluate(
            &bars,
            100.0,
            99.0,
            Side::Long,
            ExitPolicy::Fixed { target_r: 2.0 },
        )
        .expect("evaluate");
        assert!((result.r + 1.0).abs() < 1e-12);
    }

    #[test]
    fn the_stop_wins_inside_the_same_bar() {
        let bars = vec![Bar {
            high: 110.0,
            low: 98.0,
            close: 100.0,
        }];
        let result = evaluate(
            &bars,
            100.0,
            99.0,
            Side::Long,
            ExitPolicy::Fixed { target_r: 2.0 },
        )
        .expect("evaluate");
        assert!((result.r + 1.0).abs() < 1e-12, "{}", result.r);
    }

    #[test]
    fn a_time_exit_uses_the_close() {
        let result = evaluate(
            &bars(),
            100.0,
            99.0,
            Side::Long,
            ExitPolicy::Time { bars: 2 },
        )
        .expect("evaluate");
        assert!((result.r - 2.0).abs() < 1e-12);
    }

    #[test]
    fn a_partial_books_half_at_the_first_target() {
        let result = evaluate(
            &bars(),
            100.0,
            99.0,
            Side::Long,
            ExitPolicy::Partial {
                first_r: 1.0,
                final_r: 3.0,
            },
        )
        .expect("evaluate");
        assert!((result.r - 2.0).abs() < 1e-12, "{}", result.r);
    }

    #[test]
    fn a_short_reaches_its_target_downwards() {
        let bars = vec![
            Bar {
                high: 100.5,
                low: 99.0,
                close: 99.5,
            },
            Bar {
                high: 99.0,
                low: 97.0,
                close: 98.0,
            },
        ];
        let result = evaluate(
            &bars,
            100.0,
            101.0,
            Side::Short,
            ExitPolicy::Fixed { target_r: 2.0 },
        )
        .expect("evaluate");
        assert!((result.r - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_moving_average_trail_exits_on_a_cross() {
        let bars = vec![
            Bar {
                high: 101.0,
                low: 99.5,
                close: 100.8,
            },
            Bar {
                high: 101.0,
                low: 98.5,
                close: 99.0,
            },
        ];
        let result = evaluate(
            &bars,
            100.0,
            95.0,
            Side::Long,
            ExitPolicy::TrailingEma { period: 2 },
        )
        .expect("evaluate");
        assert_eq!(result.bars, 2);
        assert!(result.r < 0.0);
    }

    #[test]
    fn the_lab_runs_every_policy() {
        let policies = [
            ExitPolicy::Fixed { target_r: 2.0 },
            ExitPolicy::Time { bars: 2 },
        ];
        let results = lab(&bars(), 100.0, 99.0, Side::Long, &policies).expect("lab");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn invalid_inputs_are_refused() {
        assert_eq!(
            evaluate(
                &[],
                100.0,
                99.0,
                Side::Long,
                ExitPolicy::Fixed { target_r: 2.0 }
            ),
            Err(ExitError::Empty)
        );
        assert_eq!(
            evaluate(
                &bars(),
                100.0,
                100.0,
                Side::Long,
                ExitPolicy::Fixed { target_r: 2.0 }
            ),
            Err(ExitError::InvalidPrices)
        );
        assert_eq!(
            evaluate(
                &bars(),
                100.0,
                99.0,
                Side::Long,
                ExitPolicy::Time { bars: 0 }
            ),
            Err(ExitError::InvalidPolicy)
        );
    }
}
