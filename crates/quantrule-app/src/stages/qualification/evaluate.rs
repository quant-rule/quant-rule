//! The pure verdict evaluator.
//!
//! The evaluator receives the parsed criteria and one resolved observation per
//! criterion. It applies the operator, the partition and sample gates, and the
//! fixed verdict precedence. It reads no file, clock, or environment.
//!
//! Precedence is `fail` over `insufficient_evidence` over `pass`. A missing or
//! non-numeric measurement is `insufficient_evidence`, never zero and never a
//! pass. A valid zero is a number and is compared as one.

use serde_json::Value;

use crate::config::{Criteria, Criterion};

/// One criterion's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The evidence exists, the metric is defined, and the comparison holds.
    Pass,
    /// The evidence exists, the metric is defined, and the comparison fails.
    Fail,
    /// The evidence is missing, undefined, outside the required partition, or
    /// below the declared sample floor.
    InsufficientEvidence,
}

impl Status {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::InsufficientEvidence => "insufficient_evidence",
        }
    }
}

/// A stable reason code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The operator and threshold were satisfied.
    OperatorSatisfied,
    /// The operator and threshold were not satisfied.
    OperatorNotSatisfied,
    /// The field is absent or null.
    MissingMeasurement,
    /// The field is present but is not a number.
    NonNumericMeasurement,
    /// The evidence does not overlap a required partition.
    RequiredPartitionMissing,
    /// The closed-trade count is below the declared minimum.
    MinimumTradesNotMet,
    /// The closed-trade count is unavailable.
    TradeCountUnavailable,
}

impl Reason {
    /// The stable wire code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::OperatorSatisfied => "operator_satisfied",
            Self::OperatorNotSatisfied => "operator_not_satisfied",
            Self::MissingMeasurement => "missing_measurement",
            Self::NonNumericMeasurement => "non_numeric_measurement",
            Self::RequiredPartitionMissing => "required_partition_missing",
            Self::MinimumTradesNotMet => "minimum_trades_not_met",
            Self::TradeCountUnavailable => "trade_count_unavailable",
        }
    }
}

/// A resolved evidence field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resolved {
    /// A finite JSON number.
    Number(f64),
    /// The field exists and is null.
    Null,
    /// The field or an intermediate path is absent.
    Missing,
    /// The field exists but is not a number.
    NonNumeric,
}

/// Resolves a dotted field path inside one artifact document.
///
/// An object segment is a key; an all-digit segment indexes an array. A path
/// that crosses a scalar, misses a key, or names an index outside the array is
/// [`Resolved::Missing`]. A present null is [`Resolved::Null`]. Every other
/// non-number is [`Resolved::NonNumeric`].
#[must_use]
pub fn resolve(document: &Value, field: &str) -> Resolved {
    let mut current = document;
    for segment in field.split('.') {
        current = match current {
            Value::Object(object) => match object.get(segment) {
                Some(value) => value,
                None => return Resolved::Missing,
            },
            Value::Array(values) => {
                let Ok(index) = segment.parse::<usize>() else {
                    return Resolved::Missing;
                };
                match values.get(index) {
                    Some(value) => value,
                    None => return Resolved::Missing,
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
                return Resolved::Missing;
            }
        };
    }
    match current {
        Value::Number(number) => match number.as_f64() {
            Some(value) if value.is_finite() => Resolved::Number(value),
            _ => Resolved::NonNumeric,
        },
        Value::Null => Resolved::Null,
        _ => Resolved::NonNumeric,
    }
}

/// One criterion's evaluated result.
#[derive(Debug, Clone)]
pub struct Outcome<'a> {
    /// The criterion.
    pub criterion: &'a Criterion,
    /// Its status.
    pub status: Status,
    /// The observed number, or `None` when the measurement is missing or not
    /// numeric.
    pub observed: Option<f64>,
    /// The stable reason code.
    pub reason: Reason,
}

/// The complete evaluation.
#[derive(Debug, Clone)]
pub struct Verdict<'a> {
    /// `pass`, `fail`, or `insufficient_evidence`.
    pub status: Status,
    /// One outcome per criterion, in criteria order.
    pub outcomes: Vec<Outcome<'a>>,
}

impl Verdict<'_> {
    /// The count of one status.
    #[must_use]
    pub fn count(&self, status: Status) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| outcome.status == status)
            .count()
    }
}

/// Evaluates every criterion and computes the verdict.
///
/// `observations` must have one entry per criterion, in the same order.
#[must_use]
pub fn evaluate<'a>(
    criteria: &'a Criteria,
    observations: &[Resolved],
    partitions_covered: bool,
    observed_trades: Option<u64>,
) -> Verdict<'a> {
    let mut outcomes = Vec::with_capacity(criteria.criteria.len());
    for (criterion, observation) in criteria.criteria.iter().zip(observations) {
        outcomes.push(evaluate_one(
            criterion,
            *observation,
            partitions_covered,
            observed_trades,
            criteria.minimum_trades,
        ));
    }

    let mut any_fail = false;
    let mut any_insufficient = false;
    for outcome in &outcomes {
        if !outcome.criterion.mandatory {
            continue;
        }
        match outcome.status {
            Status::Fail => any_fail = true,
            Status::InsufficientEvidence => any_insufficient = true,
            Status::Pass => {}
        }
    }
    let status = if any_fail {
        Status::Fail
    } else if any_insufficient {
        Status::InsufficientEvidence
    } else {
        Status::Pass
    };

    Verdict { status, outcomes }
}

fn evaluate_one(
    criterion: &Criterion,
    observation: Resolved,
    partitions_covered: bool,
    observed_trades: Option<u64>,
    minimum_trades: u64,
) -> Outcome<'_> {
    let (mut status, observed, mut reason) = match observation {
        Resolved::Number(value) => {
            let pass = criterion.operator.evaluate(value, criterion.threshold);
            (
                if pass { Status::Pass } else { Status::Fail },
                Some(value),
                if pass {
                    Reason::OperatorSatisfied
                } else {
                    Reason::OperatorNotSatisfied
                },
            )
        }
        Resolved::Null | Resolved::Missing => (
            Status::InsufficientEvidence,
            None,
            Reason::MissingMeasurement,
        ),
        Resolved::NonNumeric => (
            Status::InsufficientEvidence,
            None,
            Reason::NonNumericMeasurement,
        ),
    };

    // The gates make a pass conditional on the evidence being from a required
    // partition and meeting the declared sample floor. A fail is left intact so
    // the documented `fail` precedence survives a gate.
    let gate = if !partitions_covered {
        Some(Reason::RequiredPartitionMissing)
    } else if observed_trades.is_none() {
        Some(Reason::TradeCountUnavailable)
    } else if observed_trades.is_some_and(|trades| trades < minimum_trades) {
        Some(Reason::MinimumTradesNotMet)
    } else {
        None
    };
    if let Some(gate) = gate
        && status == Status::Pass
    {
        status = Status::InsufficientEvidence;
        reason = gate;
    }

    Outcome {
        criterion,
        status,
        observed,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Reason, Resolved, Status, evaluate, resolve};
    use crate::config::{Criteria, Criterion, Dimension, EvidenceArtifact, Operator};

    fn criterion(id: &str, mandatory: bool, operator: Operator, threshold: f64) -> Criterion {
        Criterion {
            id: id.to_owned(),
            dimension: Dimension::Drawdown,
            mandatory,
            metric: id.to_owned(),
            operator,
            threshold,
            evidence_artifact: EvidenceArtifact::Portfolio,
            evidence_field: "metrics.value".to_owned(),
        }
    }

    fn criteria(entries: Vec<Criterion>, minimum_trades: u64) -> Criteria {
        Criteria {
            id: "Q-TEST".to_owned(),
            required_partitions: Vec::new(),
            minimum_trades,
            cost_model: crate::config::CostModel::ExecutionAndPortfolio,
            multiple_testing: crate::config::MultipleTesting {
                primary_cells: 1,
                trials: 1,
                correction: crate::config::Correction::None,
                family_alpha: 0.05,
            },
            criteria: entries,
            not_applicable: Vec::new(),
        }
    }

    fn evaluate_values(
        entries: Vec<Criterion>,
        observations: &[Resolved],
        minimum_trades: u64,
    ) -> super::Verdict<'static> {
        let leaked: &'static Criteria = Box::leak(Box::new(criteria(entries, minimum_trades)));
        evaluate(leaked, observations, true, Some(10))
    }

    #[test]
    fn all_mandatory_criteria_pass() {
        let verdict = evaluate_values(
            vec![
                criterion("a", true, Operator::Greater, 0.0),
                criterion("b", true, Operator::LessOrEqual, 1.0),
            ],
            &[Resolved::Number(1.0), Resolved::Number(0.5)],
            0,
        );
        assert_eq!(verdict.status, Status::Pass);
        assert_eq!(verdict.count(Status::Pass), 2);
    }

    #[test]
    fn one_mandatory_fail_fails_the_verdict() {
        let verdict = evaluate_values(
            vec![
                criterion("a", true, Operator::Greater, 0.0),
                criterion("b", true, Operator::Greater, 0.0),
            ],
            &[Resolved::Number(1.0), Resolved::Number(0.0)],
            0,
        );
        assert_eq!(verdict.status, Status::Fail);
        assert_eq!(verdict.outcomes[1].reason, Reason::OperatorNotSatisfied);
    }

    #[test]
    fn one_missing_mandatory_criterion_is_insufficient() {
        let verdict = evaluate_values(
            vec![criterion("a", true, Operator::Greater, 0.0)],
            &[Resolved::Missing],
            0,
        );
        assert_eq!(verdict.status, Status::InsufficientEvidence);
        assert_eq!(verdict.outcomes[0].observed, None);
        assert_eq!(verdict.outcomes[0].reason, Reason::MissingMeasurement);
    }

    #[test]
    fn a_null_measurement_is_insufficient_not_zero() {
        let verdict = evaluate_values(
            vec![criterion("a", true, Operator::GreaterOrEqual, 0.0)],
            &[Resolved::Null],
            0,
        );
        assert_eq!(verdict.status, Status::InsufficientEvidence);
        assert_eq!(verdict.outcomes[0].observed, None);
    }

    #[test]
    fn a_valid_zero_is_a_value() {
        let verdict = evaluate_values(
            vec![criterion("a", true, Operator::Equal, 0.0)],
            &[Resolved::Number(0.0)],
            0,
        );
        assert_eq!(verdict.status, Status::Pass);
        assert_eq!(verdict.outcomes[0].observed, Some(0.0));
    }

    #[test]
    fn fail_takes_precedence_over_insufficient_evidence() {
        let verdict = evaluate_values(
            vec![
                criterion("fail", true, Operator::Greater, 0.0),
                criterion("missing", true, Operator::Greater, 0.0),
            ],
            &[Resolved::Number(0.0), Resolved::Missing],
            0,
        );
        assert_eq!(verdict.status, Status::Fail);
        assert_eq!(verdict.count(Status::Fail), 1);
        assert_eq!(verdict.count(Status::InsufficientEvidence), 1);
    }

    #[test]
    fn a_diagnostic_failure_does_not_change_the_verdict() {
        let verdict = evaluate_values(
            vec![
                criterion("mandatory", true, Operator::Greater, 0.0),
                criterion("diagnostic", false, Operator::Greater, 0.0),
            ],
            &[Resolved::Number(1.0), Resolved::Number(0.0)],
            0,
        );
        assert_eq!(verdict.status, Status::Pass);
        assert_eq!(verdict.outcomes[1].status, Status::Fail);
    }

    #[test]
    fn a_missing_required_partition_downgrades_a_pass_but_not_a_fail() {
        let criteria = Box::leak(Box::new(criteria(
            vec![
                criterion("pass", true, Operator::Greater, 0.0),
                criterion("fail", true, Operator::Greater, 2.0),
            ],
            0,
        )));
        let verdict = evaluate(
            criteria,
            &[Resolved::Number(1.0), Resolved::Number(1.0)],
            false,
            Some(10),
        );
        assert_eq!(verdict.status, Status::Fail);
        assert_eq!(verdict.outcomes[0].reason, Reason::RequiredPartitionMissing);
        assert_eq!(verdict.outcomes[0].status, Status::InsufficientEvidence);
        assert_eq!(verdict.outcomes[1].reason, Reason::OperatorNotSatisfied);
    }

    #[test]
    fn a_sample_floor_downgrades_a_pass() {
        let criteria = Box::leak(Box::new(criteria(
            vec![criterion("pass", true, Operator::Greater, 0.0)],
            10,
        )));
        let verdict = evaluate(criteria, &[Resolved::Number(1.0)], true, Some(9));
        assert_eq!(verdict.status, Status::InsufficientEvidence);
        assert_eq!(verdict.outcomes[0].reason, Reason::MinimumTradesNotMet);
    }

    #[test]
    fn the_sample_floor_is_inclusive() {
        let criteria = Box::leak(Box::new(criteria(
            vec![criterion("pass", true, Operator::Greater, 0.0)],
            10,
        )));
        let verdict = evaluate(criteria, &[Resolved::Number(1.0)], true, Some(10));
        assert_eq!(verdict.status, Status::Pass);
    }

    #[test]
    fn a_missing_trade_count_downgrades_a_pass() {
        let criteria = Box::leak(Box::new(criteria(
            vec![criterion("pass", true, Operator::Greater, 0.0)],
            0,
        )));
        let verdict = evaluate(criteria, &[Resolved::Number(1.0)], true, None);
        assert_eq!(verdict.status, Status::InsufficientEvidence);
        assert_eq!(verdict.outcomes[0].reason, Reason::TradeCountUnavailable);
    }

    #[test]
    fn a_number_resolves_through_objects_and_arrays() {
        let document = json!({"metrics": {"monthly": [{"return_fraction": 0.25}]}});
        assert_eq!(
            resolve(&document, "metrics.monthly.0.return_fraction"),
            Resolved::Number(0.25)
        );
    }

    #[test]
    fn a_missing_path_resolves_to_missing() {
        let document = json!({"metrics": {"value": 1.0}});
        assert_eq!(resolve(&document, "metrics.absent"), Resolved::Missing);
        assert_eq!(
            resolve(&document, "metrics.value.absent"),
            Resolved::Missing
        );
        assert_eq!(resolve(&document, "metrics.monthly.0"), Resolved::Missing);
    }

    #[test]
    fn a_null_resolves_to_null_and_a_string_to_non_numeric() {
        let document = json!({"metrics": {"value": null, "label": "x"}});
        assert_eq!(resolve(&document, "metrics.value"), Resolved::Null);
        assert_eq!(resolve(&document, "metrics.label"), Resolved::NonNumeric);
    }
}
