//! Exploration: does a condition separate outcomes?
//!
//! This is the primitive behind R2. The caller supplies, for every bar, the
//! forward outcome it cares about and whether the candidate signal fired. The
//! study reports how the two groups differ and whether the difference could be
//! noise.
//!
//! It is a screen, not evidence: a candidate that passes still has to become a
//! sealed hypothesis and face out-of-sample validation.

use serde::{Deserialize, Serialize};

use crate::rng::Pcg32;

/// How many bootstrap resamples one study uses.
pub const RESAMPLES: u32 = 5_000;

/// Why a study cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExploreError {
    /// The samples differ in length or are empty.
    Mismatched,
    /// The horizon is zero.
    EmptyHorizon,
    /// The signal never fired.
    NoTreatment,
    /// The signal always fired, leaving no control.
    NoControl,
    /// An outcome was not a finite number.
    NotFinite,
}

impl std::fmt::Display for ExploreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Mismatched => "the outcomes and the signal must be the same non-zero length",
            Self::EmptyHorizon => "the horizon must be at least one bar",
            Self::NoTreatment => "the signal never fired",
            Self::NoControl => "the signal always fired, so there is no control group",
            Self::NotFinite => "an outcome is not a finite number",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExploreError {}

/// The result of one exploratory comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventStudy {
    /// Bars between the signal and the outcome.
    pub horizon: usize,
    /// Bars where the signal fired.
    pub treatment: usize,
    /// Bars where it did not.
    pub control: usize,
    /// Mean outcome when the signal fired.
    pub treatment_mean: f64,
    /// Mean outcome when it did not.
    pub control_mean: f64,
    /// Treatment mean minus control mean.
    pub difference: f64,
    /// One-sided bootstrap probability of a difference at or below zero.
    pub p_value: f64,
    /// Whether the difference cleared the five percent threshold.
    pub significant: bool,
}

/// Compare the outcomes of the signal against the outcomes of its absence.
///
/// # Errors
///
/// Returns [`ExploreError`] when the inputs are empty, differ in length, contain
/// a non-finite outcome, have no horizon, or leave one group empty.
pub fn study(
    outcomes: &[f64],
    signal: &[bool],
    horizon: usize,
    seed: u64,
) -> Result<EventStudy, ExploreError> {
    if outcomes.is_empty() || outcomes.len() != signal.len() {
        return Err(ExploreError::Mismatched);
    }
    if horizon == 0 {
        return Err(ExploreError::EmptyHorizon);
    }
    if outcomes.iter().any(|value| !value.is_finite()) {
        return Err(ExploreError::NotFinite);
    }

    let treatment: Vec<f64> = outcomes
        .iter()
        .zip(signal)
        .filter(|(_, fired)| **fired)
        .map(|(value, _)| *value)
        .collect();
    let control: Vec<f64> = outcomes
        .iter()
        .zip(signal)
        .filter(|(_, fired)| !**fired)
        .map(|(value, _)| *value)
        .collect();

    if treatment.is_empty() {
        return Err(ExploreError::NoTreatment);
    }
    if control.is_empty() {
        return Err(ExploreError::NoControl);
    }

    let treatment_mean = mean(&treatment);
    let control_mean = mean(&control);
    let difference = treatment_mean - control_mean;

    let mut generator = Pcg32::new(seed);
    let mut at_or_below_zero = 0_usize;
    for _ in 0..RESAMPLES {
        let mut treatment_sum = 0.0;
        for _ in 0..treatment.len() {
            treatment_sum += treatment[generator.below(treatment.len())];
        }
        let mut control_sum = 0.0;
        for _ in 0..control.len() {
            control_sum += control[generator.below(control.len())];
        }
        let resampled = treatment_sum / treatment.len() as f64 - control_sum / control.len() as f64;
        if resampled <= 0.0 {
            at_or_below_zero += 1;
        }
    }
    let p_value = (at_or_below_zero as f64 + 1.0) / (f64::from(RESAMPLES) + 1.0);

    Ok(EventStudy {
        horizon,
        treatment: treatment.len(),
        control: control.len(),
        treatment_mean,
        control_mean,
        difference,
        p_value,
        significant: p_value < 0.05,
    })
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signal_that_marks_the_good_bars_is_significant() {
        let outcomes = [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let signal = [true, true, true, false, false, false, false, false];
        let study = study(&outcomes, &signal, 5, 7).expect("study");
        assert_eq!(study.treatment, 3);
        assert_eq!(study.control, 5);
        assert!((study.difference - 1.0).abs() < 1e-12);
        assert!(study.significant, "p={}", study.p_value);
    }

    #[test]
    fn a_signal_with_no_separation_is_not_significant() {
        // Both groups average 0.5: the condition says nothing about the outcome.
        let outcomes = [1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0];
        let signal = [true, true, true, true, false, false, false, false];
        let study = study(&outcomes, &signal, 5, 7).expect("study");
        assert!((study.difference).abs() < 1e-12);
        assert!(!study.significant, "p={}", study.p_value);
    }

    #[test]
    fn the_same_seed_gives_the_same_p_value() {
        let outcomes = [1.0, 2.0, 0.0, 1.0, -1.0, 0.0];
        let signal = [true, true, false, false, false, false];
        let first = study(&outcomes, &signal, 3, 9).expect("first");
        let second = study(&outcomes, &signal, 3, 9).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn mismatched_inputs_are_refused() {
        assert_eq!(
            study(&[1.0], &[true, false], 1, 1),
            Err(ExploreError::Mismatched)
        );
        assert_eq!(study(&[], &[], 1, 1), Err(ExploreError::Mismatched));
    }

    #[test]
    fn a_zero_horizon_is_refused() {
        assert_eq!(
            study(&[1.0], &[true], 0, 1),
            Err(ExploreError::EmptyHorizon)
        );
    }

    #[test]
    fn an_empty_group_is_refused() {
        assert_eq!(
            study(&[1.0, 2.0], &[false, false], 1, 1),
            Err(ExploreError::NoTreatment)
        );
        assert_eq!(
            study(&[1.0, 2.0], &[true, true], 1, 1),
            Err(ExploreError::NoControl)
        );
    }

    #[test]
    fn a_study_round_trips_through_json() {
        let study = study(&[1.0, 0.0, 1.0, 0.0], &[true, false, true, false], 5, 7).expect("study");
        let text = serde_json::to_string(&study).expect("encode");
        let back: EventStudy = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, study);
    }
}
