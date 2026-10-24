//! The shared numeric kernel of the model engine.
//!
//! The kernel owns the definitions every axis, event, and aggregate uses: the
//! unavailable-reason and readiness registries, the contract's fixed clamp, the
//! sign operation, the compact-state bucketing, the empirical percentile, and
//! the per-axis state counters. It knows nothing about Direction, Momentum,
//! Structure, Progression, events, timeframes, or candles, and it performs no
//! I/O, reads no clock, and uses no randomness.
//!
//! Missing measurements are represented by [`Option`]. No helper in this module
//! returns zero to mean "unavailable": a zero from a helper is a measured zero.

/// The number of valid magnitudes the empirical percentile may use.
pub const PERCENTILE_WINDOW: usize = 252;

/// The minimum number of valid magnitudes the empirical percentile needs.
pub const PERCENTILE_MIN_SAMPLES: usize = 50;

/// The number of code-bearing closed bars in a state-change window.
pub const STATE_CHANGE_WINDOW: usize = 50;

/// The threshold that separates the compact states.
///
/// A value strictly below `-0.1` is state -1, a value from `-0.1` through
/// `+0.1` inclusive is state 0, and a value strictly above `+0.1` is state +1.
pub const COMPACT_STATE_THRESHOLD: f64 = 0.1;

/// Why an axis cannot publish a measurement.
///
/// The codes are the contract's code registry. A code is stable once
/// published; a retired code is not reused.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    /// Code 1: there are not enough closed bars.
    InsufficientBars = 1,
    /// Code 2: there are not enough confirmed pivots.
    InsufficientConfirmedPivots = 2,
    /// Code 3: a denominator is zero, or a progression target degenerates
    /// against its origin.
    InvalidDenominator = 3,
    /// Code 4: there is no usable target.
    NoTarget = 4,
    /// Code 5: the target is not stable.
    TargetUnstable = 5,
    /// Code 6: the origin is not confirmed.
    OriginUnconfirmed = 6,
}

impl UnavailableReason {
    /// Every unavailable reason, in code order.
    pub const ALL: [Self; 6] = [
        Self::InsufficientBars,
        Self::InsufficientConfirmedPivots,
        Self::InvalidDenominator,
        Self::NoTarget,
        Self::TargetUnstable,
        Self::OriginUnconfirmed,
    ];

    /// Returns the registry code.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::kernel::UnavailableReason;
    ///
    /// assert_eq!(UnavailableReason::InsufficientBars.code(), 1);
    /// assert_eq!(UnavailableReason::OriginUnconfirmed.code(), 6);
    /// ```
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Resolves a registry code.
    ///
    /// Returns `None` for a code the registry does not define.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::kernel::UnavailableReason;
    ///
    /// assert_eq!(
    ///     UnavailableReason::from_code(3),
    ///     Some(UnavailableReason::InvalidDenominator),
    /// );
    /// assert_eq!(UnavailableReason::from_code(0), None);
    /// ```
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::InsufficientBars),
            2 => Some(Self::InsufficientConfirmedPivots),
            3 => Some(Self::InvalidDenominator),
            4 => Some(Self::NoTarget),
            5 => Some(Self::TargetUnstable),
            6 => Some(Self::OriginUnconfirmed),
            _ => None,
        }
    }
}

/// How complete an axis measurement is.
///
/// Readiness is mechanical completeness, not confidence. A neutral zero can be
/// a ready measurement.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// Code 0: the primary value is missing.
    Unavailable = 0,
    /// Code 1: the primary value exists but the axis is not yet complete.
    Provisional = 1,
    /// Code 2: the primary value exists, the reference is stable, and the
    /// percentile has enough samples where the axis publishes one.
    Ready = 2,
}

impl Readiness {
    /// Every readiness code, in code order.
    pub const ALL: [Self; 3] = [Self::Unavailable, Self::Provisional, Self::Ready];

    /// Returns the registry code.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::kernel::Readiness;
    ///
    /// assert_eq!(Readiness::Unavailable.code(), 0);
    /// assert_eq!(Readiness::Ready.code(), 2);
    /// ```
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Resolves a registry code.
    ///
    /// Returns `None` for a code the registry does not define.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::kernel::Readiness;
    ///
    /// assert_eq!(Readiness::from_code(1), Some(Readiness::Provisional));
    /// assert_eq!(Readiness::from_code(3), None);
    /// ```
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Unavailable),
            1 => Some(Self::Provisional),
            2 => Some(Self::Ready),
            _ => None,
        }
    }
}

/// Clamps a value to the contract's fixed `[-1, +1]` bounds.
///
/// The bounds are fixed by the contract so no call site re-derives them. A
/// NaN is returned unchanged rather than being reported as zero, and an
/// infinity is clamped to the corresponding bound. An axis must still check
/// availability before publishing the result; the NaN pass-through is a defect
/// signal, not a supported output.
///
/// # Examples
///
/// ```
/// use quantrule_model::kernel::clamp_unit;
///
/// assert_eq!(clamp_unit(-2.0), -1.0);
/// assert_eq!(clamp_unit(0.25), 0.25);
/// assert_eq!(clamp_unit(2.0), 1.0);
/// ```
#[must_use]
pub fn clamp_unit(value: f64) -> f64 {
    value.clamp(-1.0, 1.0)
}

/// Returns the contract's sign of a value: -1 for a negative value, 0 for
/// zero, and +1 for a positive value.
///
/// A zero has sign zero. A NaN is returned unchanged rather than being reported
/// as a neutral zero, because an invalid measurement is not a measured zero.
/// An axis must check availability before publishing the result.
///
/// # Examples
///
/// ```
/// use quantrule_model::kernel::sign;
///
/// assert_eq!(sign(-2.0), -1.0);
/// assert_eq!(sign(0.0), 0.0);
/// assert_eq!(sign(2.0), 1.0);
/// ```
#[must_use]
pub fn sign(value: f64) -> f64 {
    if value.is_nan() {
        f64::NAN
    } else if value < 0.0 {
        -1.0
    } else if value > 0.0 {
        1.0
    } else {
        0.0
    }
}

/// Buckets a Direction or Momentum primary value into the contract's compact
/// state.
///
/// Returns -1 below `-0.1`, 0 from `-0.1` through `+0.1` inclusive, and +1
/// above `+0.1`. Returns `None` when the value is not finite, so a missing or
/// invalid measurement never becomes a neutral state. Structure and Progression
/// publish their own code families and do not use this bucketing.
///
/// # Examples
///
/// ```
/// use quantrule_model::kernel::compact_state;
///
/// assert_eq!(compact_state(-0.2), Some(-1));
/// assert_eq!(compact_state(-0.1), Some(0));
/// assert_eq!(compact_state(0.1), Some(0));
/// assert_eq!(compact_state(0.2), Some(1));
/// assert_eq!(compact_state(f64::NAN), None);
/// ```
#[must_use]
pub fn compact_state(value: f64) -> Option<i8> {
    if !value.is_finite() {
        return None;
    }

    if value < -COMPACT_STATE_THRESHOLD {
        Some(-1)
    } else if value > COMPACT_STATE_THRESHOLD {
        Some(1)
    } else {
        Some(0)
    }
}

/// Computes the contract's empirical percentile of `value`.
///
/// `samples` is the valid magnitude series in bar order, strictly before the
/// current bar. The function scans from the most recent sample backwards,
/// keeps at most the last [`PERCENTILE_WINDOW`] finite values, restores their
/// bar order, sorts them with a stable sort, and returns
/// `100 * count(sample <= value) / n`. It returns `None` when fewer than
/// [`PERCENTILE_MIN_SAMPLES`] finite samples remain or when `value` is not
/// finite.
///
/// The current bar is never one of its own samples: the caller passes only the
/// history before the bar being measured. This is a precondition the kernel
/// cannot enforce, so the reading layer owns the off-by-one test.
///
/// # Examples
///
/// ```
/// use quantrule_model::kernel::empirical_percentile;
///
/// let samples: Vec<f64> = (1_u32..=50).map(f64::from).collect();
/// assert_eq!(empirical_percentile(&samples, 25.0), Some(50.0));
/// assert_eq!(empirical_percentile(&samples[..49], 25.0), None);
/// ```
#[must_use]
pub fn empirical_percentile(samples: &[f64], value: f64) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }

    let mut window: Vec<f64> = Vec::with_capacity(PERCENTILE_WINDOW);
    for sample in samples.iter().rev() {
        if sample.is_finite() {
            window.push(*sample);
            if window.len() == PERCENTILE_WINDOW {
                break;
            }
        }
    }

    if window.len() < PERCENTILE_MIN_SAMPLES {
        return None;
    }

    window.reverse();
    window.sort_by(f64::total_cmp);
    let count = window.iter().filter(|sample| **sample <= value).count();

    Some(100.0 * count as f64 / window.len() as f64)
}

/// A per-axis counter of compact-state age and recent changes.
///
/// The counter is agnostic to the code family: the axis passes its published
/// compact code. A `None` code freezes both counters, per the contract. The
/// first closed bar with a code has age 1; each later bar with the same code
/// increments the age, and a change resets it to 1.
///
/// # Examples
///
/// ```
/// use quantrule_model::kernel::StateCounters;
///
/// let mut counters = StateCounters::new();
/// assert_eq!(counters.observe(Some(0)).age_bars, 1);
/// assert_eq!(counters.observe(Some(0)).age_bars, 2);
///
/// let changed = counters.observe(Some(1));
/// assert_eq!(changed.age_bars, 1);
/// assert_eq!(changed.changes_50, 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateCounters {
    last_code: Option<i8>,
    age_bars: u64,
    changes_50: u64,
    change_window: [bool; STATE_CHANGE_WINDOW],
    next_slot: usize,
}

impl StateCounters {
    /// Creates a counter with no observed bar.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_code: None,
            age_bars: 0,
            changes_50: 0,
            change_window: [false; STATE_CHANGE_WINDOW],
            next_slot: 0,
        }
    }

    /// Observes one closed bar and returns the updated counters.
    ///
    /// `code` is the axis's published compact code. `None` means the code is
    /// unavailable on this bar; both counters then hold their previous values.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::kernel::StateCounters;
    ///
    /// let mut counters = StateCounters::new();
    /// counters.observe(Some(0));
    /// let frozen = counters.observe(None);
    /// assert_eq!(frozen.age_bars, 1);
    /// ```
    pub fn observe(&mut self, code: Option<i8>) -> StateStats {
        let Some(code) = code else {
            return self.stats();
        };

        match self.last_code {
            Some(previous) if previous == code => {
                self.age_bars = self.age_bars.saturating_add(1);
                self.advance_window(false);
            }
            Some(_) => {
                self.age_bars = 1;
                self.advance_window(true);
            }
            None => {
                self.age_bars = 1;
                self.advance_window(false);
            }
        }
        self.last_code = Some(code);

        self.stats()
    }

    /// Returns the counters without observing a bar.
    #[must_use]
    pub const fn stats(&self) -> StateStats {
        StateStats {
            age_bars: self.age_bars,
            changes_50: self.changes_50,
        }
    }

    fn advance_window(&mut self, changed: bool) {
        // One slot per code-bearing bar. `changes_50` is the number of true
        // slots, so it never exceeds the window length and the subtraction is
        // guarded by the slot it is clearing.
        let slot = self.next_slot;
        if self.change_window[slot] {
            self.changes_50 -= 1;
        }
        self.change_window[slot] = changed;
        if changed {
            self.changes_50 += 1;
        }
        self.next_slot = (slot + 1) % STATE_CHANGE_WINDOW;
    }
}

impl Default for StateCounters {
    fn default() -> Self {
        Self::new()
    }
}

/// The state-counter values published for one closed bar.
///
/// Both values are non-negative integers. `age_bars` is zero only before the
/// first code; `changes_50` is at most [`STATE_CHANGE_WINDOW`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateStats {
    /// Closed bars the current state has held; zero before any code.
    pub age_bars: u64,
    /// State changes in the last 50 closed bars that carried a code.
    pub changes_50: u64,
}

#[cfg(test)]
mod tests {
    use super::{
        COMPACT_STATE_THRESHOLD, PERCENTILE_MIN_SAMPLES, PERCENTILE_WINDOW, Readiness,
        STATE_CHANGE_WINDOW, StateCounters, StateStats, UnavailableReason, clamp_unit,
        compact_state, empirical_percentile, sign,
    };

    const PERCENTILE_FIXTURE: [f64; PERCENTILE_MIN_SAMPLES] = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0, 25.0, 26.0, 27.0, 28.0, 29.0, 30.0, 31.0,
        32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0,
        47.0, 48.0, 49.0, 50.0,
    ];

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn unavailable_reason_codes_match_the_contract() {
        let codes: Vec<u8> = UnavailableReason::ALL
            .iter()
            .map(|reason| reason.code())
            .collect();
        assert_eq!(codes, [1, 2, 3, 4, 5, 6]);

        for reason in UnavailableReason::ALL {
            assert_eq!(UnavailableReason::from_code(reason.code()), Some(reason));
        }
        assert_eq!(UnavailableReason::from_code(0), None);
        assert_eq!(UnavailableReason::from_code(7), None);
    }

    #[test]
    fn readiness_codes_match_the_contract() {
        let codes: Vec<u8> = Readiness::ALL
            .iter()
            .map(|readiness| readiness.code())
            .collect();
        assert_eq!(codes, [0, 1, 2]);

        for readiness in Readiness::ALL {
            assert_eq!(Readiness::from_code(readiness.code()), Some(readiness));
        }
        assert_eq!(Readiness::from_code(3), None);
    }

    #[test]
    fn clamp_unit_uses_the_fixed_bounds() {
        assert_eq!(clamp_unit(-2.0), -1.0);
        assert_eq!(clamp_unit(-1.0), -1.0);
        assert_eq!(clamp_unit(-0.25), -0.25);
        assert_eq!(clamp_unit(0.0), 0.0);
        assert_eq!(clamp_unit(0.25), 0.25);
        assert_eq!(clamp_unit(1.0), 1.0);
        assert_eq!(clamp_unit(2.0), 1.0);
        assert_eq!(clamp_unit(f64::INFINITY), 1.0);
        assert_eq!(clamp_unit(f64::NEG_INFINITY), -1.0);
        assert!(clamp_unit(f64::NAN).is_nan());
    }

    #[test]
    fn sign_is_negative_zero_or_positive() {
        assert_eq!(sign(-2.0), -1.0);
        assert_eq!(sign(-0.0), 0.0);
        assert_eq!(sign(0.0), 0.0);
        assert_eq!(sign(2.0), 1.0);
        assert_eq!(sign(f64::INFINITY), 1.0);
        assert_eq!(sign(f64::NEG_INFINITY), -1.0);
        assert!(sign(f64::NAN).is_nan());
    }

    #[test]
    fn compact_state_is_inclusive_at_the_threshold() {
        assert_eq!(compact_state(-0.2), Some(-1));
        assert_eq!(compact_state(-COMPACT_STATE_THRESHOLD), Some(0));
        assert_eq!(compact_state(0.0), Some(0));
        assert_eq!(compact_state(COMPACT_STATE_THRESHOLD), Some(0));
        assert_eq!(compact_state(0.2), Some(1));
        assert_eq!(compact_state(f64::NAN), None);
        assert_eq!(compact_state(f64::INFINITY), None);
        assert_eq!(compact_state(f64::NEG_INFINITY), None);
    }

    #[test]
    fn percentile_matches_the_hand_computed_fixture() {
        assert_eq!(empirical_percentile(&PERCENTILE_FIXTURE, 0.0), Some(0.0));
        assert_eq!(empirical_percentile(&PERCENTILE_FIXTURE, 1.0), Some(2.0));
        assert_eq!(empirical_percentile(&PERCENTILE_FIXTURE, 25.0), Some(50.0));
        assert_eq!(empirical_percentile(&PERCENTILE_FIXTURE, 50.0), Some(100.0));
        assert_eq!(empirical_percentile(&PERCENTILE_FIXTURE, 51.0), Some(100.0));
    }

    #[test]
    fn percentile_requires_fifty_samples() {
        let forty_nine: Vec<f64> = (1_u32..=49).map(f64::from).collect();
        assert_eq!(empirical_percentile(&forty_nine, 25.0), None);

        let fifty: Vec<f64> = (1_u32..=50).map(f64::from).collect();
        assert_eq!(empirical_percentile(&fifty, 25.0), Some(50.0));
    }

    #[test]
    fn percentile_is_null_for_an_empty_or_single_sample() {
        assert_eq!(empirical_percentile(&[], 0.0), None);
        assert_eq!(empirical_percentile(&[1.0], 1.0), None);
    }

    #[test]
    fn percentile_uses_only_the_most_recent_252_samples() {
        let samples: Vec<f64> = (1_u32..=300).map(f64::from).collect();
        assert_eq!(samples.len(), 300);

        // The last 252 samples are 49..=300, so 48 is below all of them.
        assert_eq!(empirical_percentile(&samples, 48.0), Some(0.0));
        assert_close(
            empirical_percentile(&samples, 49.0).unwrap(),
            100.0 / PERCENTILE_WINDOW as f64,
        );
        assert_eq!(empirical_percentile(&samples, 300.0), Some(100.0));
    }

    #[test]
    fn percentile_is_independent_of_sample_order() {
        let samples: Vec<f64> = (1_u32..=50).map(f64::from).collect();
        let mut reversed = samples.clone();
        reversed.reverse();

        let interleaved: Vec<f64> = samples
            .iter()
            .copied()
            .step_by(2)
            .chain(samples.iter().copied().skip(1).step_by(2))
            .collect();
        assert_eq!(interleaved.len(), samples.len());

        let expected = empirical_percentile(&samples, 25.0);
        assert_eq!(empirical_percentile(&reversed, 25.0), expected);
        assert_eq!(empirical_percentile(&interleaved, 25.0), expected);
    }

    #[test]
    fn percentile_ignores_non_finite_samples() {
        let mut samples: Vec<f64> = (1_u32..=49).map(f64::from).collect();
        samples.push(f64::NAN);
        assert_eq!(empirical_percentile(&samples, 25.0), None);

        samples.push(50.0);
        samples.push(f64::INFINITY);
        assert_eq!(empirical_percentile(&samples, 25.0), Some(50.0));
        assert_eq!(empirical_percentile(&samples, f64::NAN), None);
        assert_eq!(empirical_percentile(&samples, f64::INFINITY), None);
    }

    #[test]
    fn non_finite_samples_do_not_consume_a_window_slot() {
        let mut samples: Vec<f64> = (1_u32..=251).map(f64::from).collect();
        samples.push(f64::NAN);
        samples.push(252.0);

        // The last 252 finite values are 1..=252, so 1 is still in the window.
        assert_close(
            empirical_percentile(&samples, 1.0).unwrap(),
            100.0 / PERCENTILE_WINDOW as f64,
        );
    }

    #[test]
    fn percentile_is_inclusive_and_preserves_a_valid_zero() {
        let equal = [10.0; PERCENTILE_MIN_SAMPLES];
        assert_eq!(empirical_percentile(&equal, 10.0), Some(100.0));
        assert_eq!(empirical_percentile(&equal, 9.0), Some(0.0));

        let zeros = [0.0; PERCENTILE_MIN_SAMPLES];
        assert_eq!(empirical_percentile(&zeros, 0.0), Some(100.0));
    }

    #[test]
    fn counters_start_empty() {
        let counters = StateCounters::new();
        assert_eq!(
            counters.stats(),
            StateStats {
                age_bars: 0,
                changes_50: 0,
            }
        );
    }

    #[test]
    fn a_state_ages_and_a_change_resets_it() {
        let mut counters = StateCounters::new();
        assert_eq!(counters.observe(Some(0)).age_bars, 1);
        assert_eq!(counters.observe(Some(0)).age_bars, 2);

        let stats = counters.observe(Some(0));
        assert_eq!(stats.age_bars, 3);
        assert_eq!(stats.changes_50, 0);

        let changed = counters.observe(Some(1));
        assert_eq!(changed.age_bars, 1);
        assert_eq!(changed.changes_50, 1);
    }

    #[test]
    fn a_null_freezes_both_counters() {
        let mut counters = StateCounters::new();
        counters.observe(Some(0));
        counters.observe(Some(1));

        let frozen = counters.observe(None);
        assert_eq!(frozen.age_bars, 1);
        assert_eq!(frozen.changes_50, 1);

        let resumed = counters.observe(Some(1));
        assert_eq!(resumed.age_bars, 2);
        assert_eq!(resumed.changes_50, 1);

        let changed = counters.observe(Some(0));
        assert_eq!(changed.age_bars, 1);
        assert_eq!(changed.changes_50, 2);
    }

    #[test]
    fn a_null_only_series_keeps_the_counters_empty() {
        let mut counters = StateCounters::new();
        for _ in 0..100 {
            assert_eq!(
                counters.observe(None),
                StateStats {
                    age_bars: 0,
                    changes_50: 0,
                }
            );
        }
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(StateCounters::default(), StateCounters::new());
    }

    #[test]
    fn a_null_does_not_consume_a_change_window_slot() {
        let mut counters = StateCounters::new();
        counters.observe(Some(0));
        counters.observe(Some(1));

        for _ in 0..100 {
            counters.observe(None);
        }
        assert_eq!(counters.stats().changes_50, 1);
    }

    #[test]
    fn the_change_window_holds_the_last_fifty_observations() {
        let mut counters = StateCounters::new();
        counters.observe(Some(0));

        for index in 0..60 {
            let code = if index % 2 == 0 { 1 } else { 0 };
            counters.observe(Some(code));
        }
        assert_eq!(counters.stats().changes_50, STATE_CHANGE_WINDOW as u64);

        for _ in 0..STATE_CHANGE_WINDOW {
            counters.observe(Some(0));
        }
        assert_eq!(counters.stats().changes_50, 0);
    }

    #[test]
    fn the_counter_accepts_every_published_code_family() {
        let mut counters = StateCounters::new();
        assert_eq!(counters.observe(Some(-2)).age_bars, 1);
        assert_eq!(counters.observe(Some(-2)).changes_50, 0);
        assert_eq!(counters.observe(Some(5)).age_bars, 1);
        assert_eq!(counters.stats().changes_50, 1);
    }

    #[test]
    fn repeated_counter_runs_are_identical() {
        let run = || {
            let mut counters = StateCounters::new();
            let mut stats = Vec::new();
            for code in [
                Some(0),
                Some(0),
                Some(1),
                None,
                Some(1),
                Some(-1),
                Some(-1),
                Some(0),
            ] {
                stats.push(counters.observe(code));
            }
            stats
        };

        assert_eq!(run(), run());
    }

    #[test]
    fn missing_measurements_are_null_not_zero() {
        assert_eq!(compact_state(f64::NAN), None);
        assert_eq!(empirical_percentile(&[1.0; 49], 1.0), None);
        assert_eq!(empirical_percentile(&[1.0; 50], f64::NAN), None);

        // A valid zero is still a measured value.
        assert_eq!(compact_state(0.0), Some(0));
        assert_eq!(sign(0.0), 0.0);
        assert_eq!(empirical_percentile(&[0.0; 50], 0.0), Some(100.0));
    }
}
