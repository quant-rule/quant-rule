//! View-only Progression recent pace and average net pace.
//!
//! The model already publishes the published leg's displacement, lifecycle, and
//! displacement percentile. The dashboard needs a movement channel: how fast
//! the published leg's displacement changed over the fixed three-bar window,
//! signed in the leg's direction, and how that absolute speed ranks against its
//! own history. It also publishes the net displacement per elapsed bar since
//! the origin.
//!
//! The pace is same-leg: both endpoints must publish the same confirmed leg, so
//! a leg change, a forming origin, or a missing displacement is typed absence
//! rather than a fabricated zero. A terminal leg freezes both values at the bar
//! its published record becomes terminal; later bars keep the frozen value
//! instead of decaying toward zero. A leg change discards the freeze.
//!
//! The percentile adds one sample per newly measured pace, after that bar's
//! percentile opportunity, and never adds a carried-forward frozen value, so a
//! long terminal tail cannot count one measurement many times. The current bar
//! is never its own sample. The kernel owns the 252 cap, the 50-sample minimum,
//! and inclusive ties.
//!
//! This module is pure: it reads the published Progression bars up to the view
//! bar and performs no I/O, reads no clock, and uses no randomness. It does not
//! change the Progression axis; displacement percentile, progress, stage,
//! lifecycle, scale, and target state remain the model's values.

use crate::axes::progression::{ProgressionBar, ProgressionLeg};
use crate::kernel::{PERCENTILE_WINDOW, UnavailableReason, empirical_percentile};
use crate::readings::roles::AxisCode;
use crate::timeframe::{ModelBar, TimeframeModel};

use super::super::{
    AxisDetail, AxisState, AxisValue, PaceDirection, ProgressionDetail, RECENT_PACE_WINDOW_BARS,
    Stage,
};
use super::axis_state;

/// The recent-pace window as a slice index.
const PACE_WINDOW_BARS: usize = RECENT_PACE_WINDOW_BARS as usize;

/// The view-only Progression pace at one bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ProgressionPace {
    /// The signed three-bar displacement change per bar, in ATR units.
    pub(super) signed_atr_per_bar: Option<f64>,
    /// The sign of the signed pace relative to the leg.
    pub(super) direction: Option<PaceDirection>,
    /// Percentile of the absolute pace against prior measured paces.
    pub(super) speed_percentile: Option<f64>,
    /// Net displacement per elapsed bar since the origin.
    pub(super) average_net_displacement_atr_per_bar: Option<f64>,
    /// Whether the terminal pace and average are frozen.
    pub(super) frozen_terminal: bool,
}

impl ProgressionPace {
    /// The pace for a bar with no published leg and no measurement.
    const fn undefined() -> Self {
        Self {
            signed_atr_per_bar: None,
            direction: None,
            speed_percentile: None,
            average_net_displacement_atr_per_bar: None,
            frozen_terminal: false,
        }
    }
}

/// Computes the Progression pace at `view`.
///
/// `bars` is the full published Progression series. Only `bars[..=view]` is
/// read, so a later bar cannot influence this view. Returns the undefined pace
/// when `view` is not a bar of the series.
#[must_use]
pub(super) fn progression_pace(bars: &[ProgressionBar], view: usize) -> ProgressionPace {
    if view >= bars.len() {
        return ProgressionPace::undefined();
    }

    let mut tracker = PaceTracker::new();
    let mut samples = Vec::with_capacity(view.min(PERCENTILE_WINDOW));
    let mut current = ProgressionPace::undefined();

    for index in 0..=view {
        let leg = bars[index].published_leg();
        let prior = index
            .checked_sub(PACE_WINDOW_BARS)
            .and_then(|prior| bars.get(prior))
            .and_then(ProgressionBar::published_leg);
        let (state, newly_measured) = tracker.observe(index, leg, prior);
        current = state;

        // The sample is added after this bar's own percentile would have been
        // taken, and only when the bar measured a fresh pace. A frozen value
        // carried through later bars is one measurement, not one per bar.
        if index < view
            && newly_measured
            && let Some(signed) = current.signed_atr_per_bar
        {
            push_sample(&mut samples, signed.abs());
        }
    }

    let speed_percentile = current
        .signed_atr_per_bar
        .and_then(|signed| empirical_percentile(&samples, signed.abs()));

    ProgressionPace {
        speed_percentile,
        ..current
    }
}

/// Appends a finite absolute pace to the bounded sample window.
fn push_sample(samples: &mut Vec<f64>, sample: f64) {
    if !sample.is_finite() {
        return;
    }
    if samples.len() == PERCENTILE_WINDOW {
        samples.remove(0);
    }
    samples.push(sample);
}

/// The running published-leg state.
///
/// The tracker follows the published leg, not a scale record: the contract's
/// pace is defined for the leg the view publishes. A change of identifier is a
/// leg change, and it discards the terminal freeze.
struct PaceTracker {
    leg_id: Option<u64>,
    frozen: bool,
    signed: Option<f64>,
    average: Option<f64>,
}

impl PaceTracker {
    const fn new() -> Self {
        Self {
            leg_id: None,
            frozen: false,
            signed: None,
            average: None,
        }
    }

    /// Observes one bar and returns the pace plus whether this bar measured a
    /// fresh pace rather than carrying a frozen one.
    fn observe(
        &mut self,
        index: usize,
        leg: Option<ProgressionLeg>,
        prior: Option<ProgressionLeg>,
    ) -> (ProgressionPace, bool) {
        let Some(leg) = leg else {
            self.reset();
            return (ProgressionPace::undefined(), false);
        };

        if self.leg_id != Some(leg.leg_id) {
            self.reset();
            self.leg_id = Some(leg.leg_id);
        }

        if self.frozen {
            return (self.snapshot(), false);
        }

        let average = average_net_displacement(leg, index);
        let signed = recent_pace(leg, prior, index);
        let terminal = is_terminal(leg.lifecycle_code);
        self.signed = signed;
        self.average = average;
        self.frozen = terminal;

        (self.snapshot(), signed.is_some())
    }

    /// Clears the state for a missing or replaced leg.
    fn reset(&mut self) {
        self.leg_id = None;
        self.frozen = false;
        self.signed = None;
        self.average = None;
    }

    /// Builds the pace without a percentile; the caller adds that at the view.
    fn snapshot(&self) -> ProgressionPace {
        ProgressionPace {
            signed_atr_per_bar: self.signed,
            direction: self.signed.map(pace_direction),
            speed_percentile: None,
            average_net_displacement_atr_per_bar: self.average,
            frozen_terminal: self.frozen,
        }
    }
}

/// The signed three-bar displacement change for the same confirmed leg.
fn recent_pace(leg: ProgressionLeg, prior: Option<ProgressionLeg>, index: usize) -> Option<f64> {
    if leg.origin_confirmed_code != 1 {
        return None;
    }
    // A confirmed record is never used before the bar its origin became
    // knowable. The real model sets the availability bar at confirmation; the
    // guard also keeps a synthetic record from publishing a lookahead value.
    if !leg
        .origin_available_at
        .is_some_and(|available| available <= index)
    {
        return None;
    }
    let prior = prior?;
    if prior.leg_id != leg.leg_id || prior.origin_confirmed_code != 1 {
        return None;
    }
    // Both endpoints must be knowable at their own bars. The current-bar guard
    // keeps a synthetic record from publishing a lookahead value; the prior-bar
    // guard keeps the same rule for the other end of the window.
    let prior_index = index.saturating_sub(PACE_WINDOW_BARS);
    if !prior
        .origin_available_at
        .is_some_and(|available| available <= prior_index)
    {
        return None;
    }

    let current = leg.displacement_atr.filter(|value| value.is_finite())?;
    let previous = prior.displacement_atr.filter(|value| value.is_finite())?;
    let pace = f64::from(leg.direction_code) * (current - previous) / PACE_WINDOW_BARS as f64;
    pace.is_finite().then_some(pace)
}

/// The net displacement per elapsed bar since the origin.
///
/// The denominator is at least one, so a leg measured on its origin bar still
/// has a defined average. A missing or non-finite displacement is absent, not
/// zero.
fn average_net_displacement(leg: ProgressionLeg, index: usize) -> Option<f64> {
    let displacement = leg.displacement_atr.filter(|value| value.is_finite())?;
    let elapsed = index.saturating_sub(leg.origin_occurred_at).max(1);
    let average = displacement.abs() / elapsed as f64;
    average.is_finite().then_some(average)
}

/// Maps a signed pace to its factual direction.
fn pace_direction(signed: f64) -> PaceDirection {
    if signed > 0.0 {
        PaceDirection::WithLeg
    } else if signed < 0.0 {
        PaceDirection::AgainstLeg
    } else {
        PaceDirection::Flat
    }
}

/// Whether a lifecycle code is final.
const fn is_terminal(lifecycle_code: u8) -> bool {
    matches!(lifecycle_code, 2..=4)
}

/// Builds the Progression position from the model's stage code.
#[must_use]
pub(super) fn progression_position(progression: &ProgressionBar) -> AxisValue {
    match progression.stage_code.and_then(Stage::from_code) {
        Some(stage) => AxisValue::ordinal(stage),
        None => AxisValue::undefined(progression_stage_reason(progression)),
    }
}

/// The reason a published stage is absent.
///
/// The model reports a reason when progress is absent. A forming leg or an
/// unstable target can instead leave progress present while the stage is still
/// withheld; those cases name the stage's own blocker.
pub(super) fn progression_stage_reason(progression: &ProgressionBar) -> UnavailableReason {
    if let Some(reason) = progression.unavailable_reason {
        return reason;
    }
    let Some(leg) = progression.published_leg() else {
        return UnavailableReason::OriginUnconfirmed;
    };
    if leg.origin_confirmed_code == 0 {
        return UnavailableReason::OriginUnconfirmed;
    }
    if leg.target_price.is_none() {
        return UnavailableReason::NoTarget;
    }
    if leg.target_stable_code == 0 {
        return UnavailableReason::TargetUnstable;
    }
    UnavailableReason::InsufficientBars
}

/// Builds the Progression radius from the pace percentile.
///
/// A missing radius is an unconfirmed origin when the published leg is absent
/// or still forming; otherwise it is the percentile's own insufficient history.
#[must_use]
pub(super) fn pace_size(pace: &ProgressionPace, leg: Option<ProgressionLeg>) -> AxisValue {
    match pace.speed_percentile {
        Some(percentile) => AxisValue::rank(percentile),
        None => {
            let reason = if leg.is_some_and(|leg| leg.origin_confirmed_code == 1) {
                UnavailableReason::InsufficientBars
            } else {
                UnavailableReason::OriginUnconfirmed
            };
            AxisValue::undefined(reason)
        }
    }
}

/// Builds the Progression detail from the published model values and the pace.
#[must_use]
pub(super) fn progression_detail(
    progression: ProgressionBar,
    leg: Option<ProgressionLeg>,
    pace: ProgressionPace,
) -> ProgressionDetail {
    ProgressionDetail {
        progress: progression.progress,
        stage_code: progression.stage_code,
        leg_direction: progression.leg_direction,
        leg_scale_code: progression.leg_scale_code,
        lifecycle_code: leg.map(|leg| leg.lifecycle_code),
        target_version: leg.map(|leg| leg.target_version),
        target_stable: leg.map(|leg| leg.target_stable_code == 1),
        origin_bar: leg.map(|leg| leg.origin_occurred_at),
        origin_available_bar: leg.and_then(|leg| leg.origin_available_at),
        displacement_pct: progression.size_percentile,
        recent_pace_signed_atr_per_bar: pace.signed_atr_per_bar,
        recent_pace_direction: pace.direction,
        recent_pace_window_bars: RECENT_PACE_WINDOW_BARS,
        average_net_displacement_atr_per_bar: pace.average_net_displacement_atr_per_bar,
        recent_pace_frozen_terminal: pace.frozen_terminal,
    }
}

/// Builds the Progression axis from the model bar and its view-only pace.
pub(super) fn progression_axis(timeframe: &TimeframeModel, model_bar: &ModelBar) -> AxisState {
    let progression = model_bar.progression;
    let leg = progression.published_leg();
    let pace = progression_pace(timeframe.progression().bars(), model_bar.bar_index);

    axis_state(
        AxisCode::Progression,
        progression_position(&progression),
        pace_size(&pace, leg),
        progression.readiness,
        AxisDetail::Progression(progression_detail(progression, leg, pace)),
    )
}

#[cfg(test)]
mod tests;
