use super::{
    ProgressionPace, pace_size, progression_detail, progression_pace, progression_position,
};
use crate::axes::progression::{ProgressionBar, ProgressionLeg};
use crate::dashboard::{
    AxisValue, PaceDirection, ProgressionDetail, RECENT_PACE_WINDOW_BARS, Stage,
};
use crate::kernel::{PERCENTILE_MIN_SAMPLES, PERCENTILE_WINDOW, Readiness, UnavailableReason};

const GOLDEN_FIXTURE: &str =
    include_str!("../../../../tests/fixtures/dashboard_progression_pace_golden.csv");
const TOLERANCE: f64 = 1e-9;

fn leg(id: u64, origin: usize, direction: i8, displacement: Option<f64>) -> ProgressionLeg {
    ProgressionLeg {
        leg_id: id,
        timeframe_secs: Some(60),
        scale_code: 1,
        origin_price: 100.0,
        origin_occurred_at: origin,
        origin_available_at: Some(origin),
        origin_confirmed_code: 1,
        direction_code: direction,
        target_price: Some(110.0),
        target_basis_code: Some(1),
        target_version: 0,
        target_stable_code: 1,
        progress: Some(0.5),
        displacement_price: displacement.map(|value| value * 2.0),
        displacement_atr: displacement,
        lifecycle_code: 1,
        stage_code: Some(1),
        target_reached_code: Some(0),
    }
}

fn forming(leg: &mut ProgressionLeg) {
    leg.origin_available_at = None;
    leg.origin_confirmed_code = 0;
    leg.lifecycle_code = 0;
    leg.stage_code = None;
}

fn terminal(leg: &mut ProgressionLeg, lifecycle_code: u8) {
    leg.lifecycle_code = lifecycle_code;
    leg.stage_code = Some(5);
}

fn bar(leg: Option<ProgressionLeg>) -> ProgressionBar {
    ProgressionBar {
        progress: leg.and_then(|leg| leg.progress),
        stage_code: leg.and_then(|leg| leg.stage_code),
        leg_direction: leg.map(|leg| leg.direction_code),
        leg_scale_code: leg.map(|leg| leg.scale_code),
        published_leg: leg,
        minor_leg: leg,
        major_leg: None,
        size_percentile: leg
            .and_then(|leg| leg.displacement_atr)
            .map(|value| value.abs() * 10.0),
        readiness: if leg.is_some() {
            Readiness::Provisional
        } else {
            Readiness::Unavailable
        },
        unavailable_reason: None,
        state_age_bars: 0,
        state_changes_50: 0,
    }
}

fn series(legs: &[Option<ProgressionLeg>]) -> Vec<ProgressionBar> {
    legs.iter().copied().map(bar).collect()
}

fn linear_series(bars: usize) -> Vec<ProgressionBar> {
    (0..bars)
        .map(|index| bar(Some(leg(1, 0, 1, Some(index as f64)))))
        .collect()
}

fn assert_close(actual: Option<f64>, expected: Option<f64>, context: &str) {
    match (actual, expected) {
        (None, None) => {}
        (Some(actual), Some(expected)) => assert!(
            (actual - expected).abs() < TOLERANCE,
            "{context}: expected {expected}, got {actual}"
        ),
        _ => panic!("{context}: expected {expected:?}, got {actual:?}"),
    }
}

fn pace_direction_code(direction: Option<PaceDirection>) -> Option<u8> {
    direction.map(|direction| match direction {
        PaceDirection::WithLeg => 1,
        PaceDirection::AgainstLeg => 2,
        PaceDirection::Flat => 3,
    })
}

#[test]
fn the_six_anchors_map_one_to_one() {
    for code in 0..=5 {
        let mut bar = bar(Some(leg(1, 0, 1, Some(0.0))));
        bar.stage_code = Some(code);
        assert_eq!(
            progression_position(&bar),
            AxisValue::ordinal(Stage::from_code(code).expect("a valid stage code")),
        );
    }

    let absent = bar(None);
    assert_eq!(
        progression_position(&absent),
        AxisValue::undefined(UnavailableReason::OriginUnconfirmed),
    );
}

#[test]
fn pace_is_signed_in_the_leg_direction() {
    let up_with = series(&[
        Some(leg(1, 0, 1, Some(0.0))),
        Some(leg(1, 0, 1, Some(1.0))),
        Some(leg(1, 0, 1, Some(0.0))),
        Some(leg(1, 0, 1, Some(3.0))),
    ]);
    let pace = progression_pace(&up_with, 3);
    assert_eq!(pace.signed_atr_per_bar, Some(1.0));
    assert_eq!(pace.direction, Some(PaceDirection::WithLeg));

    let down_with = series(&[
        Some(leg(1, 0, -1, Some(0.0))),
        Some(leg(1, 0, -1, Some(-1.0))),
        Some(leg(1, 0, -1, Some(0.0))),
        Some(leg(1, 0, -1, Some(-3.0))),
    ]);
    let pace = progression_pace(&down_with, 3);
    assert_eq!(pace.signed_atr_per_bar, Some(1.0));
    assert_eq!(pace.direction, Some(PaceDirection::WithLeg));

    let up_against = series(&[
        Some(leg(1, 0, 1, Some(0.0))),
        Some(leg(1, 0, 1, Some(-1.0))),
        Some(leg(1, 0, 1, Some(0.0))),
        Some(leg(1, 0, 1, Some(-3.0))),
    ]);
    let pace = progression_pace(&up_against, 3);
    assert_eq!(pace.signed_atr_per_bar, Some(-1.0));
    assert_eq!(pace.direction, Some(PaceDirection::AgainstLeg));
}

#[test]
fn a_flat_pace_is_a_measured_zero() {
    let bars = series(&[
        Some(leg(1, 0, 1, Some(1.0))),
        Some(leg(1, 0, 1, Some(1.0))),
        Some(leg(1, 0, 1, Some(1.0))),
        Some(leg(1, 0, 1, Some(1.0))),
    ]);
    let pace = progression_pace(&bars, 3);
    assert_eq!(pace.signed_atr_per_bar, Some(0.0));
    assert_eq!(pace.direction, Some(PaceDirection::Flat));

    let detail = progression_detail(bars[3], bars[3].published_leg(), pace);
    assert_eq!(detail.recent_pace_signed_atr_per_bar, Some(0.0));
    assert_eq!(detail.recent_pace_direction, Some(PaceDirection::Flat));
}

#[test]
fn the_same_confirmed_leg_is_required_at_both_endpoints() {
    let valid = series(&[
        Some(leg(1, 0, 1, Some(0.0))),
        Some(leg(1, 0, 1, Some(1.0))),
        Some(leg(1, 0, 1, Some(2.0))),
        Some(leg(1, 0, 1, Some(3.0))),
    ]);
    assert_eq!(progression_pace(&valid, 3).signed_atr_per_bar, Some(1.0));

    let mut current_forming = valid.clone();
    let mut current_leg = leg(1, 0, 1, Some(3.0));
    forming(&mut current_leg);
    current_forming[3] = bar(Some(current_leg));
    assert_eq!(
        progression_pace(&current_forming, 3).signed_atr_per_bar,
        None,
    );

    let mut prior_forming = valid.clone();
    let mut prior_leg = leg(1, 0, 1, Some(0.0));
    forming(&mut prior_leg);
    prior_forming[0] = bar(Some(prior_leg));
    assert_eq!(progression_pace(&prior_forming, 3).signed_atr_per_bar, None,);

    let mut prior_lookahead = valid.clone();
    let mut prior_leg = leg(1, 0, 1, Some(0.0));
    prior_leg.origin_available_at = Some(3);
    prior_lookahead[0] = bar(Some(prior_leg));
    assert_eq!(
        progression_pace(&prior_lookahead, 3).signed_atr_per_bar,
        None,
        "a prior endpoint confirmed only later is not knowable at its bar"
    );

    let mut prior_missing = valid.clone();
    prior_missing[0] = bar(Some(leg(1, 0, 1, None)));
    assert_eq!(progression_pace(&prior_missing, 3).signed_atr_per_bar, None,);

    let mut current_missing = valid.clone();
    current_missing[3] = bar(Some(leg(1, 0, 1, None)));
    assert_eq!(
        progression_pace(&current_missing, 3).signed_atr_per_bar,
        None,
    );

    let mut different_prior = valid.clone();
    different_prior[0] = bar(Some(leg(9, 0, 1, Some(0.0))));
    assert_eq!(
        progression_pace(&different_prior, 3).signed_atr_per_bar,
        None,
    );

    let mut different_current = valid.clone();
    different_current[3] = bar(Some(leg(9, 3, 1, Some(3.0))));
    assert_eq!(
        progression_pace(&different_current, 3).signed_atr_per_bar,
        None,
    );
}

#[test]
fn the_percentile_requires_fifty_prior_samples_and_never_samples_the_view() {
    let bars = linear_series(54);

    let before = progression_pace(&bars, 52);
    assert_eq!(before.signed_atr_per_bar, Some(1.0));
    assert_eq!(
        before.speed_percentile, None,
        "49 prior samples are below the minimum"
    );

    let at_minimum = progression_pace(&bars, 53);
    assert_eq!(
        at_minimum.speed_percentile,
        Some(100.0),
        "50 prior equal samples rank at the top through inclusive ties"
    );
    assert_eq!(53 - 3, PERCENTILE_MIN_SAMPLES);
}

#[test]
fn the_percentile_is_capped_at_252_samples() {
    let mut displacements = vec![0.0; 304];
    for index in 3..303 {
        let pace = if index <= 50 { 0.1 } else { 10.0 };
        displacements[index] = displacements[index - 3] + 3.0 * pace;
    }
    // The current bar is a new, much smaller measurement. It is not a sample,
    // and the 252-sample cap must discard the older 0.1 values.
    displacements[303] = displacements[300] + 3.0;
    let bars: Vec<ProgressionBar> = displacements
        .into_iter()
        .map(|displacement| bar(Some(leg(1, 0, 1, Some(displacement)))))
        .collect();

    let pace = progression_pace(&bars, 303);
    assert_eq!(pace.signed_atr_per_bar, Some(1.0));
    assert_eq!(pace.speed_percentile, Some(0.0));
    assert_eq!(303 - 3 - PERCENTILE_WINDOW, 48);
}

#[test]
fn a_carried_terminal_pace_is_not_re_sampled() {
    let mut displacements = vec![0.0; 61];
    for index in 3..=52 {
        let pace = if index == 52 { 10.0 } else { 0.5 };
        displacements[index] = displacements[index - 3] + 3.0 * pace;
    }
    displacements[53] = displacements[50] + 3.0;
    for index in 54..=60 {
        displacements[index] = displacements[53];
    }

    let bars: Vec<ProgressionBar> = displacements
        .iter()
        .copied()
        .enumerate()
        .map(|(index, displacement)| {
            let mut leg = leg(1, 0, 1, Some(displacement));
            if index >= 53 {
                terminal(&mut leg, 2);
            }
            bar(Some(leg))
        })
        .collect();

    let freeze_bar = progression_pace(&bars, 53);
    assert_eq!(freeze_bar.signed_atr_per_bar, Some(1.0));
    assert_eq!(
        freeze_bar.speed_percentile,
        Some(100.0 * 49.0 / 50.0),
        "the freeze bar ranks against the 50 prior measured paces"
    );
    assert!(freeze_bar.frozen_terminal);

    let later = progression_pace(&bars, 60);
    assert_eq!(later.signed_atr_per_bar, Some(1.0));
    assert_eq!(
        later.speed_percentile,
        Some(100.0 * 50.0 / 51.0),
        "the carried value contributes one sample, not one per later bar"
    );
    assert_eq!(
        later.average_net_displacement_atr_per_bar,
        Some(displacements[53] / 53.0),
        "the average is frozen at the terminal bar"
    );
    assert!(later.frozen_terminal);

    let leg = bars[60].published_leg().expect("a published leg");
    assert_eq!(
        pace_size(&later, Some(leg)),
        AxisValue::rank(100.0 * 50.0 / 51.0)
    );
}

#[test]
fn terminal_freeze_covers_completed_invalidated_and_superseded() {
    for lifecycle_code in [2, 3, 4] {
        let bars: Vec<ProgressionBar> = (0..=6)
            .map(|index| {
                let displacement = if index <= 3 { index as f64 } else { 3.0 };
                let mut leg = leg(1, 0, 1, Some(displacement));
                if index >= 3 {
                    terminal(&mut leg, lifecycle_code);
                }
                bar(Some(leg))
            })
            .collect();

        let at_terminal = progression_pace(&bars, 3);
        assert_eq!(at_terminal.signed_atr_per_bar, Some(1.0));
        assert_eq!(at_terminal.average_net_displacement_atr_per_bar, Some(1.0));
        assert!(at_terminal.frozen_terminal);

        let later = progression_pace(&bars, 6);
        assert_eq!(
            later.signed_atr_per_bar,
            Some(1.0),
            "lifecycle {lifecycle_code} must not decay"
        );
        assert_eq!(
            later.average_net_displacement_atr_per_bar,
            Some(1.0),
            "lifecycle {lifecycle_code} must not decay"
        );
        assert!(later.frozen_terminal);

        let published = bars[6].published_leg().expect("a published leg");
        let detail = progression_detail(bars[6], Some(published), later);
        assert_eq!(detail.lifecycle_code, Some(lifecycle_code));
        assert_eq!(detail.stage_code, Some(5));
        assert!(detail.recent_pace_frozen_terminal);
        assert_eq!(
            progression_position(&bars[6]),
            AxisValue::ordinal(Stage::Terminal)
        );

        let mut after_change = bars.clone();
        after_change.push(bar(Some(leg(2, 7, 1, Some(10.0)))));
        let changed = progression_pace(&after_change, 7);
        assert!(!changed.frozen_terminal);
        assert_eq!(changed.signed_atr_per_bar, None);
        assert_eq!(changed.average_net_displacement_atr_per_bar, Some(10.0));
    }
}

#[test]
fn a_terminal_freeze_keeps_the_average_when_the_pace_is_absent() {
    let bars: Vec<ProgressionBar> = (0..=5)
        .map(|index| {
            let id = if index < 3 { 1 } else { 2 };
            let origin = if index < 3 { 0 } else { 3 };
            let displacement = if index < 3 { 1.0 } else { 3.0 };
            let mut leg = leg(id, origin, 1, Some(displacement));
            if index >= 3 {
                terminal(&mut leg, 3);
            }
            bar(Some(leg))
        })
        .collect();

    let at_terminal = progression_pace(&bars, 3);
    assert_eq!(at_terminal.signed_atr_per_bar, None);
    assert_eq!(at_terminal.average_net_displacement_atr_per_bar, Some(3.0));
    assert!(at_terminal.frozen_terminal);

    let later = progression_pace(&bars, 5);
    assert_eq!(later.signed_atr_per_bar, None);
    assert_eq!(
        later.average_net_displacement_atr_per_bar,
        Some(3.0),
        "the average remains anchored to the terminal bar"
    );
    assert!(later.frozen_terminal);

    let leg = bars[5].published_leg().expect("a published leg");
    assert_eq!(
        pace_size(&later, Some(leg)),
        AxisValue::undefined(UnavailableReason::InsufficientBars)
    );
}

#[test]
fn a_terminal_freeze_can_hold_an_absent_pace_and_average() {
    let bars: Vec<ProgressionBar> = (0..=4)
        .map(|index| {
            let displacement = if index == 3 { None } else { Some(4.0) };
            let mut leg = leg(1, 0, 1, displacement);
            if index >= 3 {
                terminal(&mut leg, 4);
            }
            bar(Some(leg))
        })
        .collect();

    let later = progression_pace(&bars, 4);
    assert_eq!(later.signed_atr_per_bar, None);
    assert_eq!(later.average_net_displacement_atr_per_bar, None);
    assert!(later.frozen_terminal);
}

#[test]
fn average_net_displacement_uses_elapsed_bars_and_preserves_zero() {
    let bars = series(&[
        Some(leg(1, 0, 1, Some(2.0))),
        Some(leg(1, 0, 1, Some(4.0))),
        Some(leg(1, 0, 1, Some(6.0))),
        Some(leg(1, 0, 1, Some(6.0))),
        Some(leg(1, 0, 1, Some(8.0))),
        Some(leg(1, 0, 1, Some(0.0))),
        Some(leg(1, 0, 1, None)),
    ]);

    assert_eq!(
        progression_pace(&bars, 0).average_net_displacement_atr_per_bar,
        Some(2.0),
        "the denominator is at least one"
    );
    assert_eq!(
        progression_pace(&bars, 4).average_net_displacement_atr_per_bar,
        Some(2.0)
    );
    assert_eq!(
        progression_pace(&bars, 5).average_net_displacement_atr_per_bar,
        Some(0.0),
        "a zero displacement is a measured zero"
    );
    assert_eq!(
        progression_pace(&bars, 6).average_net_displacement_atr_per_bar,
        None
    );
}

#[test]
fn the_replay_is_causal() {
    let bars = linear_series(10);
    let full = progression_pace(&bars, 5);
    let prefix = progression_pace(&bars[..=5], 5);
    assert_eq!(full, prefix, "a later bar must not change the view");

    let mut altered = bars.clone();
    altered[6] = bar(Some(leg(1, 0, 1, Some(1_000.0))));
    altered[7] = bar(Some(leg(2, 7, -1, Some(-1_000.0))));
    assert_eq!(
        progression_pace(&altered, 5),
        full,
        "altering later bars must not change the view"
    );
}

#[test]
fn the_replay_is_deterministic() {
    let bars = linear_series(60);
    assert_eq!(progression_pace(&bars, 59), progression_pace(&bars, 59));
    assert_eq!(
        progression_pace(&bars, 59),
        progression_pace(&bars.clone(), 59)
    );
}

#[test]
fn empty_short_and_out_of_range_series_are_typed_absence() {
    assert_eq!(progression_pace(&[], 0), ProgressionPace::undefined());

    let no_leg = vec![bar(None)];
    assert_eq!(progression_pace(&no_leg, 0), ProgressionPace::undefined());

    let single = vec![bar(Some(leg(1, 0, 1, Some(2.0))))];
    let pace = progression_pace(&single, 0);
    assert_eq!(pace.signed_atr_per_bar, None);
    assert_eq!(pace.average_net_displacement_atr_per_bar, Some(2.0));
    assert!(!pace.frozen_terminal);

    let short = linear_series(3);
    assert_eq!(progression_pace(&short, 2).signed_atr_per_bar, None);
    assert_eq!(progression_pace(&short, 3), ProgressionPace::undefined());
}

#[test]
fn pace_size_uses_the_origin_reason_only_for_a_missing_or_forming_leg() {
    let mut forming_leg = leg(1, 0, 1, Some(0.0));
    forming(&mut forming_leg);
    let confirmed_leg = leg(1, 0, 1, Some(0.0));
    let pace = ProgressionPace {
        speed_percentile: None,
        ..ProgressionPace::undefined()
    };

    assert_eq!(
        pace_size(&pace, None),
        AxisValue::undefined(UnavailableReason::OriginUnconfirmed)
    );
    assert_eq!(
        pace_size(&pace, Some(forming_leg)),
        AxisValue::undefined(UnavailableReason::OriginUnconfirmed)
    );
    assert_eq!(
        pace_size(&pace, Some(confirmed_leg)),
        AxisValue::undefined(UnavailableReason::InsufficientBars)
    );
    assert_eq!(
        pace_size(
            &ProgressionPace {
                speed_percentile: Some(42.0),
                ..pace
            },
            Some(confirmed_leg)
        ),
        AxisValue::rank(42.0)
    );
    assert_eq!(
        pace_size(
            &ProgressionPace {
                speed_percentile: Some(0.0),
                ..pace
            },
            Some(confirmed_leg)
        ),
        AxisValue::rank(0.0),
        "a zero percentile is a measured rank, not absence"
    );
}

#[derive(Debug)]
struct GoldenRow {
    index: usize,
    bar: ProgressionBar,
    expected_stage: Option<i8>,
    expected_stage_reason: Option<u8>,
    expected_size: Option<f64>,
    expected_size_reason: Option<u8>,
    expected_signed: Option<f64>,
    expected_direction: Option<u8>,
    expected_average: Option<f64>,
    expected_frozen: bool,
}

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(
            field
                .parse::<f64>()
                .unwrap_or_else(|error| panic!("invalid number {field:?}: {error}")),
        )
    }
}

fn integer<T>(field: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    field
        .parse::<T>()
        .unwrap_or_else(|error| panic!("invalid integer {field:?}: {error}"))
}

fn optional_integer<T>(field: &str) -> Option<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    if field.is_empty() {
        None
    } else {
        Some(integer(field))
    }
}

fn golden_rows() -> Vec<GoldenRow> {
    const HEADER: &str = "bar,published_leg_id,origin_bar,origin_available_bar,origin_confirmed_code,direction_code,displacement_atr,lifecycle_code,stage_code,size_percentile,progress,expected_position_code,expected_position_reason_code,expected_size_pct,expected_size_reason_code,expected_signed_pace,expected_pace_direction_code,expected_average,expected_frozen";

    let mut lines = GOLDEN_FIXTURE.lines();
    assert_eq!(lines.next(), Some(HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 19, "bad golden row: {line}");
            let leg = if fields[1].is_empty() {
                None
            } else {
                Some(ProgressionLeg {
                    leg_id: integer(fields[1]),
                    timeframe_secs: Some(60),
                    scale_code: 1,
                    origin_price: 100.0,
                    origin_occurred_at: integer(fields[2]),
                    origin_available_at: optional_integer(fields[3]),
                    origin_confirmed_code: integer(fields[4]),
                    direction_code: integer(fields[5]),
                    target_price: Some(110.0),
                    target_basis_code: Some(1),
                    target_version: 0,
                    target_stable_code: 1,
                    progress: number(fields[10]),
                    displacement_price: number(fields[6]).map(|value| value * 2.0),
                    displacement_atr: number(fields[6]),
                    lifecycle_code: integer(fields[7]),
                    stage_code: optional_integer(fields[8]),
                    target_reached_code: Some(0),
                })
            };
            let bar = ProgressionBar {
                progress: number(fields[10]),
                stage_code: optional_integer(fields[8]),
                leg_direction: leg.map(|leg| leg.direction_code),
                leg_scale_code: leg.map(|leg| leg.scale_code),
                published_leg: leg,
                minor_leg: leg,
                major_leg: None,
                size_percentile: number(fields[9]),
                readiness: if leg.is_some() {
                    Readiness::Provisional
                } else {
                    Readiness::Unavailable
                },
                unavailable_reason: None,
                state_age_bars: 0,
                state_changes_50: 0,
            };

            GoldenRow {
                index: integer(fields[0]),
                bar,
                expected_stage: optional_integer(fields[11]),
                expected_stage_reason: optional_integer(fields[12]),
                expected_size: number(fields[13]),
                expected_size_reason: optional_integer(fields[14]),
                expected_signed: number(fields[15]),
                expected_direction: optional_integer(fields[16]),
                expected_average: number(fields[17]),
                expected_frozen: fields[18] == "1",
            }
        })
        .collect()
}

#[test]
fn golden_fixture_matches_the_contract() {
    let rows = golden_rows();
    let bars: Vec<ProgressionBar> = rows.iter().map(|row| row.bar).collect();

    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row.index, index, "the fixture is in bar order");
        let pace = progression_pace(&bars, index);
        let position = progression_position(&row.bar);
        let leg = row.bar.published_leg();
        let size = pace_size(&pace, leg);
        let detail = progression_detail(row.bar, leg, pace);

        match row.expected_stage {
            Some(code) => assert_eq!(
                position,
                AxisValue::ordinal(Stage::from_code(code).expect("a valid stage code")),
                "bar {index}: position"
            ),
            None => assert_eq!(
                position,
                AxisValue::undefined(
                    UnavailableReason::from_code(
                        row.expected_stage_reason.expect("a stage reason")
                    )
                    .expect("a valid reason code")
                ),
                "bar {index}: position reason"
            ),
        }

        match row.expected_size {
            Some(percentile) => assert_eq!(size, AxisValue::rank(percentile), "bar {index}: size"),
            None => assert_eq!(
                size,
                AxisValue::undefined(
                    UnavailableReason::from_code(row.expected_size_reason.expect("a size reason"))
                        .expect("a valid reason code")
                ),
                "bar {index}: size reason"
            ),
        }

        assert_close(
            detail.recent_pace_signed_atr_per_bar,
            row.expected_signed,
            &format!("bar {index}: signed pace"),
        );
        assert_close(
            detail.average_net_displacement_atr_per_bar,
            row.expected_average,
            &format!("bar {index}: average"),
        );
        assert_eq!(detail.stage_code, row.expected_stage, "bar {index}: stage");
        assert_eq!(
            detail.lifecycle_code,
            row.bar.published_leg().map(|leg| leg.lifecycle_code),
            "bar {index}: lifecycle"
        );
        if row.expected_frozen {
            assert!(
                matches!(detail.lifecycle_code, Some(2..=4)),
                "bar {index}: a frozen pace belongs to a terminal leg"
            );
            assert_eq!(
                detail.stage_code,
                Some(5),
                "bar {index}: a terminal leg publishes the terminal stage"
            );
        }
        assert_eq!(
            pace_direction_code(detail.recent_pace_direction),
            row.expected_direction,
            "bar {index}: pace direction"
        );
        assert_eq!(
            detail.recent_pace_frozen_terminal, row.expected_frozen,
            "bar {index}: terminal freeze"
        );
        assert_eq!(
            detail.recent_pace_window_bars, RECENT_PACE_WINDOW_BARS,
            "bar {index}: window"
        );
        assert_eq!(
            detail.displacement_pct, row.bar.size_percentile,
            "bar {index}: the displacement percentile remains separate"
        );
    }
}

#[test]
fn the_detail_keeps_lifecycle_distinct_from_stage() {
    let mut leg = leg(1, 0, 1, Some(3.0));
    terminal(&mut leg, 2);
    let bar = bar(Some(leg));
    let pace = progression_pace(&[bar], 0);
    let detail: ProgressionDetail = progression_detail(bar, bar.published_leg(), pace);

    assert_eq!(detail.stage_code, Some(5));
    assert_eq!(detail.lifecycle_code, Some(2));
    assert_eq!(
        progression_position(&bar),
        AxisValue::ordinal(Stage::Terminal)
    );
}
