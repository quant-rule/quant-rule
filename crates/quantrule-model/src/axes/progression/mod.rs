//! The Progression axis.
//!
//! Progression follows a structural leg from its pivot candidate through
//! confirmation, target selection, progress, and a terminal lifecycle. It keeps
//! a separate minor and a separate major leg, publishes the most recently
//! created retained leg whose origin is confirmed across the two scales, and
//! publishes the full record of the leg at each scale. A series with no
//! confirmed leg yet publishes its newest forming leg.
//!
//! The axis reads the pivot candidates and confirmed pivots from [`Readings`],
//! the ATR 14 value at the origin, and the close of each candle. It rebuilds the
//! confirmed-level registry from the pivots instead of reading the Structure
//! axis, so a correlated output cannot become a second vote. The module is
//! pure: it performs no I/O, reads no clock, and uses no randomness.
//!
//! The two scales are never blended. The published leg is one record chosen by
//! creation order among the confirmed legs, and the stage codes of the two legs
//! are never combined. A newer unconfirmed candidate does not displace a
//! confirmed leg: its origin may never exist.

mod leg;
mod target;

use crate::axes::structure::registry::ConfirmedLevelRegistry;
use crate::kernel::{
    PERCENTILE_WINDOW, Readiness, StateCounters, UnavailableReason, empirical_percentile,
};
use crate::readings::{PivotKind, PivotScale, Readings};
use quantrule_data::Candle;

use self::leg::{Leg, Lifecycle};

/// The published record of one progression leg.
///
/// The record carries the contract's §8.4 leg fields. `origin_available_at` is
/// null until the candidate confirms. `target_basis_code` is 1 for a confirmed
/// structural level and 2 for the ATR fallback. The code fields use the §14
/// registries; `progress` is unclamped and may be negative or greater than one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressionLeg {
    /// The per-series creation-order identifier.
    pub leg_id: u64,
    /// The timeframe in seconds, when the caller supplied one.
    pub timeframe_secs: Option<u64>,
    /// The pivot scale: 1 = minor, 2 = major.
    pub scale_code: u8,
    /// The candidate high or low that became the origin.
    pub origin_price: f64,
    /// The bar the origin pivot occurred on.
    pub origin_occurred_at: usize,
    /// The bar the origin became knowable, or `None` while it is forming.
    pub origin_available_at: Option<usize>,
    /// 1 when the origin pivot is confirmed, 0 while it is forming.
    pub origin_confirmed_code: u8,
    /// +1 for an upward leg from a low, -1 for a downward leg from a high.
    pub direction_code: i8,
    /// The selected target price, or `None` when no target is usable.
    pub target_price: Option<f64>,
    /// 1 = confirmed structural level, 2 = ATR fallback, or `None`.
    pub target_basis_code: Option<u8>,
    /// The number of target revisions since creation.
    pub target_version: u32,
    /// 1 when the target is stable, 0 when it is missing or unstable.
    pub target_stable_code: u8,
    /// `(Close - origin) / (target - origin)`, unclamped.
    pub progress: Option<f64>,
    /// `Close - origin`, or `None` when the close is unavailable.
    pub displacement_price: Option<f64>,
    /// `(Close - origin) / ATR_at_origin`, or `None` without a usable ATR.
    pub displacement_atr: Option<f64>,
    /// 0 = forming, 1 = active, 2 = completed, 3 = invalidated, 4 = superseded.
    pub lifecycle_code: u8,
    /// The ordinal stage 0..5, or `None` while it is unavailable.
    pub stage_code: Option<i8>,
    /// 1 when progress reached the target, 0 when it has not, or `None`.
    pub target_reached_code: Option<u8>,
}

/// The published Progression values for one closed bar.
///
/// `progress` and `stage_code` belong to the published leg. The minor and major
/// records are the most recently created retained leg at each scale, so the
/// other scale stays inspectable and the two are never blended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressionBar {
    /// The published leg's progress, unclamped.
    pub progress: Option<f64>,
    /// The published leg's ordinal stage 0..5, or `None`.
    pub stage_code: Option<i8>,
    /// The published leg's direction: +1 or -1, or `None` without a leg.
    pub leg_direction: Option<i8>,
    /// The published leg's scale: 1 or 2, or `None` without a leg.
    pub leg_scale_code: Option<u8>,
    /// The published leg record, or `None` when no leg exists.
    ///
    /// This is the leg whose progress, stage, direction, and scale the other
    /// fields describe: the newest leg whose origin is confirmed, or the newest
    /// forming leg before the first confirmation. It is not necessarily one of
    /// the two per-scale records below, because a newer candidate may be
    /// forming at the published leg's scale.
    pub published_leg: Option<ProgressionLeg>,
    /// The most recently created retained minor leg.
    pub minor_leg: Option<ProgressionLeg>,
    /// The most recently created retained major leg.
    pub major_leg: Option<ProgressionLeg>,
    /// Empirical percentile of the absolute published displacement in ATR
    /// units, in `0..=100`, or `None` with fewer than 50 valid prior samples.
    pub size_percentile: Option<f64>,
    /// Mechanical completeness of the published leg.
    pub readiness: Readiness,
    /// Why the axis is unavailable, or `None` when progress is available.
    pub unavailable_reason: Option<UnavailableReason>,
    /// Closed bars the published stage has held; zero before the first code.
    pub state_age_bars: u64,
    /// Published stage changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
}

impl ProgressionBar {
    /// The published leg record, or `None` when no leg exists.
    ///
    /// The record is the leg the axis published on this bar, so its id, target,
    /// origin, and lifecycle agree with the bar's published progress and stage.
    #[must_use]
    pub fn published_leg(&self) -> Option<ProgressionLeg> {
        self.published_leg
    }
}

/// The Progression configuration.
///
/// `timeframe_secs` is recorded on every leg. The axis does not derive it from
/// candle timestamps, because the timeframe is a property of the series, not of
/// the model arithmetic. The assembling layer supplies it; standalone use
/// leaves it null.
///
/// # Examples
///
/// ```
/// use quantrule_model::axes::progression::ProgressionConfig;
///
/// assert_eq!(ProgressionConfig::default().timeframe_secs, None);
/// assert_eq!(ProgressionConfig::new(300).timeframe_secs, Some(300));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProgressionConfig {
    /// The timeframe in seconds, or `None` when the caller does not supply it.
    pub timeframe_secs: Option<u64>,
}

impl ProgressionConfig {
    /// Builds a configuration for a timeframe of `timeframe_secs` seconds.
    #[must_use]
    pub const fn new(timeframe_secs: u64) -> Self {
        Self {
            timeframe_secs: Some(timeframe_secs),
        }
    }
}

/// The per-bar Progression series and its retained-leg ledger.
///
/// The series has exactly one [`ProgressionBar`] per input bar of the readings.
/// The ledger holds every retained leg in creation order with its current or
/// final record, including terminal legs that are no longer the per-scale
/// record. It is an inspection and test aid; the per-bar market-state output is
/// the two scale records in each bar.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::axes::progression::ProgressionSeries;
/// use quantrule_model::kernel::Readiness;
/// use quantrule_model::readings::Readings;
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let readings = Readings::compute(&[candle]);
/// let progression = ProgressionSeries::compute(&[candle], &readings);
///
/// assert_eq!(progression.len(), 1);
/// let bar = progression.get(0).expect("one bar");
/// assert_eq!(bar.progress, None);
/// assert_eq!(bar.readiness, Readiness::Unavailable);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressionSeries {
    bars: Vec<ProgressionBar>,
    legs: Vec<ProgressionLeg>,
}

impl ProgressionSeries {
    /// Computes Progression with no timeframe recorded on the legs.
    ///
    /// `candles` supplies the close used by progress and invalidation;
    /// `readings` supplies the pivot candidates, confirmed pivots, and ATR 14.
    /// The two are expected to describe the same series. A missing candle at a
    /// bar makes that bar's measurements null rather than panicking.
    #[must_use]
    pub fn compute(candles: &[Candle], readings: &Readings) -> Self {
        Self::compute_with_config(candles, readings, ProgressionConfig::default())
    }

    /// Computes Progression with an explicit configuration.
    ///
    /// The result is aligned to `readings.len()`. Each bar is a pure function
    /// of the readings knowable at that bar, the candle at that bar, and the
    /// leg state produced by earlier bars; no later bar is read.
    #[must_use]
    pub fn compute_with_config(
        candles: &[Candle],
        readings: &Readings,
        config: ProgressionConfig,
    ) -> Self {
        let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
        let mut level_index = target::LevelIndex::new(&registry);
        let mut scales = ScaleLegs::default();
        let mut bars = Vec::with_capacity(readings.len());
        let mut counters = StateCounters::new();
        // Absolute published displacement magnitudes in bar order. The current
        // bar is appended only after its percentile is computed, and the kernel
        // reads at most 252 samples, so the buffer is bounded.
        let mut magnitudes = Vec::with_capacity(PERCENTILE_WINDOW);
        let mut next_leg_id = 1_u64;

        for bar in 0..readings.len() {
            level_index.advance(bar);
            let close = candles.get(bar).map(Candle::close);

            // A confirmation promotes its own leg, transitions the most recent
            // active leg, and revises the targets that the newly confirmed
            // level can improve. The terminal prior leg freezes before the
            // revision pass, so it is not revised on the bar it ends.
            for scale in PivotScale::ALL {
                let Some(pivot) = readings.pivots.confirmed_at(scale, bar) else {
                    continue;
                };
                let new_direction = direction_for(pivot.kind);
                let legs = scales.get_mut(scale);
                let Some(index) = legs.iter().position(|leg| {
                    leg.origin_occurred_at == pivot.occurred_at && leg.direction == new_direction
                }) else {
                    continue;
                };

                transition_prior(legs, index);
                legs[index].confirm(bar);

                // A high can only target an upward leg; a low a downward leg.
                let target_direction = -new_direction;
                for leg in legs.iter_mut() {
                    if leg.lifecycle.is_terminal() || leg.direction != target_direction {
                        continue;
                    }
                    if let Some(target) = target::select_index(
                        &level_index,
                        scale,
                        leg.direction,
                        leg.origin_price,
                        leg.atr_at_origin,
                    ) {
                        leg.revise(target, bar);
                    }
                }
            }

            // A candidate is knowable at the close of its own bar, so the leg
            // and its target selection never use a later level.
            for scale in PivotScale::ALL {
                let Some(candidate) = readings.pivots.candidate(scale, bar) else {
                    continue;
                };
                let atr_at_origin = usable_atr(readings, candidate.occurred_at);
                let leg = Leg::new(next_leg_id, candidate, &level_index, atr_at_origin);
                next_leg_id = next_leg_id.saturating_add(1);
                let legs = scales.get_mut(scale);
                legs.push(leg);
                while legs.len() > MAX_RETAINED_LEGS {
                    let Some(index) = legs.iter().position(|leg| leg.lifecycle.is_terminal())
                    else {
                        break;
                    };
                    legs.remove(index);
                }
            }

            // Record a forming leg's breach, then resolve the candidates whose
            // lookback window closes on this bar. A confirmed candidate is no
            // longer forming, so it is not resolved here.
            if let Some(close) = close {
                for scale in PivotScale::ALL {
                    for leg in scales.get_mut(scale).iter_mut() {
                        if leg.lifecycle == Lifecycle::Forming && leg.breach(close) {
                            leg.breached = true;
                        }
                    }
                }
            }

            for scale in PivotScale::ALL {
                let lookback = scale.lookback();
                let legs = scales.get_mut(scale);
                let mut index = 0;
                while index < legs.len() {
                    let fails = legs[index].lifecycle == Lifecycle::Forming
                        && !legs[index].origin_confirmed
                        && legs[index].origin_occurred_at + lookback == bar;
                    if !fails {
                        index += 1;
                        continue;
                    }

                    if legs[index].breached && legs[index].atr_at_origin.is_some() {
                        legs[index].terminate(Lifecycle::Invalidated);
                        index += 1;
                    } else {
                        // The candidate did not confirm and never moved far
                        // enough against the leg to invalidate it.
                        legs.remove(index);
                    }
                }
            }

            if let Some(close) = close {
                for scale in PivotScale::ALL {
                    for leg in scales.get_mut(scale).iter_mut() {
                        if leg.lifecycle == Lifecycle::Active && leg.breach(close) {
                            leg.terminate(Lifecycle::Invalidated);
                        }
                    }
                }
            }

            // Every retained leg advances its stability clock and its
            // measurements. A leg terminated on this bar measures this bar and
            // then freezes; a leg terminated earlier is unchanged.
            for scale in PivotScale::ALL {
                for leg in scales.get_mut(scale).iter_mut() {
                    leg.update(close, bar);
                }
            }

            let published = scales.published();

            let progress = published.and_then(|leg| leg.progress);
            let stage_code = published.and_then(|leg| leg.stage);
            let magnitude = published
                .and_then(|leg| leg.displacement_atr)
                .map(f64::abs)
                .filter(|value| value.is_finite());
            let size_percentile =
                magnitude.and_then(|value| empirical_percentile(&magnitudes, value));

            let readiness = readiness(published, progress, stage_code, size_percentile);

            let unavailable_reason = if published.is_none() {
                // A leg begins from a candidate; without one there is no origin
                // to confirm.
                Some(UnavailableReason::OriginUnconfirmed)
            } else if progress.is_none() && close.is_none() {
                Some(UnavailableReason::InsufficientBars)
            } else if progress.is_none() {
                published.and_then(Leg::unavailable_reason)
            } else {
                // A frozen terminal leg keeps an available progress even when
                // this bar has no candle; an available value has no reason.
                None
            };

            let stats = counters.observe(stage_code);

            bars.push(ProgressionBar {
                progress,
                stage_code,
                leg_direction: published.map(|leg| leg.direction),
                leg_scale_code: published.map(|leg| leg.scale.code()),
                published_leg: published.map(|leg| leg.record(config.timeframe_secs)),
                minor_leg: scales
                    .minor
                    .last()
                    .map(|leg| leg.record(config.timeframe_secs)),
                major_leg: scales
                    .major
                    .last()
                    .map(|leg| leg.record(config.timeframe_secs)),
                size_percentile,
                readiness,
                unavailable_reason,
                state_age_bars: stats.age_bars,
                state_changes_50: stats.changes_50,
            });

            if let Some(value) = magnitude {
                if magnitudes.len() == PERCENTILE_WINDOW {
                    magnitudes.remove(0);
                }
                magnitudes.push(value);
            }
        }

        let mut ledger: Vec<Leg> = scales.minor.into_iter().chain(scales.major).collect();
        ledger.sort_by_key(|leg| leg.id);
        let legs = ledger
            .iter()
            .map(|leg| leg.record(config.timeframe_secs))
            .collect();

        Self { bars, legs }
    }

    /// The number of bars in the series.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Whether the series has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// The record at `bar`, or `None` when the bar is out of range.
    #[must_use]
    pub fn get(&self, bar: usize) -> Option<ProgressionBar> {
        self.bars.get(bar).copied()
    }

    /// Every bar record, in bar order.
    #[must_use]
    pub fn bars(&self) -> &[ProgressionBar] {
        &self.bars
    }

    /// The latest bar record, or `None` for an empty series.
    #[must_use]
    pub fn latest(&self) -> Option<ProgressionBar> {
        self.bars.last().copied()
    }

    /// Every retained leg, in creation order, with its current or final record.
    ///
    /// A leg that failed confirmation and was not invalidated is discarded and
    /// does not appear. Terminal legs that are no longer the per-scale record
    /// do appear, which is how the completed and superseded lifecycles remain
    /// inspectable.
    #[must_use]
    pub fn legs(&self) -> &[ProgressionLeg] {
        &self.legs
    }
}

/// The maximum number of legs retained per scale for inspection.
///
/// The per-scale published record is always the most recent leg. The bound
/// keeps the per-bar state machine linear on a long series; a terminal leg that
/// falls outside the recent window is no longer inspectable through `legs()`.
const MAX_RETAINED_LEGS: usize = 1_024;

/// The two per-scale leg lists.
#[derive(Debug, Clone, Default)]
struct ScaleLegs {
    minor: Vec<Leg>,
    major: Vec<Leg>,
}

impl ScaleLegs {
    fn get_mut(&mut self, scale: PivotScale) -> &mut Vec<Leg> {
        match scale {
            PivotScale::Minor => &mut self.minor,
            PivotScale::Major => &mut self.major,
        }
    }

    /// The leg the axis publishes.
    ///
    /// The newest leg whose origin is confirmed, across both scales; before any
    /// leg confirms, the newest leg overall. A newer unconfirmed candidate is
    /// not published, because its origin may never exist and its progress would
    /// describe a pivot that can disappear. The candidate stays visible in the
    /// per-scale record. Higher identifiers are newer, and a same-bar tie went
    /// to the major scale when the two were created.
    fn published(&self) -> Option<&Leg> {
        let newest = |confirmed_only: bool| {
            self.minor
                .iter()
                .chain(self.major.iter())
                .filter(|leg| !confirmed_only || leg.origin_confirmed)
                .max_by_key(|leg| leg.id)
        };
        newest(true).or_else(|| newest(false))
    }
}

/// The direction a pivot of `kind` creates: a low starts an upward leg, a high
/// a downward one.
const fn direction_for(kind: PivotKind) -> i8 {
    match kind {
        PivotKind::Low => 1,
        PivotKind::High => -1,
    }
}

/// The ATR 14 at `bar`, when it is a usable distance.
fn usable_atr(readings: &Readings, bar: usize) -> Option<f64> {
    readings
        .atr14
        .get(bar)
        .filter(|value| value.is_finite() && *value > 0.0)
}

/// The contract's mechanical completeness for one bar.
///
/// Ready requires an available progress, an available stage, a stable target,
/// and an available percentile. A valid progress with a missing stage, an
/// unstable target, or too few samples is provisional; a null progress is
/// unavailable. The target stability check is separate from the stage check
/// because a terminal leg publishes stage 5 while its target can still be
/// within the three-bar stability window.
fn readiness(
    published: Option<&Leg>,
    progress: Option<f64>,
    stage_code: Option<i8>,
    size_percentile: Option<f64>,
) -> Readiness {
    if published.is_none() || progress.is_none() {
        Readiness::Unavailable
    } else if stage_code.is_none() || !published.is_some_and(|leg| leg.target_stable) {
        Readiness::Provisional
    } else if size_percentile.is_some() {
        Readiness::Ready
    } else {
        Readiness::Provisional
    }
}

/// Ends the most recent active leg when a confirmation arrives.
///
/// The prior leg is completed when it points the other way and superseded when
/// it points the same way. Only an active leg is transitioned: a forming leg
/// has not confirmed its origin, and a terminal leg is final. Skipping the
/// failed candidates in between is what keeps an older active leg reachable;
/// otherwise a discarded candidate would shield it from the confirmation that
/// ends it.
pub(crate) fn transition_prior(legs: &mut [Leg], index: usize) {
    let direction = legs[index].direction;
    let Some(prior_index) = legs[..index]
        .iter()
        .rposition(|leg| leg.lifecycle == Lifecycle::Active)
    else {
        return;
    };
    let prior = &mut legs[prior_index];
    let lifecycle = if prior.direction == direction {
        Lifecycle::Superseded
    } else {
        Lifecycle::Completed
    };
    prior.terminate(lifecycle);
}

#[cfg(test)]
mod tests;
