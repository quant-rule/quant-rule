//! Comparing experiments.
//!
//! Research advances by comparing what was tried. One experiment is the
//! baseline; every other is reported against it, with its own sample size and
//! significance, so a smaller drawdown is never mistaken for a better edge.

use serde::{Deserialize, Serialize};

use crate::costs::CostModel;
use crate::stats::Statistics;
use crate::{EvaluationError, evaluate_net};

/// Why experiments cannot be compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    /// No experiment was supplied.
    Empty,
    /// Two experiments share an id.
    DuplicateId,
    /// An experiment could not be evaluated.
    Evaluation(EvaluationError),
}

impl std::fmt::Display for CompareError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a comparison needs at least one experiment"),
            Self::DuplicateId => formatter.write_str("two experiments share an id"),
            Self::Evaluation(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CompareError {}

/// One evaluated hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Experiment {
    /// A stable label, for example `h3-cross-section`.
    pub id: String,
    /// The seed that produced the bootstrap numbers.
    pub seed: u64,
    /// The round-trip cost in basis points.
    pub cost_bps: f64,
    /// What the sample measured.
    pub stats: Statistics,
}

impl Experiment {
    /// Evaluate one sample and build the record.
    ///
    /// # Errors
    ///
    /// Returns [`CompareError::Evaluation`] when the sample or the costs are
    /// invalid.
    pub fn run(
        id: impl Into<String>,
        gross: &[f64],
        model: &CostModel,
        entry_price: f64,
        risk_distance: f64,
        seed: u64,
    ) -> Result<Self, CompareError> {
        let stats = evaluate_net(gross, model, entry_price, risk_distance, seed)
            .map_err(CompareError::Evaluation)?;
        Ok(Self {
            id: id.into(),
            seed,
            cost_bps: model
                .round_trip_bps()
                .map_err(|error| CompareError::Evaluation(EvaluationError::Cost(error)))?,
            stats,
        })
    }

    /// Whether another experiment reproduced this one exactly.
    ///
    /// Two runs over the same sample, the same costs, and the same seed are the
    /// same evidence; anything else is a different experiment.
    #[must_use]
    pub fn reproduces(&self, other: &Self) -> bool {
        self.seed == other.seed && self.cost_bps == other.cost_bps && self.stats == other.stats
    }
}

/// One row of the comparison table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonRow {
    /// The experiment label.
    pub id: String,
    /// Its expectancy in R.
    pub expectancy_r: f64,
    /// Expectancy minus the baseline's, in R.
    pub delta_expectancy_r: f64,
    /// Its maximum drawdown in R.
    pub max_drawdown_r: f64,
    /// Drawdown minus the baseline's, in R.
    pub delta_drawdown_r: f64,
    /// Its sample size.
    pub trades: usize,
    /// Its p-value.
    pub p_value: f64,
    /// Whether it cleared the significance threshold.
    pub significant: bool,
}

/// A baseline and the experiments measured against it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    /// The baseline id.
    pub baseline: String,
    /// One row per experiment, baseline first.
    pub rows: Vec<ComparisonRow>,
}

/// Compare a baseline with the candidates.
///
/// # Errors
///
/// Returns [`CompareError::Empty`] when either side is empty and
/// [`CompareError::DuplicateId`] when an id repeats.
pub fn compare(
    baseline: &Experiment,
    candidates: &[Experiment],
) -> Result<Comparison, CompareError> {
    if candidates.is_empty() {
        return Err(CompareError::Empty);
    }

    let mut seen = std::collections::BTreeSet::new();
    seen.insert(baseline.id.as_str());
    for candidate in candidates {
        if !seen.insert(candidate.id.as_str()) {
            return Err(CompareError::DuplicateId);
        }
    }

    let row = |experiment: &Experiment| ComparisonRow {
        id: experiment.id.clone(),
        expectancy_r: experiment.stats.expectancy_r,
        delta_expectancy_r: experiment.stats.expectancy_r - baseline.stats.expectancy_r,
        max_drawdown_r: experiment.stats.max_drawdown_r,
        delta_drawdown_r: experiment.stats.max_drawdown_r - baseline.stats.max_drawdown_r,
        trades: experiment.stats.trades,
        p_value: experiment.stats.p_value,
        significant: experiment.stats.significant,
    };

    let mut rows = vec![row(baseline)];
    rows.extend(candidates.iter().map(row));
    Ok(Comparison {
        baseline: baseline.id.clone(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(winning: usize, losing: usize) -> Vec<f64> {
        let mut values = vec![3.0; winning];
        values.extend(vec![-1.0; losing]);
        values
    }

    fn free() -> CostModel {
        CostModel::default()
    }

    #[test]
    fn the_baseline_is_the_first_row_with_zero_delta() {
        let baseline = Experiment::run("a", &sample(50, 50), &free(), 100.0, 1.0, 7).expect("a");
        let other = Experiment::run("b", &sample(55, 45), &free(), 100.0, 1.0, 7).expect("b");
        let comparison = compare(&baseline, &[other]).expect("compare");
        assert_eq!(comparison.rows[0].id, "a");
        assert!((comparison.rows[0].delta_expectancy_r).abs() < 1e-12);
        assert!((comparison.rows[1].delta_expectancy_r - 0.2).abs() < 1e-12);
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let baseline = Experiment::run("a", &sample(50, 50), &free(), 100.0, 1.0, 7).expect("a");
        let other = Experiment::run("a", &sample(55, 45), &free(), 100.0, 1.0, 7).expect("b");
        assert_eq!(compare(&baseline, &[other]), Err(CompareError::DuplicateId));
    }

    #[test]
    fn an_empty_comparison_is_refused() {
        let baseline = Experiment::run("a", &sample(50, 50), &free(), 100.0, 1.0, 7).expect("a");
        assert_eq!(compare(&baseline, &[]), Err(CompareError::Empty));
    }

    #[test]
    fn a_rerun_reproduces_the_same_experiment() {
        let first = Experiment::run("a", &sample(55, 45), &free(), 100.0, 1.0, 7).expect("first");
        let second = Experiment::run("a", &sample(55, 45), &free(), 100.0, 1.0, 7).expect("second");
        assert!(first.reproduces(&second));
    }

    #[test]
    fn a_different_seed_is_not_a_reproduction() {
        let first = Experiment::run("a", &sample(55, 45), &free(), 100.0, 1.0, 7).expect("first");
        let second = Experiment::run("a", &sample(55, 45), &free(), 100.0, 1.0, 8).expect("second");
        assert!(!first.reproduces(&second));
    }

    #[test]
    fn costs_change_the_row() {
        let model = CostModel::new(2.0, 1.0, 2.0).expect("model");
        let paid = Experiment::run("p", &sample(55, 45), &model, 100.0, 1.0, 7).expect("paid");
        assert!((paid.cost_bps - 10.0).abs() < 1e-12);
        assert!(paid.stats.expectancy_r < 1.2);
    }

    #[test]
    fn a_comparison_round_trips_through_json() {
        let baseline = Experiment::run("a", &sample(50, 50), &free(), 100.0, 1.0, 7).expect("a");
        let other = Experiment::run("b", &sample(55, 45), &free(), 100.0, 1.0, 7).expect("b");
        let comparison = compare(&baseline, &[other]).expect("compare");
        let text = serde_json::to_string(&comparison).expect("encode");
        let back: Comparison = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, comparison);
    }
}
