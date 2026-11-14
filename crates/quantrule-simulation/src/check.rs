//! The check.
//!
//! A candidate rule found by marking is tested on a period that was never
//! marked. The check is an event study, not a backtest: it asks whether the
//! filtered events beat an unfiltered control and whether the direction holds
//! across sub-periods.
//!
//! Passing is a screen. It is not a baseline, not an edge, and not evidence that
//! survives out-of-sample validation by itself.

// port-parity: PORT-001

use serde::{Deserialize, Serialize};

/// The pre-registered thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criteria {
    /// Fewer events than this and the sample cannot speak.
    pub min_events: usize,
    /// The filtered mean must be strictly above this, in R.
    pub min_mean_net: f64,
    /// The filtered hit rate must be strictly above this.
    pub min_hit_rate: f64,
    /// Sub-periods the direction is measured across.
    pub half_years: usize,
    /// How many of them must point the same way.
    pub min_positive_half_years: usize,
}

impl Default for Criteria {
    fn default() -> Self {
        Self {
            min_events: 30,
            min_mean_net: 0.0,
            min_hit_rate: 0.5,
            half_years: 5,
            min_positive_half_years: 3,
        }
    }
}

/// One reason a check did not pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    /// The filtered sample was too small.
    TooFewEvents,
    /// The filtered mean did not clear zero.
    MeanNotPositive,
    /// The filtered hit rate was too low.
    HitRateLow,
    /// The filtered mean did not beat the control.
    ControlMeanNotBeaten,
    /// The filtered hit rate did not beat the control.
    ControlHitRateNotBeaten,
    /// The direction did not hold across enough sub-periods.
    Unstable,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::TooFewEvents => "too few filtered events",
            Self::MeanNotPositive => "the filtered mean is not positive",
            Self::HitRateLow => "the filtered hit rate is too low",
            Self::ControlMeanNotBeaten => "the control mean is not beaten",
            Self::ControlHitRateNotBeaten => "the control hit rate is not beaten",
            Self::Unstable => "the direction does not hold across sub-periods",
        };
        formatter.write_str(message)
    }
}

/// What the check measured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    /// Filtered events.
    pub events: usize,
    /// Mean filtered result, in R.
    pub mean_net: f64,
    /// Filtered hit rate.
    pub hit_rate: f64,
    /// Unfiltered events.
    pub control: usize,
    /// Mean unfiltered result, in R.
    pub control_mean_net: f64,
    /// Unfiltered hit rate.
    pub control_hit_rate: f64,
    /// Sub-periods whose mean was positive.
    pub positive_half_years: usize,
    /// Whether every criterion passed.
    pub passed: bool,
    /// Why it did not pass, in criterion order.
    pub failures: Vec<Failure>,
}

/// Run the check.
///
/// `events` and `control` are net results already reduced by the caller's cost
/// proxy; `half_years` is one mean result per sub-period.
///
/// # Errors
///
/// Returns [`CheckError`] when the inputs cannot be evaluated.
pub fn check(
    events: &[f64],
    control: &[f64],
    half_years: &[f64],
    criteria: Criteria,
) -> Result<Check, CheckError> {
    if events
        .iter()
        .chain(control)
        .chain(half_years)
        .any(|value| !value.is_finite())
    {
        return Err(CheckError::NotFinite);
    }
    if half_years.len() != criteria.half_years {
        return Err(CheckError::HalfYearCount);
    }

    let mean_net = mean(events);
    let hit_rate = win_rate(events);
    let control_mean_net = mean(control);
    let control_hit_rate = win_rate(control);
    let positive_half_years = half_years.iter().filter(|value| **value > 0.0).count();

    let mut failures = Vec::new();
    if events.len() < criteria.min_events {
        failures.push(Failure::TooFewEvents);
    }
    if mean_net <= criteria.min_mean_net {
        failures.push(Failure::MeanNotPositive);
    }
    if hit_rate <= criteria.min_hit_rate {
        failures.push(Failure::HitRateLow);
    }
    if !control.is_empty() {
        if mean_net <= control_mean_net {
            failures.push(Failure::ControlMeanNotBeaten);
        }
        if hit_rate <= control_hit_rate {
            failures.push(Failure::ControlHitRateNotBeaten);
        }
    }
    if positive_half_years < criteria.min_positive_half_years {
        failures.push(Failure::Unstable);
    }

    Ok(Check {
        events: events.len(),
        mean_net,
        hit_rate,
        control: control.len(),
        control_mean_net,
        control_hit_rate,
        positive_half_years,
        passed: failures.is_empty(),
        failures,
    })
}

/// Why a check cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// A value was not finite.
    NotFinite,
    /// The sub-period sample did not match the criteria.
    HalfYearCount,
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NotFinite => "a result is not a finite number",
            Self::HalfYearCount => "the sub-period count does not match the criteria",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CheckError {}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    // Sum in value order, not input order. A caller may hand the same set in a
    // different order; the mean must not change because of it.
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    ordered.iter().sum::<f64>() / ordered.len() as f64
}

fn win_rate(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().filter(|value| **value > 0.0).count() as f64 / values.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn winners(count: usize) -> Vec<f64> {
        vec![1.0; count]
    }

    fn control() -> Vec<f64> {
        let mut values = vec![0.1; 20];
        values.extend(vec![-0.1; 20]);
        values
    }

    fn half_years(positive: usize) -> Vec<f64> {
        (0..5)
            .map(|index| if index < positive { 0.5 } else { -0.2 })
            .collect()
    }

    #[test]
    fn a_clean_screen_passes() {
        let result = check(
            &winners(40),
            &control(),
            &half_years(4),
            Criteria::default(),
        )
        .expect("check");
        assert!(result.passed, "{:?}", result.failures);
        assert_eq!(result.events, 40);
    }

    #[test]
    fn too_few_events_fails() {
        let result = check(
            &winners(10),
            &[0.1; 10],
            &half_years(5),
            Criteria::default(),
        )
        .expect("check");
        assert!(result.failures.contains(&Failure::TooFewEvents));
    }

    #[test]
    fn a_mean_at_zero_fails() {
        let events = vec![1.0, -1.0];
        let result = check(&events, &[], &half_years(5), Criteria::default()).expect("check");
        assert!(result.failures.contains(&Failure::MeanNotPositive));
    }

    #[test]
    fn a_low_hit_rate_fails() {
        let events = vec![3.0, -1.0, -1.0, -1.0];
        let result = check(&events, &[], &half_years(5), Criteria::default()).expect("check");
        assert!(result.failures.contains(&Failure::HitRateLow));
    }

    #[test]
    fn a_control_that_is_not_beaten_fails() {
        let result = check(
            &winners(40),
            &winners(40),
            &half_years(5),
            Criteria::default(),
        )
        .expect("check");
        assert!(result.failures.contains(&Failure::ControlMeanNotBeaten));
        assert!(result.failures.contains(&Failure::ControlHitRateNotBeaten));
    }

    #[test]
    fn an_unstable_direction_fails() {
        let result = check(
            &winners(40),
            &control(),
            &half_years(2),
            Criteria::default(),
        )
        .expect("check");
        assert!(result.failures.contains(&Failure::Unstable));
    }

    #[test]
    fn reordering_the_values_does_not_change_the_result() {
        let events = [1.0, 0.25, 0.5, 0.125];
        let control = [0.1, -0.1, 0.05, -0.05];
        let first = check(&events, &control, &half_years(4), Criteria::default()).expect("first");
        let mut reversed_events = events;
        reversed_events.reverse();
        let mut reversed_control = control;
        reversed_control.reverse();
        let second = check(
            &reversed_events,
            &reversed_control,
            &half_years(4),
            Criteria::default(),
        )
        .expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn an_unmarked_check_set_reports_too_few_events() {
        let criteria = Criteria {
            half_years: 0,
            ..Criteria::default()
        };
        let result = check(&[], &[0.1], &[], criteria).expect("check");
        assert_eq!(result.events, 0);
        assert_eq!(result.mean_net, 0.0);
        assert_eq!(result.hit_rate, 0.0);
        assert_eq!(result.control, 1);
        assert!(result.failures.contains(&Failure::TooFewEvents));
        assert!(!result.passed);
    }

    #[test]
    fn the_sub_period_count_must_match() {
        assert_eq!(
            check(&winners(40), &[], &[0.1, 0.1], Criteria::default()),
            Err(CheckError::HalfYearCount)
        );
    }

    #[test]
    fn a_check_round_trips_through_json() {
        let result = check(
            &winners(40),
            &control(),
            &half_years(4),
            Criteria::default(),
        )
        .expect("check");
        let text = serde_json::to_string(&result).expect("encode");
        let back: Check = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, result);
    }
}
