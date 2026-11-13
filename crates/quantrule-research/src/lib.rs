//! Hypothesis evaluation.
//!
//! This crate answers the kata's core question: is an apparent advantage larger
//! than what randomness would produce? It takes the trade returns of one rule,
//! applies the cost model that produced them, and reports the measures a
//! researcher needs to accept, revise, or stop.
//!
//! It is pure: no I/O, no clock. The resampling seed is a parameter, so the same
//! trades and the same seed give the same numbers on every machine.

#![forbid(unsafe_code)]

pub mod compare;
pub mod costs;
pub mod rng;
pub mod split;
pub mod stats;

pub use compare::{CompareError, Comparison, ComparisonRow, Experiment, compare};
pub use costs::{CostError, CostModel};
pub use rng::Pcg32;
pub use split::{Partition, Sample, Split, SplitError, ValidationLedger};
pub use stats::{Statistics, StatsError, evaluate};

/// Why an evaluation could not be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    /// The cost model or its inputs were invalid.
    Cost(CostError),
    /// The sample could not be evaluated.
    Stats(StatsError),
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cost(error) => error.fmt(formatter),
            Self::Stats(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EvaluationError {}

/// Evaluate a gross sample after applying the cost model.
///
/// This is the entry point a caller should use: a result that skipped its costs
/// would overstate the edge.
///
/// # Errors
///
/// Returns [`EvaluationError`] when the costs or the sample are invalid.
pub fn evaluate_net(
    gross: &[f64],
    model: &CostModel,
    entry_price: f64,
    risk_distance: f64,
    seed: u64,
) -> Result<Statistics, EvaluationError> {
    let net = model
        .net_series(gross, entry_price, risk_distance)
        .map_err(EvaluationError::Cost)?;
    evaluate(&net, seed).map_err(EvaluationError::Stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gross() -> Vec<f64> {
        let mut values = vec![3.0; 60];
        values.extend(vec![-1.0; 40]);
        values
    }

    #[test]
    fn costs_reduce_the_expectancy() {
        let free = evaluate_net(&gross(), &CostModel::default(), 100.0, 1.0, 7).expect("free");
        let paid = evaluate_net(
            &gross(),
            &CostModel::new(2.0, 1.0, 2.0).expect("model"),
            100.0,
            1.0,
            7,
        )
        .expect("paid");
        assert!(paid.expectancy_r < free.expectancy_r);
        assert!((free.expectancy_r - 1.4).abs() < 1e-12);
        assert!((paid.expectancy_r - 1.3).abs() < 1e-12);
    }

    #[test]
    fn a_bad_cost_input_is_reported() {
        let model = CostModel::new(1.0, 0.0, 0.0).expect("model");
        assert!(matches!(
            evaluate_net(&gross(), &model, 0.0, 1.0, 7),
            Err(EvaluationError::Cost(CostError::InvalidPrice))
        ));
    }
}
