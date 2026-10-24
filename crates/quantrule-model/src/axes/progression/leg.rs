//! The per-scale progression leg and its lifecycle.
//!
//! A leg is created from a pivot candidate, promoted when the candidate
//! confirms, and ended by completion, supersession, or invalidation. The leg
//! owns its target, its target version and stability, and its measured
//! progress. The module is pure: it reads the candidate, the confirmed-level
//! registry, and the close passed to it.

use crate::kernel::UnavailableReason;
use crate::readings::{PivotCandidate, PivotKind};

use super::ProgressionLeg;
use super::target::{self, Target};

/// Contract §8.4 lifecycle codes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lifecycle {
    /// 0: the origin is not yet confirmed.
    Forming = 0,
    /// 1: the origin is confirmed and the leg is live.
    Active = 1,
    /// 2: an opposing pivot of the same scale confirmed.
    Completed = 2,
    /// 3: price closed beyond the origin against the leg.
    Invalidated = 3,
    /// 4: a newer pivot of the same kind replaced the leg.
    Superseded = 4,
}

impl Lifecycle {
    /// Returns the contract's code.
    pub(crate) const fn code(self) -> u8 {
        self as u8
    }

    /// Whether the lifecycle is final.
    pub(crate) const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Invalidated | Self::Superseded)
    }
}

/// One leg's mutable state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Leg {
    /// The per-series creation-order identifier.
    pub(crate) id: u64,
    /// The pivot scale.
    pub(crate) scale: crate::readings::PivotScale,
    /// +1 for an upward leg, -1 for a downward leg.
    pub(crate) direction: i8,
    /// The candidate high or low.
    pub(crate) origin_price: f64,
    /// The bar the pivot occurred on.
    pub(crate) origin_occurred_at: usize,
    /// The confirmation bar, once confirmed.
    pub(crate) origin_available_at: Option<usize>,
    /// Whether the origin pivot is confirmed.
    pub(crate) origin_confirmed: bool,
    /// A usable ATR 14 at the origin bar, if one exists.
    pub(crate) atr_at_origin: Option<f64>,
    /// The selected target, if one exists.
    pub(crate) target: Option<Target>,
    /// The number of target revisions.
    pub(crate) target_version: u32,
    /// Whether the target is stable.
    pub(crate) target_stable: bool,
    /// The bar of the most recent target revision, until it stabilises.
    revision_bar: Option<usize>,
    /// The lifecycle code.
    pub(crate) lifecycle: Lifecycle,
    /// `(Close - origin) / (target - origin)`, unclamped.
    pub(crate) progress: Option<f64>,
    /// `Close - origin`.
    pub(crate) displacement_price: Option<f64>,
    /// `(Close - origin) / ATR_at_origin`.
    pub(crate) displacement_atr: Option<f64>,
    /// The ordinal stage, 0..5 or null.
    pub(crate) stage: Option<i8>,
    /// Whether the most recent bar had no candle. A null progress then means
    /// the close was missing, not that the target degenerated.
    missing_close: bool,
    /// 1 when progress reached 1, 0 below it, null without progress.
    pub(crate) target_reached: Option<u8>,
    /// A forming leg's recorded close beyond its origin against it.
    pub(crate) breached: bool,
    /// Whether the terminal measurements are frozen.
    frozen: bool,
}

impl Leg {
    /// Creates a forming leg from a pivot candidate.
    ///
    /// The target is selected from the levels knowable at the candidate's own
    /// bar. The leg never reads a level that is not yet available.
    pub(crate) fn new(
        id: u64,
        candidate: PivotCandidate,
        index: &target::LevelIndex,
        atr_at_origin: Option<f64>,
    ) -> Self {
        let direction = match candidate.kind {
            PivotKind::Low => 1,
            PivotKind::High => -1,
        };
        let target = target::select_index(
            index,
            candidate.scale,
            direction,
            candidate.price,
            atr_at_origin,
        );

        Self {
            id,
            scale: candidate.scale,
            direction,
            origin_price: candidate.price,
            origin_occurred_at: candidate.occurred_at,
            origin_available_at: None,
            origin_confirmed: false,
            atr_at_origin,
            target,
            target_version: 0,
            target_stable: target.is_some(),
            revision_bar: None,
            lifecycle: Lifecycle::Forming,
            progress: None,
            displacement_price: None,
            displacement_atr: None,
            stage: None,
            missing_close: false,
            target_reached: None,
            breached: false,
            frozen: false,
        }
    }

    /// Confirms the origin at `bar`.
    ///
    /// A forming leg becomes active. A leg that is already terminal keeps its
    /// terminal lifecycle, because a terminal state is final, but it still
    /// records that its origin confirmed.
    pub(crate) fn confirm(&mut self, bar: usize) {
        self.origin_confirmed = true;
        self.origin_available_at = Some(bar);
        // A confirmed origin is no longer forming, so a breach recorded while
        // it was forming is spent: the active rule applies from this bar on.
        self.breached = false;
        if self.lifecycle == Lifecycle::Forming {
            self.lifecycle = Lifecycle::Active;
        }
    }

    /// Moves the leg to a terminal lifecycle.
    ///
    /// The measurements are computed for the current bar by [`Self::update`],
    /// which then freezes them.
    pub(crate) fn terminate(&mut self, lifecycle: Lifecycle) {
        if !lifecycle.is_terminal() {
            return;
        }
        self.lifecycle = lifecycle;
        self.breached = false;
    }

    /// Applies a newly selected target.
    ///
    /// The version and stability change only when the selected price or basis
    /// actually changes, so a confirmation that leaves the nearest target in
    /// place does not manufacture a revision.
    pub(crate) fn revise(&mut self, target: Target, bar: usize) {
        let changed = match self.target {
            Some(current) => {
                current.price != target.price || current.basis_code != target.basis_code
            }
            None => true,
        };
        if !changed {
            return;
        }

        self.target = Some(target);
        self.target_version = self.target_version.saturating_add(1);
        self.target_stable = false;
        self.revision_bar = Some(bar);
    }

    /// Whether `close` is beyond the origin against the leg by more than
    /// `0.2 * ATR_at_origin`.
    ///
    /// A leg without a usable ATR can never be invalidated on this rule.
    pub(crate) fn breach(&self, close: f64) -> bool {
        let Some(atr) = self.atr_at_origin else {
            return false;
        };
        let beyond = if self.direction > 0 {
            self.origin_price - close
        } else {
            close - self.origin_price
        };
        beyond > 0.2 * atr
    }

    /// Observes one closed bar.
    ///
    /// The target's stability clock keeps advancing after the measurements
    /// freeze, because stability is a property of the unchanged reference, not
    /// of the frozen measurements. A terminal leg revised shortly before it
    /// ended must still become stable after three unchanged closed bars.
    /// A frozen terminal leg's progress, displacement, target, stage, and
    /// target-reached fields are not changed. A missing candle clears the
    /// measurements but still advances the target's stability clock, because
    /// the bar itself is closed.
    pub(crate) fn update(&mut self, close: Option<f64>, bar: usize) {
        self.update_stability(bar);

        if self.frozen {
            return;
        }

        match close {
            Some(close) => {
                self.missing_close = false;
                self.measure(close);
            }
            None => {
                self.missing_close = true;
                self.progress = None;
                self.displacement_price = None;
                self.displacement_atr = None;
                self.target_reached = None;
            }
        }
        self.update_stage();

        if self.lifecycle.is_terminal() {
            self.frozen = true;
        }
    }

    /// Why the leg's progress is null, or `None` when it is available.
    ///
    /// A forming leg reports the unconfirmed origin before the target state,
    /// because the origin is what the leg is still waiting for. A terminal or
    /// active leg reports the target state: no target, a degenerate target, a
    /// missing close, or a non-finite computation. The missing-close case can
    /// outlive the missing bar on a frozen terminal leg, which is why it is
    /// recorded rather than inferred.
    pub(crate) fn unavailable_reason(&self) -> Option<UnavailableReason> {
        if self.progress.is_some() {
            return None;
        }
        if self.lifecycle == Lifecycle::Forming {
            return Some(UnavailableReason::OriginUnconfirmed);
        }
        match self.target {
            None => Some(UnavailableReason::NoTarget),
            Some(_) if self.target_is_degenerate() => Some(UnavailableReason::InvalidDenominator),
            Some(_) if self.missing_close => Some(UnavailableReason::InsufficientBars),
            // A usable target and a close still leave a non-finite progress
            // only when the arithmetic overflows, which is a denominator the
            // caller cannot use.
            Some(_) => Some(UnavailableReason::InvalidDenominator),
        }
    }

    /// Builds the published record.
    pub(crate) fn record(&self, timeframe_secs: Option<u64>) -> ProgressionLeg {
        ProgressionLeg {
            leg_id: self.id,
            timeframe_secs,
            scale_code: self.scale.code(),
            origin_price: self.origin_price,
            origin_occurred_at: self.origin_occurred_at,
            origin_available_at: self.origin_available_at,
            origin_confirmed_code: u8::from(self.origin_confirmed),
            direction_code: self.direction,
            target_price: self.target.map(|target| target.price),
            target_basis_code: self.target.map(|target| target.basis_code),
            target_version: self.target_version,
            target_stable_code: u8::from(self.target_stable),
            progress: self.progress,
            displacement_price: self.displacement_price,
            displacement_atr: self.displacement_atr,
            lifecycle_code: self.lifecycle.code(),
            stage_code: self.stage,
            target_reached_code: self.target_reached,
        }
    }

    fn update_stability(&mut self, bar: usize) {
        if let Some(revision_bar) = self.revision_bar {
            if bar >= revision_bar.saturating_add(3) {
                self.target_stable = true;
                self.revision_bar = None;
            }
        } else {
            self.target_stable = self.target.is_some();
        }
    }

    fn measure(&mut self, close: f64) {
        self.displacement_price = Some(close - self.origin_price);
        self.displacement_atr = self
            .atr_at_origin
            .map(|atr| (close - self.origin_price) / atr);

        let Some(target) = self.target else {
            self.progress = None;
            self.target_reached = None;
            return;
        };

        if self.target_is_degenerate() {
            self.progress = None;
            self.target_reached = None;
            return;
        }

        let progress = (close - self.origin_price) / (target.price - self.origin_price);
        if !progress.is_finite() {
            self.progress = None;
            self.target_reached = None;
            return;
        }

        self.progress = Some(progress);
        self.target_reached = Some(if progress >= 1.0 { 1 } else { 0 });
    }

    /// Whether the target is at the origin or closer than `0.2 * ATR_at_origin`.
    fn target_is_degenerate(&self) -> bool {
        let Some(target) = self.target else {
            return false;
        };
        let distance = (target.price - self.origin_price).abs();
        target.price == self.origin_price
            || self.atr_at_origin.is_some_and(|atr| distance < 0.2 * atr)
    }

    fn update_stage(&mut self) {
        if self.lifecycle.is_terminal() {
            self.stage = Some(5);
            return;
        }
        if self.lifecycle == Lifecycle::Forming || !self.target_stable {
            self.stage = None;
            return;
        }
        let Some(progress) = self.progress else {
            self.stage = None;
            return;
        };
        self.stage = Some(stage_for(progress));
    }
}

/// The contract's progress bucket: 0 below zero, then 0.25-wide bands, and 4
/// from 0.75 upward.
fn stage_for(progress: f64) -> i8 {
    if progress < 0.0 {
        0
    } else if progress < 0.25 {
        1
    } else if progress < 0.5 {
        2
    } else if progress < 0.75 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::{Leg, Lifecycle, stage_for};
    use crate::axes::progression::target::Target;
    use crate::axes::structure::registry::ConfirmedLevelRegistry;
    use crate::kernel::UnavailableReason;
    use crate::readings::{PivotCandidate, PivotKind, PivotScale, PivotSeries};

    fn empty_registry() -> ConfirmedLevelRegistry {
        ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&[]))
    }

    fn candidate(kind: PivotKind, price: f64, occurred_at: usize) -> PivotCandidate {
        PivotCandidate {
            scale: PivotScale::Minor,
            kind,
            price,
            occurred_at,
            available_at: occurred_at,
        }
    }

    fn forming_leg(direction: i8) -> Leg {
        let kind = if direction > 0 {
            PivotKind::Low
        } else {
            PivotKind::High
        };
        let registry = empty_registry();
        let mut index = super::target::LevelIndex::new(&registry);
        index.advance(4);
        Leg::new(1, candidate(kind, 100.0, 4), &index, None)
    }

    fn active_leg(origin: f64, target: f64, direction: i8, atr: Option<f64>) -> Leg {
        let mut leg = forming_leg(direction);
        leg.origin_price = origin;
        leg.target = Some(Target {
            price: target,
            basis_code: 1,
        });
        leg.target_stable = true;
        leg.atr_at_origin = atr;
        leg.confirm(6);
        leg
    }

    #[test]
    fn lifecycle_codes_match_the_contract() {
        assert_eq!(Lifecycle::Forming.code(), 0);
        assert_eq!(Lifecycle::Active.code(), 1);
        assert_eq!(Lifecycle::Completed.code(), 2);
        assert_eq!(Lifecycle::Invalidated.code(), 3);
        assert_eq!(Lifecycle::Superseded.code(), 4);
        assert!(Lifecycle::Completed.is_terminal());
        assert!(Lifecycle::Invalidated.is_terminal());
        assert!(Lifecycle::Superseded.is_terminal());
        assert!(!Lifecycle::Forming.is_terminal());
        assert!(!Lifecycle::Active.is_terminal());
    }

    #[test]
    fn a_candidate_creates_a_forming_leg_with_its_origin() {
        let leg = forming_leg(1);

        assert_eq!(leg.origin_price, 100.0);
        assert_eq!(leg.origin_occurred_at, 4);
        assert_eq!(leg.direction, 1);
        assert_eq!(leg.lifecycle, Lifecycle::Forming);
        assert!(!leg.origin_confirmed);
        assert_eq!(leg.origin_available_at, None);
        assert_eq!(leg.target, None);
        assert_eq!(leg.target_version, 0);
        assert_eq!(leg.stage, None);
    }

    #[test]
    fn confirmation_promotes_a_forming_leg_and_records_the_bar() {
        let mut leg = forming_leg(1);
        leg.confirm(6);

        assert_eq!(leg.lifecycle, Lifecycle::Active);
        assert!(leg.origin_confirmed);
        assert_eq!(leg.origin_available_at, Some(6));
    }

    #[test]
    fn confirmation_never_reopens_a_terminal_leg() {
        let mut leg = forming_leg(1);
        leg.terminate(Lifecycle::Completed);
        leg.confirm(6);

        assert_eq!(leg.lifecycle, Lifecycle::Completed);
        assert!(leg.origin_confirmed);
        assert_eq!(leg.origin_available_at, Some(6));
    }

    #[test]
    fn an_opposing_confirmation_completes_the_prior_leg() {
        let mut prior = forming_leg(1);
        prior.confirm(5);
        let mut legs = [prior, forming_leg(-1)];
        super::super::transition_prior(&mut legs, 1);

        assert_eq!(legs[0].lifecycle, Lifecycle::Completed);
        assert_eq!(legs[1].lifecycle, Lifecycle::Forming);
    }

    #[test]
    fn a_same_kind_confirmation_supersedes_the_prior_leg() {
        let mut prior = forming_leg(1);
        prior.confirm(5);
        let mut legs = [prior, forming_leg(1)];
        super::super::transition_prior(&mut legs, 1);

        assert_eq!(legs[0].lifecycle, Lifecycle::Superseded);
    }

    #[test]
    fn a_terminal_prior_leg_is_not_transitioned_again() {
        let mut prior = forming_leg(1);
        prior.confirm(5);
        prior.terminate(Lifecycle::Completed);
        let mut legs = [prior, forming_leg(-1)];
        super::super::transition_prior(&mut legs, 1);

        assert_eq!(legs[0].lifecycle, Lifecycle::Completed);
    }

    #[test]
    fn a_forming_prior_leg_is_not_transitioned() {
        let mut legs = [forming_leg(1), forming_leg(-1)];
        super::super::transition_prior(&mut legs, 1);

        assert_eq!(legs[0].lifecycle, Lifecycle::Forming);
    }

    #[test]
    fn the_most_recent_active_leg_is_transitioned_past_discarded_candidates() {
        let mut active = forming_leg(1);
        active.confirm(5);
        let mut discarded = forming_leg(-1);
        discarded.terminate(Lifecycle::Invalidated);
        let mut legs = [active, discarded, forming_leg(-1)];
        super::super::transition_prior(&mut legs, 2);

        assert_eq!(legs[0].lifecycle, Lifecycle::Completed);
        assert_eq!(legs[1].lifecycle, Lifecycle::Invalidated);
        assert_eq!(legs[2].lifecycle, Lifecycle::Forming);
    }

    #[test]
    fn progress_is_unclamped_in_both_directions() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));

        leg.update(Some(105.0), 7);
        assert_eq!(leg.progress, Some(0.5));
        assert_eq!(leg.displacement_price, Some(5.0));
        assert_eq!(leg.displacement_atr, Some(5.0));
        assert_eq!(leg.stage, Some(3));
        assert_eq!(leg.target_reached, Some(0));

        leg.update(Some(95.0), 8);
        assert_eq!(leg.progress, Some(-0.5));
        assert_eq!(leg.stage, Some(0));

        leg.update(Some(115.0), 9);
        assert_eq!(leg.progress, Some(1.5));
        assert_eq!(leg.stage, Some(4));
        assert_eq!(leg.target_reached, Some(1));
    }

    #[test]
    fn a_degenerate_target_nulls_progress_with_reason_three() {
        let mut leg = active_leg(100.0, 100.1, 1, Some(1.0));
        leg.update(Some(100.0), 7);

        assert_eq!(leg.progress, None);
        assert_eq!(leg.target_reached, None);
        assert_eq!(
            leg.unavailable_reason(),
            Some(UnavailableReason::InvalidDenominator)
        );
        assert_eq!(leg.stage, None);
    }

    #[test]
    fn a_missing_target_nulls_progress_with_reason_four() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.target = None;
        leg.update(Some(105.0), 7);

        assert_eq!(leg.progress, None);
        assert_eq!(leg.unavailable_reason(), Some(UnavailableReason::NoTarget));
        assert_eq!(leg.stage, None);
        assert_eq!(leg.displacement_price, Some(5.0));
    }

    #[test]
    fn a_missing_close_nulls_progress_with_reason_one() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.update(None, 7);

        assert_eq!(leg.progress, None);
        assert_eq!(leg.stage, None);
        assert_eq!(
            leg.unavailable_reason(),
            Some(UnavailableReason::InsufficientBars)
        );
    }

    #[test]
    fn a_terminal_leg_frozen_on_a_missing_close_keeps_reason_one() {
        // A terminal leg freezes on the bar it ends. When that bar has no
        // candle, the null progress is caused by the missing close, not by a
        // degenerate target, and later bars must not relabel it.
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.terminate(Lifecycle::Completed);
        leg.update(None, 7);
        leg.update(Some(105.0), 8);
        leg.update(Some(105.0), 9);

        assert_eq!(leg.stage, Some(5));
        assert_eq!(leg.progress, None);
        assert_eq!(
            leg.unavailable_reason(),
            Some(UnavailableReason::InsufficientBars)
        );
    }

    #[test]
    fn an_unconfirmed_origin_publishes_progress_but_no_stage() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.lifecycle = Lifecycle::Forming;
        leg.origin_confirmed = false;
        leg.origin_available_at = None;
        leg.update(Some(105.0), 7);

        assert_eq!(leg.progress, Some(0.5));
        assert_eq!(leg.stage, None);
        assert_eq!(leg.unavailable_reason(), None);
    }

    #[test]
    fn a_forming_leg_without_a_target_reports_reason_six() {
        let mut leg = forming_leg(1);
        leg.update(Some(105.0), 7);

        assert_eq!(leg.progress, None);
        assert_eq!(leg.stage, None);
        assert_eq!(
            leg.unavailable_reason(),
            Some(UnavailableReason::OriginUnconfirmed)
        );
    }

    #[test]
    fn a_forming_leg_reports_reason_six_before_the_target_reason() {
        // The origin is unconfirmed, so the axis reports reason 6 even when a
        // target exists and degenerates. This is what makes reason 6 reachable
        // on a leg created from a candidate, as §8.4 requires.
        let mut leg = active_leg(100.0, 100.0, 1, Some(1.0));
        leg.lifecycle = Lifecycle::Forming;
        leg.origin_confirmed = false;
        leg.origin_available_at = None;
        leg.update(Some(100.0), 7);

        assert_eq!(leg.progress, None);
        assert_eq!(
            leg.unavailable_reason(),
            Some(UnavailableReason::OriginUnconfirmed)
        );
    }

    #[test]
    fn a_confirmed_origin_forgets_the_forming_breach() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.lifecycle = Lifecycle::Forming;
        leg.origin_confirmed = false;
        leg.origin_available_at = None;
        leg.breached = true;

        leg.confirm(7);

        assert_eq!(leg.lifecycle, Lifecycle::Active);
        assert!(!leg.breached);
    }

    #[test]
    fn an_unstable_target_suppresses_the_stage() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.revise(
            Target {
                price: 108.0,
                basis_code: 1,
            },
            7,
        );
        leg.update(Some(105.0), 7);

        assert_eq!(leg.progress, Some(0.625));
        assert_eq!(leg.stage, None);
    }

    #[test]
    fn a_same_target_does_not_manufacture_a_revision() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.revise(
            Target {
                price: 110.0,
                basis_code: 1,
            },
            10,
        );

        assert_eq!(leg.target_version, 0);
        assert!(leg.target_stable);
        assert_eq!(leg.revision_bar, None);
    }

    #[test]
    fn stability_returns_on_the_third_unchanged_closed_bar() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        let next = Target {
            price: 108.0,
            basis_code: 1,
        };
        leg.revise(next, 10);
        assert!(!leg.target_stable);
        assert_eq!(leg.target_version, 1);

        leg.update(Some(105.0), 10);
        assert!(!leg.target_stable);
        leg.update(Some(105.0), 11);
        assert!(!leg.target_stable);
        leg.update(Some(105.0), 12);
        assert!(!leg.target_stable);
        leg.update(Some(105.0), 13);
        assert!(leg.target_stable);
        assert_eq!(leg.stage, Some(3));
    }

    #[test]
    fn a_second_revision_restarts_the_stability_clock() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.revise(
            Target {
                price: 108.0,
                basis_code: 1,
            },
            10,
        );
        leg.update(Some(105.0), 11);
        leg.update(Some(105.0), 12);
        assert!(!leg.target_stable);

        // A second revision at bar 12 must restart the three-bar count, so the
        // target does not stabilise at 13.
        leg.revise(
            Target {
                price: 107.0,
                basis_code: 1,
            },
            12,
        );
        assert_eq!(leg.target_version, 2);
        leg.update(Some(105.0), 13);
        leg.update(Some(105.0), 14);
        assert!(!leg.target_stable);
        leg.update(Some(105.0), 15);
        assert!(leg.target_stable);
    }

    #[test]
    fn a_terminal_leg_regains_target_stability() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.revise(
            Target {
                price: 108.0,
                basis_code: 1,
            },
            7,
        );
        leg.terminate(Lifecycle::Completed);

        leg.update(Some(105.0), 7);
        assert_eq!(leg.stage, Some(5));
        assert!(!leg.target_stable);

        leg.update(Some(200.0), 8);
        leg.update(Some(200.0), 9);
        assert!(!leg.target_stable);
        leg.update(Some(200.0), 10);
        assert!(leg.target_stable);

        // The measurements remain frozen even though the stability clock ran.
        assert_eq!(leg.progress, Some(0.625));
        assert_eq!(leg.displacement_price, Some(5.0));
        assert_eq!(leg.stage, Some(5));
    }

    #[test]
    fn a_revision_changes_progress_without_a_price_move() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        let close = 105.0;
        leg.update(Some(close), 7);
        assert_eq!(leg.progress, Some(0.5));

        leg.revise(
            Target {
                price: 108.0,
                basis_code: 1,
            },
            8,
        );
        leg.update(Some(close), 8);

        assert_eq!(leg.target_version, 1);
        assert_eq!(leg.progress, Some(0.625));
        assert_eq!(leg.displacement_price, Some(5.0));
    }

    #[test]
    fn a_revision_that_does_not_change_the_selection_does_not_increment() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.revise(
            Target {
                price: 110.0,
                basis_code: 1,
            },
            8,
        );

        assert_eq!(leg.target_version, 0);
        assert!(leg.target_stable);
    }

    #[test]
    fn the_breach_threshold_is_strict() {
        let leg = active_leg(100.0, 110.0, 1, Some(5.0));

        assert!(!leg.breach(99.0));
        assert!(leg.breach(98.99));
        assert!(!leg.breach(100.0));
        assert!(leg.breach(98.0));

        let down = active_leg(100.0, 90.0, -1, Some(5.0));
        assert!(!down.breach(101.0));
        assert!(down.breach(101.01));
    }

    #[test]
    fn a_leg_without_atr_is_never_invalidated_by_the_rule() {
        let leg = active_leg(100.0, 110.0, 1, None);
        assert!(!leg.breach(0.0));
    }

    #[test]
    fn a_terminal_leg_freezes_its_measurements() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.update(Some(105.0), 7);
        leg.terminate(Lifecycle::Completed);
        leg.update(Some(105.0), 7);
        assert_eq!(leg.stage, Some(5));
        assert_eq!(leg.progress, Some(0.5));

        leg.update(Some(200.0), 8);
        assert_eq!(leg.progress, Some(0.5));
        assert_eq!(leg.displacement_price, Some(5.0));
        assert_eq!(leg.stage, Some(5));
    }

    #[test]
    fn every_stage_boundary_has_its_contract_bucket() {
        assert_eq!(stage_for(-0.1), 0);
        assert_eq!(stage_for(-0.0), 1);
        assert_eq!(stage_for(0.0), 1);
        assert_eq!(stage_for(0.249), 1);
        assert_eq!(stage_for(0.25), 2);
        assert_eq!(stage_for(0.499), 2);
        assert_eq!(stage_for(0.5), 3);
        assert_eq!(stage_for(0.749), 3);
        assert_eq!(stage_for(0.75), 4);
        assert_eq!(stage_for(2.0), 4);
    }

    #[test]
    fn a_missing_candle_clears_the_measurements_but_not_the_target() {
        let mut leg = active_leg(100.0, 110.0, 1, Some(1.0));
        leg.update(Some(105.0), 7);
        leg.update(None, 8);

        assert_eq!(leg.progress, None);
        assert_eq!(leg.displacement_price, None);
        assert_eq!(leg.displacement_atr, None);
        assert_eq!(leg.target_version, 0);
    }
}
