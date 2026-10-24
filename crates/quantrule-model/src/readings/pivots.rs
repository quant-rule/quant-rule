//! Confirmed swing pivots and pivot candidates.
//!
//! A pivot candidate is a bar whose high is strictly greater than the highs of
//! the lookback bars to its left, or whose low is strictly lower than the lows
//! of the lookback bars to its left. A candidate is knowable at the close of
//! its own bar. A confirmed pivot adds the same strict comparison against the
//! lookback bars to its right, so it becomes knowable only at
//! `occurred_at + lookback`.
//!
//! The confirmed vectors are indexed by the availability bar. Reading the
//! vector at or before a bar therefore cannot expose a pivot that is not yet
//! knowable. A bar that qualifies as both a candidate high and a candidate low
//! at one scale is discarded at that scale, and the same rule applies to a
//! confirmed pivot.

use quantrule_data::Candle;

/// A pivot scale.
///
/// The scale's lookback is the number of bars required on each side of the
/// pivot. The minor scale uses two; the major scale uses five.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PivotScale {
    /// Minor scale: two bars on each side.
    Minor = 1,
    /// Major scale: five bars on each side.
    Major = 2,
}

impl PivotScale {
    /// Both scales, minor first.
    pub const ALL: [Self; 2] = [Self::Minor, Self::Major];

    /// The scale's registry code: 1 for minor, 2 for major.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The number of bars required on each side of the pivot.
    #[must_use]
    pub const fn lookback(self) -> usize {
        match self {
            Self::Minor => 2,
            Self::Major => 5,
        }
    }
}

/// Whether a pivot is a swing high or a swing low.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PivotKind {
    /// A swing high.
    High,
    /// A swing low.
    Low,
}

/// A pivot candidate.
///
/// The candidate is knowable on its occurrence bar, so `available_at` equals
/// `occurred_at`. It is not a confirmed pivot until the lookback bars to its
/// right have closed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PivotCandidate {
    /// The pivot scale.
    pub scale: PivotScale,
    /// Whether the candidate is a high or a low.
    pub kind: PivotKind,
    /// The candidate's high or low price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_at: usize,
    /// The first bar on which the candidate was knowable.
    pub available_at: usize,
}

/// A confirmed swing pivot.
///
/// The pivot is stored at its availability bar inside [`PivotSeries`]. Its
/// `occurred_at` is earlier by the scale's lookback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pivot {
    /// The pivot scale.
    pub scale: PivotScale,
    /// Whether the pivot is a high or a low.
    pub kind: PivotKind,
    /// The pivot's high or low price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_at: usize,
    /// The first bar on which the pivot became knowable.
    pub available_at: usize,
}

/// The pivot readings for one candle series.
///
/// The candidate vectors have one entry per input candle. The confirmed
/// vectors also have one entry per input candle, but the entry at index `bar`
/// is the pivot that became knowable on `bar`, if any. A pivot that occurs near
/// the end of the series and is not yet confirmed is absent from the confirmed
/// vectors, while its candidate remains available.
#[derive(Debug, Clone, PartialEq)]
pub struct PivotSeries {
    minor_candidates: Vec<Option<PivotCandidate>>,
    major_candidates: Vec<Option<PivotCandidate>>,
    minor_confirmed: Vec<Option<Pivot>>,
    major_confirmed: Vec<Option<Pivot>>,
}

impl PivotSeries {
    /// Computes both scales from the closed candles.
    #[must_use]
    pub fn compute(candles: &[Candle]) -> Self {
        let len = candles.len();
        let mut minor_candidates = vec![None; len];
        let mut major_candidates = vec![None; len];
        let mut minor_confirmed = vec![None; len];
        let mut major_confirmed = vec![None; len];

        for index in 0..len {
            for scale in PivotScale::ALL {
                let Some(candidate) = candidate_at(candles, index, scale) else {
                    continue;
                };

                match scale {
                    PivotScale::Minor => minor_candidates[index] = Some(candidate),
                    PivotScale::Major => major_candidates[index] = Some(candidate),
                }

                let Some(pivot) = confirm_candidate(candles, candidate) else {
                    continue;
                };
                match scale {
                    PivotScale::Minor => minor_confirmed[pivot.available_at] = Some(pivot),
                    PivotScale::Major => major_confirmed[pivot.available_at] = Some(pivot),
                }
            }
        }

        Self {
            minor_candidates,
            major_candidates,
            minor_confirmed,
            major_confirmed,
        }
    }

    /// The number of input bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.minor_candidates.len()
    }

    /// Whether the series has no input bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.minor_candidates.is_empty()
    }

    /// The candidate at `bar` for `scale`, or `None`.
    #[must_use]
    pub fn candidate(&self, scale: PivotScale, bar: usize) -> Option<PivotCandidate> {
        self.candidates(scale).get(bar).copied().flatten()
    }

    /// The candidate vector for `scale`, one entry per input bar.
    #[must_use]
    pub fn candidates(&self, scale: PivotScale) -> &[Option<PivotCandidate>] {
        match scale {
            PivotScale::Minor => &self.minor_candidates,
            PivotScale::Major => &self.major_candidates,
        }
    }

    /// The pivot that became knowable at `bar` for `scale`, or `None`.
    #[must_use]
    pub fn confirmed_at(&self, scale: PivotScale, bar: usize) -> Option<Pivot> {
        self.confirmed_series(scale).get(bar).copied().flatten()
    }

    /// Every confirmed pivot of `scale` that is knowable at or before `bar`.
    ///
    /// A pivot with `available_at` later than `bar` is never yielded.
    pub fn confirmed_as_of(
        &self,
        scale: PivotScale,
        bar: usize,
    ) -> impl Iterator<Item = &Pivot> + '_ {
        self.confirmed_series(scale)
            .iter()
            .take(bar.saturating_add(1))
            .filter_map(Option::as_ref)
    }

    /// Every confirmed pivot of `scale`, in availability order.
    ///
    /// This includes pivots whose availability bar is later than any particular
    /// consumer's current bar; use [`Self::confirmed_as_of`] when the consumer
    /// is evaluating a bar.
    pub fn all_confirmed(&self, scale: PivotScale) -> impl Iterator<Item = &Pivot> + '_ {
        self.confirmed_series(scale)
            .iter()
            .filter_map(Option::as_ref)
    }

    /// The number of candidates of `scale`.
    #[must_use]
    pub fn candidate_count(&self, scale: PivotScale) -> usize {
        self.candidates(scale).iter().flatten().count()
    }

    /// The number of confirmed pivots of `scale`.
    #[must_use]
    pub fn confirmed_count(&self, scale: PivotScale) -> usize {
        self.all_confirmed(scale).count()
    }

    fn confirmed_series(&self, scale: PivotScale) -> &[Option<Pivot>] {
        match scale {
            PivotScale::Minor => &self.minor_confirmed,
            PivotScale::Major => &self.major_confirmed,
        }
    }
}

fn candidate_at(candles: &[Candle], index: usize, scale: PivotScale) -> Option<PivotCandidate> {
    let lookback = scale.lookback();
    if index < lookback {
        return None;
    }

    let candle = candles[index];
    let left = &candles[(index - lookback)..index];
    let high_candidate = left.iter().all(|prior| candle.high() > prior.high());
    let low_candidate = left.iter().all(|prior| candle.low() < prior.low());

    match (high_candidate, low_candidate) {
        (true, false) => Some(PivotCandidate {
            scale,
            kind: PivotKind::High,
            price: candle.high(),
            occurred_at: index,
            available_at: index,
        }),
        (false, true) => Some(PivotCandidate {
            scale,
            kind: PivotKind::Low,
            price: candle.low(),
            occurred_at: index,
            available_at: index,
        }),
        _ => None,
    }
}

fn confirm_candidate(candles: &[Candle], candidate: PivotCandidate) -> Option<Pivot> {
    let available_at = candidate.occurred_at + candidate.scale.lookback();
    if available_at >= candles.len() {
        return None;
    }

    let right = &candles[(candidate.occurred_at + 1)..=available_at];
    let confirmed = match candidate.kind {
        PivotKind::High => right.iter().all(|later| candidate.price > later.high()),
        PivotKind::Low => right.iter().all(|later| candidate.price < later.low()),
    };
    if !confirmed {
        return None;
    }

    Some(Pivot {
        scale: candidate.scale,
        kind: candidate.kind,
        price: candidate.price,
        occurred_at: candidate.occurred_at,
        available_at,
    })
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;

    use super::{Pivot, PivotKind, PivotScale, PivotSeries};

    fn candles(highs: &[f64], lows: &[f64]) -> Vec<Candle> {
        assert_eq!(highs.len(), lows.len());
        highs
            .iter()
            .zip(lows)
            .enumerate()
            .map(|(index, (high, low))| {
                let close = (high + low) / 2.0;
                Candle::new(index as u64, index as i64, close, *high, *low, close, None)
                    .expect("valid test candle")
            })
            .collect()
    }

    fn pivots_at(series: &PivotSeries, scale: PivotScale, bar: usize) -> Vec<Pivot> {
        series.confirmed_as_of(scale, bar).copied().collect()
    }

    #[test]
    fn scale_codes_and_lookbacks_match_the_contract() {
        assert_eq!(PivotScale::Minor.code(), 1);
        assert_eq!(PivotScale::Major.code(), 2);
        assert_eq!(PivotScale::Minor.lookback(), 2);
        assert_eq!(PivotScale::Major.lookback(), 5);
        assert_eq!(PivotScale::ALL, [PivotScale::Minor, PivotScale::Major]);
    }

    #[test]
    fn a_minor_pivot_high_is_confirmed_at_occurrence_plus_two() {
        let candles = candles(&[1.0, 2.0, 3.0, 2.0, 1.0], &[0.0; 5]);
        let series = PivotSeries::compute(&candles);

        assert_eq!(
            series.candidate(PivotScale::Minor, 2),
            Some(super::PivotCandidate {
                scale: PivotScale::Minor,
                kind: PivotKind::High,
                price: 3.0,
                occurred_at: 2,
                available_at: 2,
            })
        );
        assert_eq!(series.confirmed_at(PivotScale::Minor, 3), None);
        let pivot = series
            .confirmed_at(PivotScale::Minor, 4)
            .expect("the pivot is confirmed at bar 4");
        assert_eq!(pivot.kind, PivotKind::High);
        assert_eq!(pivot.price, 3.0);
        assert_eq!(pivot.occurred_at, 2);
        assert_eq!(pivot.available_at, 4);
    }

    #[test]
    fn a_minor_pivot_low_is_confirmed_at_occurrence_plus_two() {
        let candles = candles(&[5.0; 5], &[2.0, 1.0, 0.0, 1.0, 2.0]);
        let series = PivotSeries::compute(&candles);

        let pivot = series
            .confirmed_at(PivotScale::Minor, 4)
            .expect("the pivot is confirmed at bar 4");
        assert_eq!(pivot.kind, PivotKind::Low);
        assert_eq!(pivot.price, 0.0);
        assert_eq!(pivot.occurred_at, 2);
    }

    #[test]
    fn a_major_pivot_high_is_confirmed_at_occurrence_plus_five() {
        let candles = candles(
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0],
            &[0.0; 11],
        );
        let series = PivotSeries::compute(&candles);

        assert_eq!(series.confirmed_at(PivotScale::Major, 9), None);
        let pivot = series
            .confirmed_at(PivotScale::Major, 10)
            .expect("the major pivot is confirmed at bar 10");
        assert_eq!(pivot.kind, PivotKind::High);
        assert_eq!(pivot.occurred_at, 5);
        assert_eq!(pivot.available_at, 10);
    }

    #[test]
    fn equal_highs_and_equal_lows_are_not_candidates() {
        let candles = candles(&[5.0, 5.0, 5.0], &[0.0, 0.0, 0.0]);
        let series = PivotSeries::compute(&candles);

        assert_eq!(series.candidate(PivotScale::Minor, 2), None);
        assert_eq!(series.confirmed_count(PivotScale::Minor), 0);
    }

    #[test]
    fn an_equal_left_high_blocks_only_the_high_candidate() {
        let candles = candles(&[5.0, 5.0, 5.0, 4.0, 4.0], &[1.0, 2.0, 0.0, 2.0, 3.0]);
        let series = PivotSeries::compute(&candles);

        let candidate = series
            .candidate(PivotScale::Minor, 2)
            .expect("the low is still a candidate");
        assert_eq!(candidate.kind, PivotKind::Low);
        assert_eq!(candidate.price, 0.0);

        let pivot = series
            .confirmed_at(PivotScale::Minor, 4)
            .expect("the low is confirmed at bar 4");
        assert_eq!(pivot.kind, PivotKind::Low);
        assert_eq!(pivot.price, 0.0);
    }

    #[test]
    fn a_bar_that_is_both_candidates_is_discarded() {
        let candles = candles(&[10.0, 10.0, 11.0], &[0.0, 0.0, -1.0]);
        let series = PivotSeries::compute(&candles);

        assert_eq!(series.candidate(PivotScale::Minor, 2), None);
        assert_eq!(series.confirmed_count(PivotScale::Minor), 0);
    }

    #[test]
    fn a_candidate_that_fails_confirmation_remains_a_candidate_only() {
        let candles = candles(&[1.0, 2.0, 3.0, 3.0, 1.0], &[0.0; 5]);
        let series = PivotSeries::compute(&candles);

        assert_eq!(
            series
                .candidate(PivotScale::Minor, 2)
                .expect("the candidate exists")
                .kind,
            PivotKind::High
        );
        assert_eq!(series.confirmed_at(PivotScale::Minor, 4), None);
    }

    #[test]
    fn a_pivot_is_absent_before_its_availability_bar() {
        let candles = candles(&[1.0, 2.0, 3.0, 2.0, 1.0], &[0.0; 5]);
        let series = PivotSeries::compute(&candles);

        assert!(pivots_at(&series, PivotScale::Minor, 3).is_empty());
        assert_eq!(pivots_at(&series, PivotScale::Minor, 4).len(), 1);
    }

    #[test]
    fn every_confirmed_pivot_agrees_with_its_candidate() {
        let candles = candles(
            &[
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 2.0, 3.0,
            ],
            &[
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
        );
        let series = PivotSeries::compute(&candles);

        for scale in PivotScale::ALL {
            for pivot in series.all_confirmed(scale) {
                let candidate = series
                    .candidate(scale, pivot.occurred_at)
                    .expect("every confirmed pivot came from a candidate");
                assert_eq!(candidate.kind, pivot.kind);
                assert_eq!(candidate.price, pivot.price);
                assert_eq!(candidate.occurred_at, pivot.occurred_at);
            }
        }
    }

    #[test]
    fn no_confirmed_pivot_exists_without_a_candidate() {
        let candles = candles(&[1.0, 2.0, 3.0, 2.0, 1.0], &[0.0; 5]);
        let series = PivotSeries::compute(&candles);

        for scale in PivotScale::ALL {
            for pivot in series.all_confirmed(scale) {
                assert!(series.candidate(scale, pivot.occurred_at).is_some());
            }
        }
    }

    #[test]
    fn a_gap_can_form_a_pivot() {
        let candles = candles(
            &[1.0, 2.0, 3.0, 10.0, 1.0, 2.0, 3.0],
            &[0.0, 0.0, 0.0, 9.0, 0.0, 0.0, 0.0],
        );
        let series = PivotSeries::compute(&candles);

        let pivot = series
            .confirmed_at(PivotScale::Minor, 5)
            .expect("the gap bar is a confirmed minor pivot high");
        assert_eq!(pivot.kind, PivotKind::High);
        assert_eq!(pivot.occurred_at, 3);
        assert_eq!(pivot.available_at, 5);
    }

    #[test]
    fn a_zero_volume_candle_can_form_a_pivot() {
        let candles: Vec<Candle> = [1.0, 2.0, 3.0, 2.0, 1.0]
            .into_iter()
            .enumerate()
            .map(|(index, high)| {
                Candle::new(index as u64, index as i64, high, high, 0.0, high, Some(0.0))
                    .expect("valid test candle")
            })
            .collect();
        let series = PivotSeries::compute(&candles);

        let pivot = series
            .confirmed_at(PivotScale::Minor, 4)
            .expect("the zero-volume bar is a confirmed pivot high");
        assert_eq!(pivot.kind, PivotKind::High);
        assert_eq!(pivot.price, 3.0);
    }

    #[test]
    fn an_empty_series_has_no_pivots() {
        let series = PivotSeries::compute(&[]);

        assert!(series.is_empty());
        assert_eq!(series.len(), 0);
        assert_eq!(series.candidate(PivotScale::Minor, 0), None);
        assert_eq!(series.confirmed_count(PivotScale::Minor), 0);
        assert_eq!(series.confirmed_count(PivotScale::Major), 0);
    }

    #[test]
    fn a_single_candle_has_no_pivots() {
        let candles = candles(&[3.0], &[1.0]);
        let series = PivotSeries::compute(&candles);

        assert_eq!(series.len(), 1);
        assert_eq!(series.candidate(PivotScale::Minor, 0), None);
        assert_eq!(series.confirmed_count(PivotScale::Minor), 0);
        assert_eq!(series.confirmed_count(PivotScale::Major), 0);
    }

    #[test]
    fn a_short_series_has_no_major_pivots() {
        let candles = candles(&[1.0, 2.0, 3.0, 2.0, 1.0], &[0.0; 5]);
        let series = PivotSeries::compute(&candles);

        assert_eq!(series.confirmed_count(PivotScale::Major), 0);
    }

    #[test]
    fn appended_bars_do_not_change_earlier_confirmed_pivots() {
        let candles = candles(
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 2.0],
            &[0.0; 12],
        );
        let full = PivotSeries::compute(&candles);
        let prefix = PivotSeries::compute(&candles[..11]);

        for scale in PivotScale::ALL {
            assert_eq!(
                pivots_at(&full, scale, 10),
                pivots_at(&prefix, scale, 10),
                "scale {scale:?}"
            );
        }
    }

    #[test]
    fn repeated_runs_are_identical() {
        let candles = candles(
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0],
            &[0.0; 11],
        );
        let first = PivotSeries::compute(&candles);
        let second = PivotSeries::compute(&candles);

        assert_eq!(first, second);
    }
}
