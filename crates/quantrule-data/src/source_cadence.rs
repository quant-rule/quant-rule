//! Source-row spacing for the timeframe ladder.
//!
//! The ladder is built from an M1 source. The shell observes every parsed
//! source row before a date bound or retention bound is applied, so the spacing
//! measured here describes the file rather than the analysis window. A file
//! whose rows are never closer than a coarser timeframe is a native
//! higher-timeframe series; treating it as M1 would label hourly candles `1m`
//! and leave M5, M15, and H1 empty.
//!
//! This module performs no I/O and reads no clock.

use crate::{Timeframe, TimeframeError};

/// The smallest spacing observed between consecutive source rows.
///
/// The tracker is fed every parsed row in file order. It keeps the smallest
/// positive delta, so a sparse M1 file with gaps is accepted as soon as one
/// adjacent pair is one minute apart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceCadence {
    last_timestamp: Option<i64>,
    minimum_seconds: Option<i64>,
}

impl SourceCadence {
    /// Creates a tracker with no observations.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_timestamp: None,
            minimum_seconds: None,
        }
    }

    /// Records one source timestamp in file order.
    ///
    /// Non-positive deltas are ignored. Duplicate and out-of-order rows are
    /// rejected by the candle window with their own errors, and they must not
    /// make a coarser source look M1-spaced before that error is reached.
    pub fn observe(&mut self, timestamp: i64) {
        if let Some(previous) = self.last_timestamp {
            let spacing = timestamp - previous;
            if spacing > 0 && self.minimum_seconds.is_none_or(|minimum| spacing < minimum) {
                self.minimum_seconds = Some(spacing);
            }
        }
        self.last_timestamp = Some(timestamp);
    }

    /// The smallest positive spacing between consecutive rows, in seconds.
    ///
    /// `None` until two rows have been observed. A single row has no spacing to
    /// measure and is therefore not rejected on cadence.
    #[must_use]
    pub const fn minimum_seconds(&self) -> Option<i64> {
        self.minimum_seconds
    }

    /// Requires the observed rows to be spaced as M1 candles.
    ///
    /// # Errors
    ///
    /// Returns [`TimeframeError::SourceCadence`] when at least two rows were
    /// observed and their smallest spacing is wider than one minute. A valid M1
    /// source with gaps is accepted because at least one adjacent pair is one
    /// minute apart.
    pub fn require_m1(&self) -> Result<(), TimeframeError> {
        match self.minimum_seconds {
            Some(observed_seconds) if observed_seconds > i64::from(Timeframe::M1.seconds()) => {
                Err(TimeframeError::SourceCadence { observed_seconds })
            }
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SourceCadence;
    use crate::TimeframeError;

    const BASE: i64 = 1_735_689_600;

    fn cadence_at(steps: &[i64]) -> SourceCadence {
        let mut cadence = SourceCadence::new();
        let mut timestamp = BASE;
        for step in steps {
            cadence.observe(timestamp);
            timestamp += step;
        }
        cadence
    }

    #[test]
    fn empty_and_single_row_sources_have_no_measured_cadence() {
        let mut cadence = SourceCadence::new();
        assert_eq!(cadence.minimum_seconds(), None);
        assert_eq!(cadence.require_m1(), Ok(()));

        cadence.observe(BASE);
        assert_eq!(cadence.minimum_seconds(), None);
        assert_eq!(cadence.require_m1(), Ok(()));
    }

    #[test]
    fn consecutive_minutes_are_accepted() {
        let cadence = cadence_at(&[60, 60, 60]);
        assert_eq!(cadence.minimum_seconds(), Some(60));
        assert_eq!(cadence.require_m1(), Ok(()));
    }

    #[test]
    fn a_sparse_m1_source_is_accepted_when_one_pair_is_one_minute() {
        let cadence = cadence_at(&[60, 240, 60, 300]);
        assert_eq!(cadence.minimum_seconds(), Some(60));
        assert_eq!(cadence.require_m1(), Ok(()));
    }

    #[test]
    fn native_m5_m15_and_h1_sources_are_rejected() {
        for (step, observed_seconds) in [(300, 300), (900, 900), (3_600, 3_600)] {
            let cadence = cadence_at(&[step, step, step]);
            assert_eq!(cadence.minimum_seconds(), Some(step));
            assert_eq!(
                cadence.require_m1(),
                Err(TimeframeError::SourceCadence { observed_seconds }),
                "step {step}"
            );
        }
    }

    #[test]
    fn a_coarser_source_is_not_hidden_by_non_positive_deltas() {
        let mut cadence = SourceCadence::new();
        cadence.observe(BASE);
        cadence.observe(BASE + 3_600);
        cadence.observe(BASE + 3_600);
        cadence.observe(BASE + 1_800);

        assert_eq!(cadence.minimum_seconds(), Some(3_600));
        assert_eq!(
            cadence.require_m1(),
            Err(TimeframeError::SourceCadence {
                observed_seconds: 3_600
            })
        );
    }

    #[test]
    fn the_smallest_positive_spacing_wins() {
        let cadence = cadence_at(&[3_600, 3_600, 60, 900]);
        assert_eq!(cadence.minimum_seconds(), Some(60));
        assert_eq!(cadence.require_m1(), Ok(()));
    }

    #[test]
    fn the_error_names_the_observed_cadence_and_the_fix() {
        let cadence = cadence_at(&[3_600, 3_600]);
        let error = cadence
            .require_m1()
            .expect_err("an hourly source is rejected");
        assert_eq!(
            error.to_string(),
            "source cadence is 3600s, not 60s; provide an M1 source file for the timeframe ladder"
        );
    }
}
