use super::events::EVENT_WINDOW_BARS;
use super::levels::MAX_REFERENCES;
use super::progression::ProgressionAggregates;
use super::{CrossTimeframeAggregates, EventSubtypeCounts, ReferenceSet, TimeframeSnapshot};
use crate::axes::direction::DirectionBar;
use crate::axes::momentum::MomentumBar;
use crate::axes::progression::{ProgressionBar, ProgressionLeg};
use crate::axes::structure::StructureBar;
use crate::context::ContextBar;
use crate::events::{EventRecord, EventType};
use crate::kernel::{Readiness, UnavailableReason};

fn direction(score: Option<f64>, readiness: Readiness) -> DirectionBar {
    DirectionBar {
        direction: score,
        state_code: score.map(|value| {
            if value > 0.1 {
                1
            } else if value < -0.1 {
                -1
            } else {
                0
            }
        }),
        size_percentile: None,
        readiness,
        unavailable_reason: score
            .is_none()
            .then_some(UnavailableReason::InsufficientBars),
        state_age_bars: 0,
        state_changes_50: 0,
    }
}

fn momentum(force: Option<f64>, acceleration: Option<f64>, readiness: Readiness) -> MomentumBar {
    MomentumBar {
        force,
        acceleration,
        intensification: None,
        state_code: force.map(|value| {
            if value > 0.1 {
                1
            } else if value < -0.1 {
                -1
            } else {
                0
            }
        }),
        size_percentile: None,
        readiness,
        unavailable_reason: force
            .is_none()
            .then_some(UnavailableReason::InsufficientBars),
        state_age_bars: 0,
        state_changes_50: 0,
    }
}

fn structure(
    score: Option<f64>,
    code: Option<i8>,
    high: Option<i8>,
    low: Option<i8>,
    readiness: Readiness,
) -> StructureBar {
    StructureBar {
        high_relation_code: high,
        low_relation_code: low,
        score,
        state_code: code,
        scale_code: 2,
        ema_distance_atr_20: None,
        ema_distance_atr_50: None,
        ema_distance_atr_200: None,
        ema_cross_code_20: None,
        ema_cross_code_50: None,
        ema_cross_code_200: None,
        ema_retest_code_20: None,
        ema_retest_code_50: None,
        ema_retest_code_200: None,
        size_percentile: None,
        readiness,
        unavailable_reason: score
            .is_none()
            .then_some(UnavailableReason::InsufficientBars),
        state_age_bars: 0,
        state_changes_50: 0,
    }
}

fn leg(
    id: u64,
    scale: u8,
    direction: i8,
    origin_price: f64,
    lifecycle: u8,
    stage: Option<i8>,
) -> ProgressionLeg {
    ProgressionLeg {
        leg_id: id,
        timeframe_secs: None,
        scale_code: scale,
        origin_price,
        origin_occurred_at: 10,
        origin_available_at: Some(12),
        origin_confirmed_code: 1,
        direction_code: direction,
        target_price: Some(origin_price + 10.0),
        target_basis_code: Some(2),
        target_version: 0,
        target_stable_code: 1,
        progress: Some(0.2),
        displacement_price: Some(1.0),
        displacement_atr: Some(0.5),
        lifecycle_code: lifecycle,
        stage_code: stage,
        target_reached_code: Some(0),
    }
}

fn progression(
    minor: Option<ProgressionLeg>,
    major: Option<ProgressionLeg>,
    readiness: Readiness,
) -> ProgressionBar {
    ProgressionBar {
        progress: minor.or(major).and_then(|leg| leg.progress),
        stage_code: minor.or(major).and_then(|leg| leg.stage_code),
        leg_direction: minor.or(major).map(|leg| leg.direction_code),
        leg_scale_code: minor.or(major).map(|leg| leg.scale_code),
        minor_leg: minor,
        major_leg: major,
        size_percentile: None,
        readiness,
        unavailable_reason: None,
        state_age_bars: 0,
        state_changes_50: 0,
    }
}

fn snapshot(timeframe_secs: u64) -> TimeframeSnapshot {
    TimeframeSnapshot {
        timeframe_secs,
        latest_bar_index: 100,
        direction: direction(None, Readiness::Unavailable),
        momentum: momentum(None, None, Readiness::Unavailable),
        structure: structure(None, None, None, None, Readiness::Unavailable),
        progression: progression(None, None, Readiness::Unavailable),
        context: ContextBar {
            atr_ratio: None,
            rvol_ratio: None,
            context_code: None,
        },
        atr: Some(1.0),
        references: ReferenceSet::new(Vec::new()),
        events: Vec::new(),
        minor_origin_time_secs: None,
        major_origin_time_secs: None,
    }
}

fn event(
    event_type: EventType,
    subtype: u8,
    direction: i8,
    occurred: usize,
    available: usize,
) -> EventRecord {
    EventRecord {
        event_type,
        event_subtype_code: subtype,
        source_code: 1,
        timeframe_secs: 300,
        occurred_bar_index: occurred,
        available_bar_index: available,
        direction_code: direction,
        reference_code: None,
        primary_value: Some(1.0),
        reference_value: Some(0.0),
    }
}

#[test]
fn direction_aggregates_use_only_valid_scores() {
    let mut first = snapshot(60);
    first.direction = direction(Some(-0.5), Readiness::Ready);
    let mut second = snapshot(300);
    second.direction = direction(Some(0.0), Readiness::Ready);
    let mut third = snapshot(900);
    third.direction = direction(Some(0.5), Readiness::Ready);
    let mut fourth = snapshot(3600);
    fourth.direction = direction(Some(0.3), Readiness::Provisional);

    let aggregates = CrossTimeframeAggregates::compute(&[fourth, second, first, third]);
    let direction = aggregates.direction;

    assert_eq!(aggregates.ladder, [60, 300, 900, 3600]);
    assert_eq!(direction.timeframes, [60, 300, 900, 3600]);
    assert_eq!(direction.direction_timeframe_count, 4);
    assert!((direction.direction_mean.expect("mean") - 0.075).abs() <= 1e-12);
    assert_eq!(direction.direction_bullish_share, Some(0.5));
    assert_eq!(direction.direction_bearish_share, Some(0.25));
    assert_eq!(direction.direction_neutral_share, Some(0.25));
    assert!((direction.direction_gradient.expect("gradient") - 0.8).abs() <= 1e-12);
    assert_eq!(direction.direction_ready_count, 3);
}

#[test]
fn a_missing_direction_is_not_counted_as_neutral() {
    let mut first = snapshot(60);
    first.direction = direction(None, Readiness::Unavailable);
    let mut second = snapshot(300);
    second.direction = direction(Some(0.0), Readiness::Ready);

    let aggregates = CrossTimeframeAggregates::compute(&[first, second]);
    assert_eq!(aggregates.direction.timeframes, [300]);
    assert_eq!(aggregates.direction.direction_neutral_share, Some(1.0));
    assert_eq!(aggregates.direction.direction_mean, Some(0.0));
}

#[test]
fn momentum_aggregates_report_means_shares_and_gaps() {
    let mut fast = snapshot(60);
    fast.momentum = momentum(Some(-0.5), Some(-0.2), Readiness::Ready);
    let mut second = snapshot(300);
    second.momentum = momentum(Some(0.0), Some(0.0), Readiness::Ready);
    let mut third = snapshot(900);
    third.momentum = momentum(Some(0.5), Some(0.2), Readiness::Ready);
    let mut slow = snapshot(3600);
    slow.momentum = momentum(Some(0.2), None, Readiness::Provisional);

    let aggregates = CrossTimeframeAggregates::compute(&[fast, second, third, slow]);
    let momentum = aggregates.momentum;

    assert_eq!(momentum.timeframes, [60, 300, 900, 3600]);
    assert!((momentum.momentum_force_mean.expect("force mean") - 0.05).abs() <= 1e-12);
    assert_eq!(momentum.momentum_positive_share, Some(0.5));
    assert_eq!(momentum.momentum_negative_share, Some(0.25));
    assert!(
        (momentum
            .momentum_acceleration_mean
            .expect("acceleration mean")
            - 0.0)
            .abs()
            <= 1e-12
    );
    assert!((momentum.momentum_force_fast_slow_gap.expect("force gap") - (-0.7)).abs() <= 1e-12);
    assert_eq!(momentum.momentum_acceleration_fast_slow_gap, None);
    assert!((momentum.momentum_gradient.expect("gradient") - 0.7).abs() <= 1e-12);
    assert_eq!(momentum.momentum_ready_count, 3);
}

#[test]
fn structure_aggregates_count_codes_and_keep_the_span() {
    let mut first = snapshot(60);
    first.structure = structure(Some(-0.5), Some(-2), Some(-1), Some(1), Readiness::Ready);
    let mut second = snapshot(300);
    second.structure = structure(Some(0.0), Some(0), Some(0), Some(0), Readiness::Ready);
    let mut third = snapshot(900);
    third.structure = structure(Some(0.5), Some(2), Some(1), Some(1), Readiness::Ready);
    let mut fourth = snapshot(3600);
    fourth.structure = structure(Some(1.0), Some(2), Some(1), Some(0), Readiness::Provisional);

    let aggregates = CrossTimeframeAggregates::compute(&[first, second, third, fourth]);
    let structure = aggregates.structure;

    assert_eq!(structure.structure_code_histogram, [1, 0, 1, 0, 2]);
    assert_eq!(structure.structure_state_span, Some(4));
    assert_eq!(structure.structure_bullish_share, Some(0.5));
    assert_eq!(structure.structure_bearish_share, Some(0.25));
    assert_eq!(structure.structure_neutral_share, Some(0.25));
    assert!((structure.structure_gradient.expect("gradient") - 1.5).abs() <= 1e-12);
    assert_eq!(structure.structure_ready_share, Some(0.75));
    assert_eq!(structure.structure_conflict_timeframe_count, 1);
}

#[test]
fn progression_stage_statistics_are_per_scale() {
    let mut first = snapshot(60);
    first.progression = progression(
        Some(leg(1, 1, 1, 100.0, 1, Some(0))),
        Some(leg(2, 2, 1, 100.0, 1, Some(5))),
        Readiness::Ready,
    );
    let mut second = snapshot(300);
    second.progression = progression(
        Some(leg(3, 1, 1, 101.0, 1, Some(2))),
        Some(leg(4, 2, 1, 101.0, 1, Some(5))),
        Readiness::Ready,
    );
    let mut third = snapshot(900);
    third.progression = progression(
        Some(leg(5, 1, -1, 102.0, 1, Some(2))),
        Some(leg(6, 2, 1, 102.0, 1, Some(4))),
        Readiness::Provisional,
    );
    let mut fourth = snapshot(3600);
    fourth.progression = progression(
        None,
        Some(leg(7, 2, 1, 103.0, 1, Some(3))),
        Readiness::Ready,
    );

    let aggregates = CrossTimeframeAggregates::compute(&[first, second, third, fourth]);
    let progression = aggregates.progression;

    assert_eq!(progression.timeframes, [60, 300, 900, 3600]);
    assert_eq!(progression.minor.stage_histogram, [1, 0, 2, 0, 0, 0]);
    assert_eq!(progression.minor.stage_mode, Some(2));
    assert_eq!(progression.minor.stage_min, Some(0));
    assert_eq!(progression.minor.stage_max, Some(2));
    assert_eq!(progression.minor.stage_span, Some(2));
    assert_eq!(progression.minor.modal_stage_share, Some(2.0 / 3.0));
    assert_eq!(progression.major.stage_histogram, [0, 0, 0, 1, 1, 2]);
    assert_eq!(progression.major.stage_mode, Some(5));
    assert_eq!(progression.major.stage_span, Some(2));
    assert_eq!(progression.major.modal_stage_share, Some(0.5));
    assert_eq!(progression.active_minor_leg_count, 3);
    assert_eq!(progression.active_major_leg_count, 4);
    assert_eq!(progression.progression_ready_share, Some(0.75));
    assert_eq!(progression.leg_direction_agreement_share, Some(6.0 / 7.0));
}

#[test]
fn cross_timeframe_leg_matches_require_scale_direction_time_and_price() {
    let mut fast = snapshot(300);
    fast.progression = progression(
        Some(leg(1, 1, 1, 100.0, 1, Some(2))),
        None,
        Readiness::Ready,
    );
    fast.minor_origin_time_secs = Some(1_000);
    fast.atr = Some(2.0);
    let mut slow = snapshot(900);
    slow.progression = progression(
        Some(leg(2, 1, 1, 100.5, 1, Some(2))),
        None,
        Readiness::Ready,
    );
    slow.minor_origin_time_secs = Some(1_600);
    slow.atr = Some(2.0);
    let mut other = snapshot(3600);
    other.progression = progression(
        None,
        Some(leg(3, 2, 1, 100.0, 1, Some(2))),
        Readiness::Ready,
    );
    other.major_origin_time_secs = Some(1_000);
    other.atr = Some(2.0);

    let aggregates = CrossTimeframeAggregates::compute(&[fast, slow, other]);
    assert_eq!(aggregates.progression.cross_timeframe_leg_match_count, 1);
}

#[test]
fn event_counts_use_the_five_bar_window() {
    let mut first = snapshot(300);
    first.events = vec![
        event(EventType::Level, 1, 0, 90, 90),
        event(EventType::Level, 3, 1, 96, 96),
        event(EventType::Divergence, 1, 1, 99, 99),
        event(EventType::Signal, 1, 1, 100, 100),
        event(EventType::Divergence, 2, -1, 101, 101),
    ];
    let mut second = snapshot(900);
    second.events = vec![event(EventType::Divergence, 2, -1, 98, 98)];

    let aggregates = CrossTimeframeAggregates::compute(&[first, second]);
    let events = aggregates.events;

    assert_eq!(events.event_count_by_type, [1, 2, 1]);
    assert_eq!(events.event_count_by_subtype.level, [0, 0, 1]);
    assert_eq!(events.event_count_by_subtype.divergence, [1, 1, 0, 0]);
    assert_eq!(events.event_count_by_subtype.signal, [1, 0, 0]);
    assert_eq!(events.divergence_timeframe_count, 2);
    assert_eq!(events.divergence_direction_agreement_share, Some(0.5));
    assert_eq!(events.level_event_timeframe_count, 1);
    assert_eq!(events.signal_event_timeframe_count, 1);

    let summary = &events.per_timeframe[0];
    assert_eq!(summary.counts_by_type, [1, 1, 1]);
    assert_eq!(summary.latest_event_type_code, Some(3));
    assert_eq!(summary.latest_available_bar_index, Some(100));
    assert_eq!(summary.latest_event_age_bars, Some(0));
}

#[test]
fn the_event_window_is_exactly_five_bars() {
    assert_eq!(EVENT_WINDOW_BARS, 5);
    let mut first = snapshot(300);
    first.events = vec![
        event(EventType::Level, 1, 0, 95, 95),
        event(EventType::Level, 1, 0, 96, 96),
    ];
    let aggregates = CrossTimeframeAggregates::compute(&[first]);
    assert_eq!(aggregates.events.event_count_by_type[0], 1);
}

#[test]
fn level_clusters_link_within_half_the_larger_atr() {
    let mut first = snapshot(60);
    first.atr = Some(1.0);
    first.references = ReferenceSet::new(vec![100.0]);
    let mut second = snapshot(300);
    second.atr = Some(2.0);
    second.references = ReferenceSet::new(vec![100.5]);
    let mut third = snapshot(900);
    third.atr = Some(10.0);
    third.references = ReferenceSet::new(vec![200.0]);

    let aggregates = CrossTimeframeAggregates::compute(&[first, second, third]);
    let structure = aggregates.structure;

    assert_eq!(structure.level_cluster_count, 2);
    assert_eq!(structure.largest_level_cluster_timeframe_count, 2);
    assert!((structure.largest_level_cluster_span_atr.expect("span") - 0.25).abs() <= 1e-12);
}

#[test]
fn single_linkage_can_chain_levels() {
    let mut first = snapshot(60);
    first.atr = Some(1.0);
    first.references = ReferenceSet::new(vec![100.0]);
    let mut second = snapshot(300);
    second.atr = Some(1.0);
    second.references = ReferenceSet::new(vec![100.5]);
    let mut third = snapshot(900);
    third.atr = Some(1.0);
    third.references = ReferenceSet::new(vec![101.0]);

    let aggregates = CrossTimeframeAggregates::compute(&[first, second, third]);
    assert_eq!(aggregates.structure.level_cluster_count, 1);
    assert_eq!(
        aggregates.structure.largest_level_cluster_timeframe_count,
        3
    );
    assert!(
        (aggregates
            .structure
            .largest_level_cluster_span_atr
            .expect("span")
            - 1.0)
            .abs()
            <= 1e-12
    );
}

#[test]
fn the_reference_set_keeps_at_most_seven_prices() {
    let set = ReferenceSet::new((0..10).map(f64::from).collect());
    assert_eq!(set.len(), MAX_REFERENCES);
    let parts = ReferenceSet::from_parts(
        Some(1.0),
        Some(2.0),
        Some(3.0),
        Some(4.0),
        Some(5.0),
        Some(6.0),
        Some(7.0),
    );
    assert_eq!(parts.len(), MAX_REFERENCES);
    assert_eq!(set.prices(), [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
}

#[test]
fn an_empty_ladder_produces_empty_aggregates() {
    let aggregates = CrossTimeframeAggregates::compute(&[]);
    assert!(aggregates.ladder.is_empty());
    assert_eq!(aggregates.direction.direction_timeframe_count, 0);
    assert_eq!(aggregates.direction.direction_mean, None);
    assert_eq!(aggregates.momentum.momentum_force_mean, None);
    assert_eq!(aggregates.structure.structure_code_histogram, [0; 5]);
    assert_eq!(aggregates.progression.minor.stage_mode, None);
    assert_eq!(aggregates.events.event_count_by_type, [0; 3]);
}

#[test]
fn the_order_of_the_snapshots_does_not_change_the_aggregates() {
    let mut first = snapshot(60);
    first.direction = direction(Some(-0.5), Readiness::Ready);
    first.momentum = momentum(Some(-0.5), Some(-0.2), Readiness::Ready);
    first.structure = structure(Some(-1.0), Some(-2), Some(-1), Some(1), Readiness::Ready);
    first.references = ReferenceSet::new(vec![100.0]);
    first.atr = Some(1.0);
    first.events = vec![event(EventType::Level, 1, 0, 99, 99)];
    let mut second = snapshot(300);
    second.direction = direction(Some(0.0), Readiness::Ready);
    second.momentum = momentum(Some(0.0), Some(0.0), Readiness::Ready);
    second.structure = structure(Some(0.0), Some(0), Some(0), Some(0), Readiness::Ready);
    second.references = ReferenceSet::new(vec![100.5]);
    second.atr = Some(2.0);
    let mut third = snapshot(900);
    third.direction = direction(Some(0.5), Readiness::Ready);
    third.momentum = momentum(Some(0.5), Some(0.2), Readiness::Ready);
    third.structure = structure(Some(1.0), Some(2), Some(1), Some(0), Readiness::Ready);
    third.references = ReferenceSet::new(vec![200.0]);
    third.atr = Some(10.0);

    let forward =
        CrossTimeframeAggregates::compute(&[first.clone(), second.clone(), third.clone()]);
    let reversed = CrossTimeframeAggregates::compute(&[third, second, first]);
    assert_eq!(forward, reversed);
}

#[test]
fn the_aggregates_are_deterministic() {
    let mut first = snapshot(60);
    first.direction = direction(Some(-0.5), Readiness::Ready);
    let mut second = snapshot(300);
    second.direction = direction(Some(0.5), Readiness::Ready);
    let inputs = [first, second];

    let first_run = CrossTimeframeAggregates::compute(&inputs);
    let second_run = CrossTimeframeAggregates::compute(&inputs);
    assert_eq!(first_run, second_run);
}

#[test]
fn event_subtype_counts_are_separate_by_family() {
    let counts = EventSubtypeCounts {
        level: [1, 2, 3],
        divergence: [4, 5, 6, 7],
        signal: [8, 9, 10],
    };
    assert_eq!(counts.level, [1, 2, 3]);
    assert_eq!(counts.divergence, [4, 5, 6, 7]);
    assert_eq!(counts.signal, [8, 9, 10]);
    let _ = ProgressionAggregates {
        timeframes: Vec::new(),
        progression_timeframe_count: 0,
        minor: super::progression::ScaleStageAggregates {
            stage_histogram: [0; 6],
            stage_mode: None,
            stage_min: None,
            stage_max: None,
            stage_span: None,
            modal_stage_share: None,
        },
        major: super::progression::ScaleStageAggregates {
            stage_histogram: [0; 6],
            stage_mode: None,
            stage_min: None,
            stage_max: None,
            stage_span: None,
            modal_stage_share: None,
        },
        active_major_leg_count: 0,
        active_minor_leg_count: 0,
        progression_ready_share: None,
        leg_direction_agreement_share: None,
        cross_timeframe_leg_match_count: 0,
    };
}
