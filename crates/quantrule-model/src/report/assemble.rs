//! Assembly of [`MarketReport`] from a completed model run.
//!
//! The assembler is pure and deterministic. It reads each series in ascending
//! bar order, accumulates floating-point sums in that order, and sorts the
//! ladder and the chart events explicitly, so no unordered collection can
//! affect the result.
//!
//! The view selects and counts published values. It never recomputes an
//! indicator, an axis, an event, or a leg.

use crate::aggregate::{CrossTimeframeAggregates, EventSubtypeCounts};
use crate::axes::progression::ProgressionLeg;
use crate::events::EventRecord;
use crate::readings::roles::AxisCode;
use crate::run::ModelRun;
use crate::timeframe::{ModelBar, TimeframeModel};

use super::{
    AxisReport, ChartReport, EventReport, EventTypeCounts, LatestEvent, LatestState, MarketReport,
    ReportConsensus, ReportContext, ReportDirection, ReportLeg, ReportMomentum, ReportPeriod,
    ReportProgression, ReportReadings, ReportStructure, TimeframeEventReport, TimeframeReport,
};
use crate::events::order::event_cmp;

impl MarketReport {
    /// Projects a completed model run onto the full-period report.
    ///
    /// The ladder is sorted ascending and deduplicated. A ladder entry with no
    /// series in `run` produces a zero-bar [`TimeframeReport`]. The constructor
    /// never fails: an empty ladder produces an empty report.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::report::MarketReport;
    /// use quantrule_model::run::ModelRun;
    /// use quantrule_model::input::MarketStateConfig;
    ///
    /// use quantrule_model::aggregate::CrossTimeframeAggregates;
    ///
    /// let run = ModelRun::compute(&[], &MarketStateConfig::default());
    /// let aggregates = CrossTimeframeAggregates::compute(&run.snapshots());
    /// let report = MarketReport::compute(&run, &[300, 60, 300], &aggregates);
    ///
    /// assert_eq!(report.ladder, vec![60, 300]);
    /// assert_eq!(report.timeframes.len(), 2);
    /// assert_eq!(report.timeframes[0].bars, 0);
    /// ```
    #[must_use]
    pub fn compute(run: &ModelRun, ladder: &[u64], aggregates: &CrossTimeframeAggregates) -> Self {
        let mut ladder = ladder.to_vec();
        ladder.sort_unstable();
        ladder.dedup();

        let timeframes: Vec<TimeframeReport> = ladder
            .iter()
            .map(|&timeframe_secs| TimeframeReport::compute(run, timeframe_secs))
            .collect();
        let period = ReportPeriod::from_timeframes(&timeframes);
        // An empty ladder is an empty report. Carrying the run's aggregates
        // would publish timeframes the report does not list; a non-empty
        // ladder carries the existing aggregate unchanged, as the contract
        // requires, even when the run has a timeframe the ladder omits.
        let cross_timeframe = if ladder.is_empty() {
            CrossTimeframeAggregates::compute(&[])
        } else {
            aggregates.clone()
        };
        let events = EventReport::compute(run, &ladder);
        let charts = ChartReport::compute(run, &timeframes);

        Self {
            period,
            ladder,
            timeframes,
            cross_timeframe,
            events,
            charts,
        }
    }
}

impl ReportPeriod {
    /// Folds the present timeframes into the report's actual bounds.
    fn from_timeframes(timeframes: &[TimeframeReport]) -> Self {
        let first_open_time = timeframes
            .iter()
            .filter_map(|timeframe| timeframe.first_open_time)
            .min();
        let last_open_time = timeframes
            .iter()
            .filter_map(|timeframe| timeframe.last_open_time)
            .max();
        Self {
            first_open_time,
            last_open_time,
        }
    }
}

impl TimeframeReport {
    /// Builds one ladder member's report, present or absent.
    fn compute(run: &ModelRun, timeframe_secs: u64) -> Self {
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            return Self::absent(timeframe_secs);
        };

        let bars = timeframe.len();
        let first_open_time = timeframe.candle(0).map(|candle| candle.timestamp());
        let last_open_time = timeframe
            .candle(bars.saturating_sub(1))
            .map(|candle| candle.timestamp());
        let (expected_bars, missing_bars, coverage_share) =
            coverage(bars, first_open_time, last_open_time, timeframe_secs);

        Self {
            timeframe_secs,
            bars,
            expected_bars,
            missing_bars,
            coverage_share,
            first_open_time,
            last_open_time,
            axes: axis_reports(timeframe),
            latest: timeframe.latest_bar().map(|bar| latest_state(&bar)),
        }
    }

    /// A zero-bar report for a configured timeframe that is absent from the
    /// run.
    fn absent(timeframe_secs: u64) -> Self {
        Self {
            timeframe_secs,
            bars: 0,
            expected_bars: 0,
            missing_bars: 0,
            coverage_share: None,
            first_open_time: None,
            last_open_time: None,
            axes: AxisCode::ALL.iter().map(|&axis| empty_axis(axis)).collect(),
            latest: None,
        }
    }
}

/// Computes the coverage fields.
///
/// A well-formed series has unique ascending open times, so `bars` never
/// exceeds `expected_bars`. The ratio is clamped to `0..=1` and the missing
/// count saturates rather than panicking on a malformed series; the published
/// bar and expected counts still expose the defect.
fn coverage(
    bars: usize,
    first_open_time: Option<i64>,
    last_open_time: Option<i64>,
    timeframe_secs: u64,
) -> (usize, usize, Option<f64>) {
    let (Some(first), Some(last)) = (first_open_time, last_open_time) else {
        return (0, 0, None);
    };
    if timeframe_secs == 0 {
        return (0, 0, None);
    }
    let span = i128::from(last) - i128::from(first);
    let steps = span / i128::from(timeframe_secs);
    let expected_bars = if steps < 0 {
        1
    } else {
        usize::try_from(steps + 1).unwrap_or(usize::MAX).max(1)
    };
    let missing_bars = expected_bars.saturating_sub(bars);
    let coverage_share = if expected_bars == 0 {
        None
    } else {
        Some((bars as f64 / expected_bars as f64).min(1.0))
    };
    (expected_bars, missing_bars, coverage_share)
}

/// Builds the four axis reports in [`AxisCode::ALL`] order.
fn axis_reports(timeframe: &TimeframeModel) -> Vec<AxisReport> {
    let mut direction = AxisAccumulator::new(histogram_len(AxisCode::Direction));
    let mut momentum = AxisAccumulator::new(histogram_len(AxisCode::Momentum));
    let mut structure = AxisAccumulator::new(histogram_len(AxisCode::Structure));
    let mut progression = AxisAccumulator::new(histogram_len(AxisCode::Progression));

    for (((direction_bar, momentum_bar), structure_bar), progression_bar) in timeframe
        .direction()
        .bars()
        .iter()
        .zip(timeframe.momentum().bars())
        .zip(timeframe.structure().bars())
        .zip(timeframe.progression().bars())
    {
        direction.push(
            direction_bar.direction,
            direction_bar.state_code,
            compact_index,
        );
        momentum.push(momentum_bar.force, momentum_bar.state_code, compact_index);
        structure.push(
            structure_bar.score,
            structure_bar.state_code,
            structure_index,
        );
        progression.push(
            progression_bar.progress,
            progression_bar.stage_code,
            stage_index,
        );
    }

    vec![
        direction.finish(AxisCode::Direction),
        momentum.finish(AxisCode::Momentum),
        structure.finish(AxisCode::Structure),
        progression.finish(AxisCode::Progression),
    ]
}

/// The fixed histogram length for an axis.
fn histogram_len(axis: AxisCode) -> usize {
    match axis {
        AxisCode::Direction | AxisCode::Momentum => 3,
        AxisCode::Structure => 5,
        AxisCode::Progression => 6,
    }
}

/// An empty axis report for a timeframe with no bars.
fn empty_axis(axis: AxisCode) -> AxisReport {
    AxisReport {
        axis,
        present_count: 0,
        null_count: 0,
        min: None,
        max: None,
        mean: None,
        code_counts: vec![0; histogram_len(axis)],
        code_null_count: 0,
    }
}

/// The fixed code index for Direction and Momentum: `code + 1`.
fn compact_index(code: i8) -> Option<usize> {
    match code {
        -1 => Some(0),
        0 => Some(1),
        1 => Some(2),
        _ => None,
    }
}

/// The fixed code index for Structure: `code + 2`.
fn structure_index(code: i8) -> Option<usize> {
    match code {
        -2..=2 => Some((code + 2) as usize),
        _ => None,
    }
}

/// The fixed code index for Progression: `code`.
fn stage_index(code: i8) -> Option<usize> {
    match code {
        0..=5 => Some(code as usize),
        _ => None,
    }
}

/// Accumulates one axis's primary values and ordinal codes.
struct AxisAccumulator {
    present_count: usize,
    null_count: usize,
    min: Option<f64>,
    max: Option<f64>,
    sum: f64,
    code_counts: Vec<u32>,
    code_null_count: usize,
}

impl AxisAccumulator {
    fn new(histogram_len: usize) -> Self {
        Self {
            present_count: 0,
            null_count: 0,
            min: None,
            max: None,
            sum: 0.0,
            code_counts: vec![0; histogram_len],
            code_null_count: 0,
        }
    }

    /// Observes one bar's primary value and code.
    ///
    /// A code outside its documented range is a model defect. It is neither
    /// counted nor treated as null, so the histogram totals expose the
    /// mismatch instead of folding the value into a neighbouring bin.
    fn push(&mut self, primary: Option<f64>, code: Option<i8>, index: fn(i8) -> Option<usize>) {
        match primary {
            Some(value) => {
                self.present_count = self.present_count.saturating_add(1);
                self.min = Some(self.min.map_or(value, |current| {
                    if value.total_cmp(&current).is_lt() {
                        value
                    } else {
                        current
                    }
                }));
                self.max = Some(self.max.map_or(value, |current| {
                    if value.total_cmp(&current).is_gt() {
                        value
                    } else {
                        current
                    }
                }));
                self.sum += value;
            }
            None => self.null_count = self.null_count.saturating_add(1),
        }

        match code {
            Some(code) => {
                if let Some(slot) = index(code).and_then(|index| self.code_counts.get_mut(index)) {
                    *slot = slot.saturating_add(1);
                }
            }
            None => self.code_null_count = self.code_null_count.saturating_add(1),
        }
    }

    fn finish(self, axis: AxisCode) -> AxisReport {
        AxisReport {
            axis,
            present_count: self.present_count,
            null_count: self.null_count,
            min: self.min,
            max: self.max,
            mean: if self.present_count == 0 {
                None
            } else {
                Some(self.sum / self.present_count as f64)
            },
            code_counts: self.code_counts,
            code_null_count: self.code_null_count,
        }
    }
}

/// Maps the latest model bar onto the report's latest-state types.
fn latest_state(bar: &ModelBar) -> LatestState {
    LatestState {
        bar_index: bar.bar_index,
        open_time: bar.candle.timestamp(),
        readings: ReportReadings {
            ema20: bar.readings.ema20,
            ema50: bar.readings.ema50,
            ema200: bar.readings.ema200,
            rsi14: bar.readings.rsi14,
            atr14: bar.readings.atr14,
            rvol20: bar.readings.rvol20,
        },
        direction: ReportDirection {
            direction: bar.direction.direction,
            state_code: bar.direction.state_code,
            size_percentile: bar.direction.size_percentile,
            readiness_code: bar.direction.readiness.code(),
            reason_code: bar.direction.unavailable_reason.map(|reason| reason.code()),
            state_age_bars: bar.direction.state_age_bars,
            state_changes_50: bar.direction.state_changes_50,
        },
        momentum: ReportMomentum {
            force: bar.momentum.force,
            acceleration: bar.momentum.acceleration,
            intensification: bar.momentum.intensification,
            state_code: bar.momentum.state_code,
            size_percentile: bar.momentum.size_percentile,
            readiness_code: bar.momentum.readiness.code(),
            reason_code: bar.momentum.unavailable_reason.map(|reason| reason.code()),
            state_age_bars: bar.momentum.state_age_bars,
            state_changes_50: bar.momentum.state_changes_50,
        },
        structure: ReportStructure {
            high_relation_code: bar.structure.high_relation_code,
            low_relation_code: bar.structure.low_relation_code,
            score: bar.structure.score,
            state_code: bar.structure.state_code,
            scale_code: bar.structure.scale_code,
            ema_distance_atr_20: bar.structure.ema_distance_atr_20,
            ema_distance_atr_50: bar.structure.ema_distance_atr_50,
            ema_distance_atr_200: bar.structure.ema_distance_atr_200,
            ema_cross_code_20: bar.structure.ema_cross_code_20,
            ema_cross_code_50: bar.structure.ema_cross_code_50,
            ema_cross_code_200: bar.structure.ema_cross_code_200,
            ema_retest_code_20: bar.structure.ema_retest_code_20,
            ema_retest_code_50: bar.structure.ema_retest_code_50,
            ema_retest_code_200: bar.structure.ema_retest_code_200,
            size_percentile: bar.structure.size_percentile,
            readiness_code: bar.structure.readiness.code(),
            reason_code: bar.structure.unavailable_reason.map(|reason| reason.code()),
            state_age_bars: bar.structure.state_age_bars,
            state_changes_50: bar.structure.state_changes_50,
        },
        progression: ReportProgression {
            progress: bar.progression.progress,
            stage_code: bar.progression.stage_code,
            leg_direction: bar.progression.leg_direction,
            leg_scale_code: bar.progression.leg_scale_code,
            size_percentile: bar.progression.size_percentile,
            readiness_code: bar.progression.readiness.code(),
            reason_code: bar
                .progression
                .unavailable_reason
                .map(|reason| reason.code()),
            state_age_bars: bar.progression.state_age_bars,
            state_changes_50: bar.progression.state_changes_50,
            leg: bar.progression.published_leg().map(|leg| report_leg(&leg)),
        },
        context: ReportContext {
            atr_ratio: bar.context.atr_ratio,
            rvol_ratio: bar.context.rvol_ratio,
            context_code: bar.context.context_code,
        },
        consensus: ReportConsensus {
            bullish_axis_count: bar.consensus.bullish_axis_count,
            bearish_axis_count: bar.consensus.bearish_axis_count,
            neutral_axis_count: bar.consensus.neutral_axis_count,
            weakest_directional_axis_code: bar.consensus.weakest_directional_axis_code,
        },
    }
}

/// Maps one published progression leg onto the report type.
fn report_leg(leg: &ProgressionLeg) -> ReportLeg {
    ReportLeg {
        leg_id: leg.leg_id,
        timeframe_secs: leg.timeframe_secs,
        scale_code: leg.scale_code,
        origin_price: leg.origin_price,
        origin_occurred_at: leg.origin_occurred_at,
        origin_available_at: leg.origin_available_at,
        origin_confirmed_code: leg.origin_confirmed_code,
        direction_code: leg.direction_code,
        target_price: leg.target_price,
        target_basis_code: leg.target_basis_code,
        target_version: leg.target_version,
        target_stable_code: leg.target_stable_code,
        progress: leg.progress,
        displacement_price: leg.displacement_price,
        displacement_atr: leg.displacement_atr,
        lifecycle_code: leg.lifecycle_code,
        stage_code: leg.stage_code,
        target_reached_code: leg.target_reached_code,
    }
}

impl EventReport {
    /// Counts every event in the full run, per timeframe and overall.
    fn compute(run: &ModelRun, ladder: &[u64]) -> Self {
        let mut timeframes = Vec::with_capacity(ladder.len());
        let mut by_type = EventTypeCounts::default();
        let mut by_subtype = EventSubtypeCounts::default();

        for &timeframe_secs in ladder {
            let (counts, subtypes, latest) = match run.timeframe(timeframe_secs) {
                Some(timeframe) => count_events(timeframe.events()),
                None => (
                    EventTypeCounts::default(),
                    EventSubtypeCounts::default(),
                    None,
                ),
            };
            by_type.add_counts(counts);
            by_subtype.add_counts(&subtypes);
            timeframes.push(TimeframeEventReport {
                timeframe_secs,
                total: counts.total(),
                by_type: counts,
                by_subtype: subtypes,
                latest,
            });
        }

        Self {
            total: by_type.total(),
            by_type,
            by_subtype,
            timeframes,
        }
    }
}

/// Counts one timeframe's events once, at their availability bar.
fn count_events(
    events: &[EventRecord],
) -> (EventTypeCounts, EventSubtypeCounts, Option<LatestEvent>) {
    let mut by_type = EventTypeCounts::default();
    let mut by_subtype = EventSubtypeCounts::default();
    let mut latest: Option<&EventRecord> = None;

    for event in events {
        by_type.add_code(event.event_type.code());
        by_subtype.add_event(event);
        if latest.is_none_or(|current| event_cmp(event, current).is_gt()) {
            latest = Some(event);
        }
    }

    (by_type, by_subtype, latest.map(latest_event))
}

/// Maps one event onto the latest-event type.
fn latest_event(event: &EventRecord) -> LatestEvent {
    LatestEvent {
        event_type_code: event.event_type.code(),
        event_subtype_code: event.event_subtype_code,
        source_code: event.source_code,
        occurred_bar_index: event.occurred_bar_index,
        available_bar_index: event.available_bar_index,
        direction_code: event.direction_code,
        reference_code: event.reference_code,
        primary_value: event.primary_value,
        reference_value: event.reference_value,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AxisAccumulator, compact_index, count_events, coverage, stage_index, structure_index,
    };
    use crate::events::{EventRecord, EventType};
    use crate::readings::roles::AxisCode;

    #[test]
    fn coverage_counts_internal_gaps() {
        let (expected, missing, share) = coverage(4, Some(0), Some(240), 60);
        assert_eq!(expected, 5);
        assert_eq!(missing, 1);
        assert_eq!(share, Some(0.8));
    }

    #[test]
    fn coverage_is_one_for_a_single_bar() {
        let (expected, missing, share) = coverage(1, Some(100), Some(100), 60);
        assert_eq!(expected, 1);
        assert_eq!(missing, 0);
        assert_eq!(share, Some(1.0));
    }

    #[test]
    fn coverage_is_null_without_a_bar() {
        assert_eq!(coverage(0, None, None, 60), (0, 0, None));
    }

    #[test]
    fn a_malformed_series_does_not_panic() {
        let (expected, missing, share) = coverage(4, Some(240), Some(0), 60);
        assert_eq!(expected, 1);
        assert_eq!(missing, 0);
        assert_eq!(share, Some(1.0));

        let (expected, missing, share) = coverage(2, Some(0), Some(60), 0);
        assert_eq!((expected, missing, share), (0, 0, None));
    }

    #[test]
    fn axis_accumulator_counts_present_null_and_histogram() {
        let mut axis = AxisAccumulator::new(3);
        axis.push(Some(0.0), Some(0), compact_index);
        axis.push(Some(1.0), Some(1), compact_index);
        axis.push(None, None, compact_index);
        axis.push(Some(-1.0), Some(-1), compact_index);

        let report = axis.finish(AxisCode::Direction);
        assert_eq!(report.present_count, 3);
        assert_eq!(report.null_count, 1);
        assert_eq!(report.min, Some(-1.0));
        assert_eq!(report.max, Some(1.0));
        assert_eq!(report.mean, Some(0.0));
        assert_eq!(report.code_counts, vec![1, 1, 1]);
        assert_eq!(report.code_null_count, 1);
    }

    #[test]
    fn a_zero_primary_value_is_present_not_missing() {
        let mut axis = AxisAccumulator::new(5);
        axis.push(Some(0.0), Some(0), structure_index);

        let report = axis.finish(AxisCode::Structure);
        assert_eq!(report.present_count, 1);
        assert_eq!(report.null_count, 0);
        assert_eq!(report.min, Some(0.0));
        assert_eq!(report.max, Some(0.0));
        assert_eq!(report.mean, Some(0.0));
        assert_eq!(report.code_counts, vec![0, 0, 1, 0, 0]);
    }

    #[test]
    fn progress_and_stage_are_counted_independently() {
        let mut axis = AxisAccumulator::new(6);
        axis.push(Some(0.5), None, stage_index);

        let report = axis.finish(AxisCode::Progression);
        assert_eq!(report.present_count, 1);
        assert_eq!(report.code_null_count, 1);
        assert_eq!(report.code_counts, vec![0; 6]);
    }

    #[test]
    fn an_out_of_range_code_is_not_folded_into_a_bin() {
        let mut axis = AxisAccumulator::new(3);
        axis.push(None, Some(7), compact_index);
        axis.push(None, Some(0), compact_index);

        let report = axis.finish(AxisCode::Direction);
        assert_eq!(report.null_count, 2);
        assert_eq!(report.code_counts, vec![0, 1, 0]);
        assert_eq!(report.code_null_count, 0);
    }

    #[test]
    fn the_fixed_code_indices_match_the_contract() {
        assert_eq!(compact_index(-1), Some(0));
        assert_eq!(compact_index(0), Some(1));
        assert_eq!(compact_index(1), Some(2));
        assert_eq!(compact_index(2), None);

        assert_eq!(structure_index(-2), Some(0));
        assert_eq!(structure_index(0), Some(2));
        assert_eq!(structure_index(2), Some(4));
        assert_eq!(structure_index(-3), None);

        assert_eq!(stage_index(0), Some(0));
        assert_eq!(stage_index(5), Some(5));
        assert_eq!(stage_index(-1), None);
        assert_eq!(stage_index(6), None);
    }

    #[test]
    fn the_latest_event_uses_availability_then_occurrence_then_type() {
        let older = record(5, 5, EventType::Level, 1);
        let later_occurrence = record(6, 1, EventType::Level, 1);
        let later_type = record(6, 1, EventType::Signal, 1);
        let events = [older, later_type, later_occurrence];

        let (_, _, latest) = count_events(&events);
        let latest = latest.expect("one event is the latest");
        assert_eq!(latest.available_bar_index, 6);
        assert_eq!(latest.occurred_bar_index, 1);
        assert_eq!(latest.event_type_code, EventType::Signal.code());
    }

    fn record(
        available_bar_index: usize,
        occurred_bar_index: usize,
        event_type: EventType,
        event_subtype_code: u8,
    ) -> EventRecord {
        EventRecord {
            event_type,
            event_subtype_code,
            source_code: 1,
            timeframe_secs: 60,
            occurred_bar_index,
            available_bar_index,
            direction_code: 1,
            reference_code: None,
            primary_value: Some(1.0),
            reference_value: None,
        }
    }
}
