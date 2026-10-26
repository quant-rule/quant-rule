//! View-only Direction bias tenure.
//!
//! The model already publishes a signed Direction score and a compact state
//! age. The dashboard needs a different measure: how many observed closed bars
//! the current bullish, bearish, or neutral bias has held since its last genuine
//! origin. A neutral pause keeps the prior bias tenure; an unavailable bar
//! freezes the counter; an opposite sign after a gap starts a new origin at the
//! first bar that is actually observed, never backdates the reversal, and keeps
//! the gap visible in the origin kind.
//!
//! The percentile compares the running tenure with the tenure published on each
//! prior observed bar. A bar without a tenure contributes no sample, and the
//! current bar is never its own sample, so the radius answers "how unusual is
//! this age against the observed age history" rather than "how long did past
//! completed biases last".
//!
//! This module is pure: it reads the published Direction bars up to the view
//! bar and performs no I/O, reads no clock, and uses no randomness. It does not
//! change the Direction axis; the model's existing magnitude percentile remains
//! available as detail.

use crate::axes::direction::DirectionBar;
use crate::kernel::{COMPACT_STATE_THRESHOLD, PERCENTILE_WINDOW, empirical_percentile};

use super::super::{BiasContinuity, BiasState, TenureOriginKind};

/// The Direction evidence computed from the bars up to one view bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DirectionTenure {
    /// The current bias state.
    pub(super) bias_state: BiasState,
    /// Whether the current tenure was interrupted by an unavailable interval.
    pub(super) bias_continuity: BiasContinuity,
    /// Observed bars since the origin, or `None` while unavailable or before
    /// the first establishment.
    pub(super) tenure_bars: Option<u64>,
    /// What established the current tenure.
    pub(super) origin_kind: TenureOriginKind,
    /// The bar that established the current tenure.
    pub(super) origin_bar: Option<usize>,
    /// Percentile of the current tenure against prior valid tenures.
    pub(super) tenure_percentile: Option<f64>,
}

impl DirectionTenure {
    /// The evidence for a bar with no established bias and no measurement.
    const fn undefined() -> Self {
        Self {
            bias_state: BiasState::Undefined,
            bias_continuity: BiasContinuity::Undefined,
            tenure_bars: None,
            origin_kind: TenureOriginKind::None,
            origin_bar: None,
            tenure_percentile: None,
        }
    }
}

/// The bias state of one signed Direction value.
///
/// The corridor is inclusive, so a measured `+0.1` or `-0.1` is neutral, not a
/// weak bullish or bearish reading.
#[must_use]
pub(super) fn bias_state(direction: Option<f64>) -> BiasState {
    match direction {
        None => BiasState::Undefined,
        Some(value) if value > COMPACT_STATE_THRESHOLD => BiasState::Bullish,
        Some(value) if value < -COMPACT_STATE_THRESHOLD => BiasState::Bearish,
        Some(_) => BiasState::Neutral,
    }
}

/// Computes the bias tenure and its percentile at `bar`.
///
/// `bars` is the full Direction series. Only `bars[..=bar]` is read, so a later
/// bar cannot influence this view. Returns undefined evidence when `bar` is not
/// a bar of the series.
#[must_use]
pub(super) fn direction_tenure(bars: &[DirectionBar], bar: usize) -> DirectionTenure {
    if bar >= bars.len() {
        return DirectionTenure::undefined();
    }

    let mut tracker = TenureTracker::new();
    let mut samples = Vec::with_capacity(bar.min(PERCENTILE_WINDOW));
    let mut current = DirectionTenure::undefined();

    for (index, record) in bars.iter().take(bar.saturating_add(1)).enumerate() {
        current = tracker.observe(index, record.direction);

        // The current bar is never one of its own percentile samples. The
        // kernel applies the 252 cap and the 50-sample minimum.
        if index < bar
            && let Some(tenure) = current.tenure_bars
        {
            if samples.len() == PERCENTILE_WINDOW {
                samples.remove(0);
            }
            samples.push(tenure as f64);
        }
    }

    let tenure_percentile = current
        .tenure_bars
        .and_then(|tenure| empirical_percentile(&samples, tenure as f64));

    DirectionTenure {
        tenure_percentile,
        ..current
    }
}

/// The running tenure state.
///
/// `side` keeps the sign of the origin, not the sign of the current bar, so a
/// neutral pause can compare a later return against the bias it paused.
struct TenureTracker {
    tenure: Option<u64>,
    origin_bar: Option<usize>,
    origin_kind: TenureOriginKind,
    side: Option<i8>,
    interrupted: bool,
    previous_bar_unavailable: bool,
}

impl TenureTracker {
    const fn new() -> Self {
        Self {
            tenure: None,
            origin_bar: None,
            origin_kind: TenureOriginKind::None,
            side: None,
            interrupted: false,
            previous_bar_unavailable: false,
        }
    }

    /// Observes one bar and returns the evidence at that bar.
    fn observe(&mut self, bar: usize, direction: Option<f64>) -> DirectionTenure {
        let Some(value) = direction else {
            self.previous_bar_unavailable = true;
            if self.tenure.is_some() {
                self.interrupted = true;
                return DirectionTenure {
                    bias_state: BiasState::Undefined,
                    bias_continuity: BiasContinuity::Interrupted,
                    tenure_bars: None,
                    origin_kind: self.origin_kind,
                    origin_bar: self.origin_bar,
                    tenure_percentile: None,
                };
            }
            return DirectionTenure {
                bias_state: BiasState::Undefined,
                bias_continuity: BiasContinuity::Undefined,
                tenure_bars: None,
                origin_kind: TenureOriginKind::None,
                origin_bar: None,
                tenure_percentile: None,
            };
        };

        let was_unavailable = self.previous_bar_unavailable;
        self.previous_bar_unavailable = false;
        let state = bias_state(Some(value));

        match self.tenure {
            None => {
                if state == BiasState::Neutral {
                    // A neutral reading before any non-neutral bias establishes
                    // no origin, so it has no tenure to preserve.
                    return DirectionTenure {
                        bias_state: BiasState::Neutral,
                        bias_continuity: BiasContinuity::Continuous,
                        tenure_bars: None,
                        origin_kind: TenureOriginKind::None,
                        origin_bar: None,
                        tenure_percentile: None,
                    };
                }
                self.establish(bar, state, TenureOriginKind::FirstEstablishment);
                self.evidence(state, BiasContinuity::Continuous)
            }
            Some(tenure) => match state {
                BiasState::Neutral => {
                    // A neutral pause is not a reset: the counter keeps
                    // advancing and the origin stays where it was.
                    self.tenure = Some(tenure.saturating_add(1));
                    self.evidence(
                        state,
                        if self.interrupted {
                            BiasContinuity::Interrupted
                        } else {
                            BiasContinuity::Continuous
                        },
                    )
                }
                BiasState::Bullish | BiasState::Bearish if self.side == Some(sign(state)) => {
                    self.tenure = Some(tenure.saturating_add(1));
                    self.evidence(
                        state,
                        if self.interrupted {
                            BiasContinuity::Interrupted
                        } else {
                            BiasContinuity::Continuous
                        },
                    )
                }
                BiasState::Bullish | BiasState::Bearish => {
                    // The previous bar decides whether this is an observed
                    // reversal or one that happened somewhere inside a gap.
                    // In the second case the new origin is the first observed
                    // bar, never a bar the caller cannot justify.
                    let kind = if was_unavailable {
                        TenureOriginKind::NewOriginAfterUnavailable
                    } else {
                        TenureOriginKind::Reversal
                    };
                    self.establish(bar, state, kind);
                    // A fresh origin starts a fresh uninterrupted tenure. The
                    // gap remains visible in the origin kind.
                    self.evidence(state, BiasContinuity::Continuous)
                }
                BiasState::Undefined => {
                    // The caller only reaches this arm with a measured value,
                    // which cannot map to Undefined. Keep the state explicit
                    // instead of inventing a reversal if that changes.
                    self.evidence(
                        state,
                        if self.interrupted {
                            BiasContinuity::Interrupted
                        } else {
                            BiasContinuity::Continuous
                        },
                    )
                }
            },
        }
    }

    /// Starts a new tenure at `bar`.
    fn establish(&mut self, bar: usize, state: BiasState, kind: TenureOriginKind) {
        self.tenure = Some(1);
        self.origin_bar = Some(bar);
        self.origin_kind = kind;
        self.side = Some(sign(state));
        self.interrupted = false;
    }

    /// Builds the evidence for a continuing tenure.
    fn evidence(&self, state: BiasState, continuity: BiasContinuity) -> DirectionTenure {
        DirectionTenure {
            bias_state: state,
            bias_continuity: continuity,
            tenure_bars: self.tenure,
            origin_kind: self.origin_kind,
            origin_bar: self.origin_bar,
            tenure_percentile: None,
        }
    }
}

/// The sign of a directional state. Neutral is not a side.
const fn sign(state: BiasState) -> i8 {
    match state {
        BiasState::Bullish => 1,
        BiasState::Bearish => -1,
        BiasState::Neutral | BiasState::Undefined => 0,
    }
}

#[cfg(test)]
mod tests;
