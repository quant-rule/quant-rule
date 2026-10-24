//! Unit tests for the Progression axis: creation, confirmation, publication,
//! readiness, counters, and the edge paths the golden fixture does not reach.
//!
//! The tests live beside the module so they can reach its private helpers; the
//! golden fixture in `tests/progression_golden.rs` covers the published values
//! over the checked-in 260-bar series.

use quantrule_data::Candle;

use super::leg::Leg;
use super::{ProgressionConfig, ProgressionSeries, readiness};
use crate::axes::structure::registry::ConfirmedLevelRegistry;
use crate::kernel::{Readiness, UnavailableReason};
use crate::readings::{PivotCandidate, PivotKind, PivotScale, PivotSeries, Readings};

/// Candles from a piecewise-linear mid-price path, with a half-point range.
fn zigzag_candles(points: &[(usize, f64)]) -> Vec<Candle> {
    let last_bar = points.last().expect("the path has a last point").0;
    let mut mids = vec![0.0; last_bar + 1];

    for pair in points.windows(2) {
        let (start_bar, start_mid) = pair[0];
        let (end_bar, end_mid) = pair[1];
        let span = (end_bar - start_bar) as f64;
        for (bar, mid) in mids
            .iter_mut()
            .enumerate()
            .take(end_bar + 1)
            .skip(start_bar)
        {
            let offset = (bar - start_bar) as f64;
            *mid = start_mid + (end_mid - start_mid) * offset / span;
        }
    }

    mids.iter()
        .enumerate()
        .map(|(bar, mid)| {
            Candle::new(
                bar as u64,
                bar as i64,
                *mid,
                mid + 0.5,
                mid - 0.5,
                *mid,
                Some(10.0),
            )
            .expect("the test candle is valid")
        })
        .collect()
}

fn series(path: &[(usize, f64)]) -> ProgressionSeries {
    let candles = zigzag_candles(path);
    let readings = Readings::compute(&candles);
    ProgressionSeries::compute(&candles, &readings)
}

/// Candles from explicit high, low, and close values, with open at the
/// close. The helper exists so a test can shape a wick without shaping the
/// whole path.
fn ohlc_candles(rows: &[(f64, f64, f64)]) -> Vec<Candle> {
    rows.iter()
        .enumerate()
        .map(|(bar, (high, low, close))| {
            Candle::new(
                bar as u64,
                bar as i64,
                *close,
                *high,
                *low,
                *close,
                Some(10.0),
            )
            .expect("the test candle is valid")
        })
        .collect()
}

fn series_from_ohlc(rows: &[(f64, f64, f64)]) -> ProgressionSeries {
    let candles = ohlc_candles(rows);
    let readings = Readings::compute(&candles);
    ProgressionSeries::compute(&candles, &readings)
}

fn reference_path() -> Vec<Candle> {
    zigzag_candles(&[
        (0, 100.0),
        (4, 110.0),
        (8, 90.0),
        (12, 110.0),
        (16, 90.0),
        (20, 110.0),
        (24, 90.0),
        (28, 110.0),
    ])
}

#[test]
fn an_empty_series_has_no_bars_and_no_legs() {
    let readings = Readings::compute(&[]);
    let series = ProgressionSeries::compute(&[], &readings);

    assert!(series.is_empty());
    assert_eq!(series.len(), 0);
    assert_eq!(series.get(0), None);
    assert_eq!(series.latest(), None);
    assert_eq!(series.legs(), []);
}

#[test]
fn a_single_candle_has_no_leg_and_reports_an_unconfirmed_origin() {
    let candle =
        Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0)).expect("the test candle is valid");
    let readings = Readings::compute(&[candle]);
    let series = ProgressionSeries::compute(&[candle], &readings);
    let bar = series.latest().expect("one bar");

    assert_eq!(bar.progress, None);
    assert_eq!(bar.stage_code, None);
    assert_eq!(bar.leg_direction, None);
    assert_eq!(bar.leg_scale_code, None);
    assert_eq!(bar.minor_leg, None);
    assert_eq!(bar.major_leg, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::OriginUnconfirmed)
    );
    assert_eq!(bar.state_age_bars, 0);
    assert_eq!(bar.state_changes_50, 0);
}

#[test]
fn a_candidate_creates_a_forming_leg_with_an_unconfirmed_origin() {
    let series = series(&[(0, 100.0), (4, 110.0), (8, 90.0)]);

    // The first minor candidate is the rising high at bar 2.
    let bar = series.get(2).expect("bar 2");
    let minor = bar.minor_leg.expect("the minor leg exists");
    assert_eq!(minor.leg_id, 1);
    assert_eq!(minor.scale_code, 1);
    assert_eq!(minor.direction_code, -1);
    assert_eq!(minor.origin_price, 105.5);
    assert_eq!(minor.origin_occurred_at, 2);
    assert_eq!(minor.origin_available_at, None);
    assert_eq!(minor.origin_confirmed_code, 0);
    assert_eq!(minor.lifecycle_code, 0);
    assert_eq!(minor.stage_code, None);
    assert_eq!(bar.major_leg, None);
    assert_eq!(bar.leg_scale_code, Some(1));
    assert_eq!(bar.leg_direction, Some(-1));
    // ATR 14 is unavailable and no level is confirmed, so the forming leg
    // has no target and reports the unconfirmed origin.
    assert_eq!(minor.target_price, None);
    assert_eq!(minor.target_basis_code, None);
    assert_eq!(bar.progress, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::OriginUnconfirmed)
    );

    // The first major candidate appears once five bars stand to its left.
    let bar = series.get(6).expect("bar 6");
    let major = bar.major_leg.expect("the major leg exists");
    assert_eq!(major.scale_code, 2);
    assert_eq!(major.direction_code, 1);
    assert_eq!(major.origin_price, 99.5);
    assert_eq!(major.lifecycle_code, 0);
    assert_eq!(major.origin_confirmed_code, 0);
    assert_eq!(bar.leg_scale_code, Some(2));
}

#[test]
fn a_confirmation_promotes_the_origin_and_completes_the_prior_leg() {
    let series = series(&[(0, 100.0), (4, 110.0), (8, 90.0), (12, 110.0)]);

    // The minor high at 4 confirms at 6 and becomes active.
    let leg = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 1 && leg.origin_occurred_at == 4)
        .expect("the minor high at 4 is retained");
    assert_eq!(leg.origin_confirmed_code, 1);
    assert_eq!(leg.origin_available_at, Some(6));

    // The minor low at 8 confirms at 10 and completes the downward leg.
    let leg = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 1 && leg.origin_occurred_at == 8)
        .expect("the minor low at 8 is retained");
    assert_eq!(leg.lifecycle_code, 1);
    assert_eq!(leg.origin_confirmed_code, 1);
    assert_eq!(leg.origin_available_at, Some(10));

    let completed = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 1 && leg.origin_occurred_at == 4)
        .expect("the completed leg is retained");
    assert_eq!(completed.lifecycle_code, 2);
    assert_eq!(completed.stage_code, Some(5));
}

#[test]
fn a_major_confirmation_promotes_the_major_leg() {
    let series = series(&[
        (0, 100.0),
        (4, 110.0),
        (8, 90.0),
        (12, 110.0),
        (16, 90.0),
        (20, 110.0),
        (24, 90.0),
        (28, 110.0),
    ]);

    let leg = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 2 && leg.origin_occurred_at == 8)
        .expect("the major low at 8 is retained");
    assert_eq!(leg.origin_confirmed_code, 1);
    assert_eq!(leg.origin_available_at, Some(13));
    assert_eq!(leg.lifecycle_code, 2);
}

#[test]
fn the_published_leg_is_the_newest_record_across_scales() {
    let series = series(&[(0, 100.0), (4, 110.0), (8, 90.0), (12, 110.0)]);

    // At bar 8 the low candidate creates a new major leg, newer than the
    // minor leg.
    let bar = series.get(8).expect("bar 8");
    let minor = bar.minor_leg.expect("the minor leg exists");
    let major = bar.major_leg.expect("the major leg exists");
    assert!(major.leg_id > minor.leg_id);
    assert_eq!(bar.leg_scale_code, Some(2));
    assert_eq!(bar.leg_direction, Some(1));

    // At bar 10 a new minor candidate is newer than the major leg.
    let bar = series.get(10).expect("bar 10");
    let minor = bar.minor_leg.expect("the minor leg exists");
    let major = bar.major_leg.expect("the major leg exists");
    assert!(minor.leg_id > major.leg_id);
    assert_eq!(bar.leg_scale_code, Some(1));
    assert_eq!(bar.leg_direction, Some(-1));
}

#[test]
fn a_candidate_that_fails_confirmation_without_a_breach_is_discarded() {
    let series = series(&[(0, 100.0), (4, 110.0), (8, 90.0), (12, 110.0)]);

    // The minor high candidate at bar 10 fails confirmation at bar 12.
    // ATR 14 is still unavailable, so it never breached and is discarded.
    assert!(
        series
            .legs()
            .iter()
            .all(|leg| !(leg.scale_code == 1 && leg.origin_occurred_at == 10))
    );
    for leg in series.legs() {
        assert!(
            !(leg.lifecycle_code == 0 && leg.stage_code.is_some()),
            "a forming leg cannot have a stage"
        );
    }
}

#[test]
fn a_forming_leg_that_breaches_and_fails_is_invalidated() {
    let series = series(&[
        (0, 100.0),
        (4, 110.0),
        (8, 90.0),
        (12, 110.0),
        (16, 90.0),
        (20, 110.0),
        (24, 90.0),
        (28, 110.0),
    ]);

    // The minor low candidate at bar 14 falls to 89.5 at bar 16 and fails
    // confirmation; the fall is beyond its origin, so it is invalidated
    // rather than discarded.
    let leg = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 1 && leg.origin_occurred_at == 14)
        .expect("the invalidated forming leg is retained");
    assert_eq!(leg.lifecycle_code, 3);
    assert_eq!(leg.origin_confirmed_code, 0);
    assert_eq!(leg.origin_available_at, None);
    assert_eq!(leg.stage_code, Some(5));
}

#[test]
fn a_same_kind_confirmation_supersedes_the_active_leg() {
    // The high at 6 confirms at 8 and opens a downward leg. The two low
    // candidates at 9 and 10 fail without becoming active: bar 11 repeats
    // bar 10's low, and bar 12's low is above it. The high at 12 confirms
    // at 14 and supersedes the still-active downward leg from 6.
    let series = series_from_ohlc(&[
        (101.0, 99.0, 100.0),
        (103.0, 101.0, 102.0),
        (105.0, 103.0, 104.0),
        (107.0, 105.0, 106.0),
        (109.0, 107.0, 108.0),
        (111.0, 109.0, 110.0),
        (120.0, 110.0, 115.0),
        (115.0, 108.0, 110.0),
        (112.0, 105.0, 108.0),
        (108.0, 100.0, 102.0),
        (104.0, 99.0, 100.0),
        (103.0, 99.0, 102.0),
        (125.0, 101.0, 120.0),
        (122.0, 110.0, 118.0),
        (121.0, 112.0, 119.0),
    ]);

    let prior = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 1 && leg.origin_occurred_at == 6)
        .expect("the first downward leg is retained");
    assert_eq!(prior.origin_confirmed_code, 1);
    assert_eq!(prior.lifecycle_code, 4);
    assert_eq!(prior.stage_code, Some(5));

    let newer = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 1 && leg.origin_occurred_at == 12)
        .expect("the newer downward leg is retained");
    assert_eq!(newer.origin_confirmed_code, 1);
    assert_eq!(newer.origin_available_at, Some(14));
    assert_eq!(newer.lifecycle_code, 1);
}

#[test]
fn a_close_beyond_the_origin_invalidates_an_active_leg() {
    // Fourteen flat, volatile bars give ATR 14 = 5.0 at bar 14. The high
    // at 14 confirms at 16 and opens a downward leg with origin 120 and a
    // 1.0 invalidation threshold. The low candidate at 18 fails at 20
    // because bar 19 repeats its low, so no opposing leg confirms. The
    // close of 122 at bar 20 is beyond the origin by 2.0 and invalidates
    // the active leg.
    let mut rows = vec![(104.0, 100.0, 102.0); 14];
    rows.extend_from_slice(&[
        (120.0, 103.0, 118.0),
        (117.0, 110.0, 112.0),
        (116.0, 108.0, 110.0),
        (112.0, 104.0, 106.0),
        (108.0, 100.0, 102.0),
        (106.0, 100.0, 104.0),
        (125.0, 105.0, 122.0),
    ]);
    let series = series_from_ohlc(&rows);

    let leg = series
        .legs()
        .iter()
        .find(|leg| leg.scale_code == 1 && leg.origin_occurred_at == 14)
        .expect("the invalidated leg is retained");
    assert_eq!(leg.origin_confirmed_code, 1);
    assert_eq!(leg.origin_available_at, Some(16));
    assert_eq!(leg.lifecycle_code, 3);
    assert_eq!(leg.stage_code, Some(5));
    assert_eq!(leg.displacement_price, Some(2.0));
}

#[test]
fn a_fallback_target_becomes_structural_when_an_older_pivot_confirms() {
    let series = series(&[
        (0, 100.0),
        (4, 110.0),
        (8, 90.0),
        (12, 110.0),
        (16, 90.0),
        (20, 110.0),
        (24, 90.0),
        (28, 110.0),
    ]);

    // The major low candidate at 15 has no structural high above it yet,
    // so its target is the ATR fallback at creation.
    let bar = series.get(15).expect("bar 15");
    let leg = bar.major_leg.expect("the major leg exists");
    assert_eq!(leg.origin_occurred_at, 15);
    assert_eq!(leg.target_basis_code, Some(2));
    assert_eq!(leg.target_version, 0);
    assert_eq!(leg.target_stable_code, 1);

    // The major high at 12 confirms at 17 and becomes the nearest
    // structural target above the origin.
    let leg_id = leg.leg_id;
    let revised = series
        .legs()
        .iter()
        .find(|candidate| candidate.leg_id == leg_id)
        .expect("the revised leg is retained");
    assert_eq!(revised.target_basis_code, Some(1));
    assert_eq!(revised.target_price, Some(110.5));
    assert_eq!(revised.target_version, 1);
    assert_eq!(revised.target_stable_code, 1);
}

#[test]
fn a_flat_series_has_no_leg() {
    let candles: Vec<Candle> = (0..40)
        .map(|bar| {
            Candle::new(bar, bar as i64, 100.0, 100.0, 100.0, 100.0, Some(0.0))
                .expect("the test candle is valid")
        })
        .collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    for bar in 0..series.len() {
        let bar = series.get(bar).expect("a bar");
        assert_eq!(bar.leg_direction, None);
        assert_eq!(bar.readiness, Readiness::Unavailable);
        assert_eq!(
            bar.unavailable_reason,
            Some(UnavailableReason::OriginUnconfirmed)
        );
    }
    assert!(series.legs().is_empty());
}

#[test]
fn a_missing_candle_nulls_the_published_measurements() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&[], &readings);

    let bar = series.get(20).expect("bar 20");
    assert_eq!(bar.progress, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::InsufficientBars)
    );
}

#[test]
fn the_timeframe_is_stamped_on_every_leg_when_configured() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let series =
        ProgressionSeries::compute_with_config(&candles, &readings, ProgressionConfig::new(300));

    assert!(
        series
            .legs()
            .iter()
            .all(|leg| leg.timeframe_secs == Some(300))
    );
    let bar = series.get(20).expect("bar 20");
    for leg in [bar.minor_leg, bar.major_leg].into_iter().flatten() {
        assert_eq!(leg.timeframe_secs, Some(300));
    }
}

#[test]
fn the_series_length_matches_the_readings_length() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    assert_eq!(series.len(), readings.len());
    assert_eq!(series.bars().len(), readings.len());
}

#[test]
fn later_bars_do_not_change_earlier_records() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let full = ProgressionSeries::compute(&candles, &readings);

    for length in 1..=candles.len() {
        let prefix =
            ProgressionSeries::compute(&candles[..length], &Readings::compute(&candles[..length]));
        assert_eq!(full.get(length - 1), prefix.get(length - 1), "bar {length}");
    }
}

#[test]
fn changing_a_later_candle_does_not_change_earlier_records() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let baseline = ProgressionSeries::compute(&candles, &readings);

    for change_at in [5, 10, 20, 28] {
        let mut changed = candles.clone();
        let candle = changed[change_at];
        changed[change_at] = Candle::new(
            candle.series_index(),
            candle.timestamp(),
            candle.open() * 0.5,
            candle.high() * 1.5,
            candle.low() * 0.5,
            candle.close() * 1.2,
            candle.volume(),
        )
        .expect("the changed candle is valid");

        let changed_readings = Readings::compute(&changed);
        let after = ProgressionSeries::compute(&changed, &changed_readings);
        for bar in 0..change_at {
            assert_eq!(
                baseline.get(bar),
                after.get(bar),
                "bar {bar} changed by the candle at {change_at}"
            );
        }
    }
}

#[test]
fn repeated_runs_are_identical() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);

    assert_eq!(
        ProgressionSeries::compute(&candles, &readings),
        ProgressionSeries::compute(&candles, &readings)
    );
}

#[test]
fn timestamps_do_not_affect_the_output() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let baseline = ProgressionSeries::compute(&candles, &readings);

    let shifted: Vec<Candle> = candles
        .iter()
        .map(|candle| {
            Candle::new(
                candle.series_index(),
                candle.timestamp() + 1_000_000,
                candle.open(),
                candle.high(),
                candle.low(),
                candle.close(),
                candle.volume(),
            )
            .expect("the shifted candle is valid")
        })
        .collect();

    let shifted_readings = Readings::compute(&shifted);
    assert_eq!(
        baseline,
        ProgressionSeries::compute(&shifted, &shifted_readings)
    );
}

#[test]
fn irregular_timestamps_do_not_affect_the_output() {
    // The axis never reads a timestamp; only bar order and the configured
    // timeframe matter. Non-uniform spacing, including gaps and a repeated
    // second, must leave every published value unchanged.
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let baseline = ProgressionSeries::compute(&candles, &readings);

    let irregular: Vec<Candle> = candles
        .iter()
        .enumerate()
        .map(|(bar, candle)| {
            let timestamp = match bar % 4 {
                0 => (bar as i64) * 60,
                1 => (bar as i64) * 60 + 7,
                2 => (bar as i64) * 60 + 7,
                _ => (bar as i64) * 300,
            };
            Candle::new(
                candle.series_index(),
                timestamp,
                candle.open(),
                candle.high(),
                candle.low(),
                candle.close(),
                candle.volume(),
            )
            .expect("the irregular candle is valid")
        })
        .collect();

    let irregular_readings = Readings::compute(&irregular);
    assert_eq!(
        baseline,
        ProgressionSeries::compute(&irregular, &irregular_readings)
    );
}

#[test]
fn zero_volume_does_not_change_the_output() {
    let candles = reference_path();
    let readings = Readings::compute(&candles);
    let baseline = ProgressionSeries::compute(&candles, &readings);
    let zero_volume: Vec<Candle> = candles
        .iter()
        .map(|candle| {
            Candle::new(
                candle.series_index(),
                candle.timestamp(),
                candle.open(),
                candle.high(),
                candle.low(),
                candle.close(),
                Some(0.0),
            )
            .expect("the zero-volume candle is valid")
        })
        .collect();
    let zero_readings = Readings::compute(&zero_volume);

    assert_eq!(
        baseline,
        ProgressionSeries::compute(&zero_volume, &zero_readings)
    );
}

#[test]
fn a_price_gap_still_creates_a_leg_at_its_candle() {
    let mut candles = reference_path();
    // A gap upward is a strict high, so it creates a downward leg on its own
    // bar. The gap is a price discontinuity, not a missing candle.
    candles[9] =
        Candle::new(9, 9, 120.0, 130.0, 110.0, 120.0, Some(0.0)).expect("the test candle is valid");
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    assert_eq!(series.len(), candles.len());
    assert!(
        series
            .legs()
            .iter()
            .any(|leg| leg.origin_occurred_at == 9 && leg.direction_code == -1),
        "the gap bar is a strict high and creates a downward leg"
    );
}

#[test]
fn equal_highs_and_equal_lows_create_one_candidate_and_no_confirmed_pivot() {
    // Two equal highs on a plateau and two equal lows on a later plateau. The
    // first extreme is a candidate because it is strict against its left
    // window; the equal second extreme is not. Both first candidates fail
    // confirmation because the equal extreme is not strictly beyond them, and
    // without an ATR at their origin they are discarded rather than
    // invalidated.
    let mut rows = vec![(100.0, 95.0, 98.0); 24];
    rows[4] = (110.0, 95.0, 105.0);
    rows[5] = (110.0, 95.0, 105.0);
    rows[14] = (100.0, 90.0, 92.0);
    rows[15] = (100.0, 90.0, 92.0);
    let series = series_from_ohlc(&rows);

    let bar = series.get(4).expect("bar 4");
    let high = bar.minor_leg.expect("the first high is a candidate");
    assert_eq!(high.origin_occurred_at, 4);
    assert_eq!(high.direction_code, -1);
    assert_eq!(bar.major_leg, None);

    let bar = series.get(5).expect("bar 5");
    assert_eq!(
        bar.minor_leg
            .expect("the candidate is still retained")
            .origin_occurred_at,
        4
    );

    let bar = series.get(14).expect("bar 14");
    assert_eq!(
        bar.minor_leg
            .expect("the first low is a candidate")
            .origin_occurred_at,
        14
    );
    assert_eq!(
        bar.major_leg
            .expect("the first low is a major candidate")
            .origin_occurred_at,
        14
    );

    let bar = series.get(15).expect("bar 15");
    assert_eq!(
        bar.minor_leg
            .expect("the candidate is still retained")
            .origin_occurred_at,
        14
    );
    assert_eq!(
        bar.major_leg
            .expect("the candidate is still retained")
            .origin_occurred_at,
        14
    );

    // The equal second extremes did not create new legs, and the first
    // candidates were discarded at their failed confirmations.
    assert!(
        series
            .legs()
            .iter()
            .all(|leg| !matches!(leg.origin_occurred_at, 4 | 5 | 14 | 15))
    );
}

#[test]
fn every_published_stage_and_ready_are_reachable_on_a_crafted_path() {
    // A confirmed major high at bar 10 and a confirmed major low at bar 16
    // give an up leg with origin 90, structural target 120, and a usable
    // ATR at its origin. The path then holds the high and low flat so no
    // later candidate displaces the leg, and moves the close through every
    // progress bucket. Bars 30-35 repeat a lower low so both scale
    // candidates fail and are discarded; the active leg becomes published
    // again at bar 35 with a negative progress, which is stage 0.
    let mut rows = vec![(119.0, 91.0, 100.0); 10];
    rows.push((120.0, 110.0, 118.0)); // bar 10: major high candidate
    rows.push((119.0, 108.0, 110.0)); // bar 11
    rows.push((119.0, 106.0, 108.0)); // bar 12
    rows.push((119.0, 104.0, 106.0)); // bar 13
    rows.push((119.0, 102.0, 104.0)); // bar 14
    rows.push((119.0, 91.0, 100.0)); // bar 15: low candidate, fails
    rows.push((119.0, 90.0, 91.0)); // bar 16: origin of the up leg
    rows.extend(std::iter::repeat_n((119.0, 91.0, 91.0), 5)); // 17-21
    rows.extend([
        (119.0, 91.0, 92.0), // 22: stage 1
        (119.0, 91.0, 95.0), // 23
        (119.0, 91.0, 97.5), // 24: stage 2
        (119.0, 91.0, 97.5), // 25
        (119.0, 91.0, 97.5), // 26
        (119.0, 91.0, 97.5), // 27
        (119.0, 91.0, 97.5), // 28
        (119.0, 91.0, 97.5), // 29
    ]);
    rows.extend(std::iter::repeat_n((119.0, 89.0, 89.5), 6)); // 30-35
    rows.extend([
        (119.0, 91.0, 92.0),  // 36: stage 1 again
        (119.0, 91.0, 97.5),  // 37: stage 2
        (119.0, 91.0, 105.0), // 38: stage 3
        (119.0, 91.0, 112.5), // 39: stage 4
    ]);
    rows.extend(std::iter::repeat_n((119.0, 91.0, 112.5), 61)); // 40-100
    let series = series_from_ohlc(&rows);

    let stage = |bar: usize| series.get(bar).expect("a bar").stage_code;
    assert_eq!(stage(22), Some(1));
    assert_eq!(stage(24), Some(2));
    assert_eq!(stage(35), Some(0));
    assert_eq!(stage(36), Some(1));
    assert_eq!(stage(37), Some(2));
    assert_eq!(stage(38), Some(3));
    assert_eq!(stage(39), Some(4));

    // The stage-bearing bars before 50 samples are provisional, and the
    // first bar with 50 prior magnitude samples is ready.
    assert_eq!(
        series.get(39).expect("bar 39").readiness,
        Readiness::Provisional
    );
    let ready = series.get(66).expect("bar 66");
    assert_eq!(ready.stage_code, Some(4));
    assert_eq!(ready.readiness, Readiness::Ready);
    assert!(ready.size_percentile.is_some());
    assert!(ready.state_changes_50 > 0);
    assert_eq!(ready.state_age_bars, 28);

    // The leg is published as one record; its stage is never blended with
    // the other scale.
    let bar = series.get(39).expect("bar 39");
    assert_eq!(bar.leg_scale_code, Some(2));
    assert_eq!(bar.leg_direction, Some(1));
    assert_eq!(bar.published_leg().expect("a published leg").scale_code, 2);
}

fn bare_leg() -> Leg {
    let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&[]));
    let candidate = PivotCandidate {
        scale: PivotScale::Minor,
        kind: PivotKind::Low,
        price: 100.0,
        occurred_at: 0,
        available_at: 0,
    };
    Leg::new(1, candidate, &registry, None)
}

#[test]
fn ready_requires_a_stable_target() {
    let mut leg = bare_leg();
    leg.target_stable = true;
    leg.progress = Some(0.5);
    leg.stage = Some(2);

    assert_eq!(
        readiness(Some(&leg), Some(0.5), Some(2), Some(50.0)),
        Readiness::Ready
    );

    // A terminal leg publishes stage 5 while its target can still be inside
    // the three-bar stability window; it is provisional, not ready.
    leg.target_stable = false;
    assert_eq!(
        readiness(Some(&leg), Some(0.5), Some(2), Some(50.0)),
        Readiness::Provisional
    );
    assert_eq!(
        readiness(Some(&leg), None, Some(2), Some(50.0)),
        Readiness::Unavailable
    );
    assert_eq!(readiness(None, None, None, None), Readiness::Unavailable);
}

#[test]
fn a_zero_atr_is_not_a_usable_origin_distance() {
    let candles: Vec<Candle> = (0..40)
        .map(|bar| {
            Candle::new(bar, bar as i64, 100.0, 100.0, 100.0, 100.0, Some(0.0))
                .expect("the test candle is valid")
        })
        .collect();
    let readings = Readings::compute(&candles);

    assert_eq!(readings.atr14.get(20), Some(0.0));
    assert_eq!(super::usable_atr(&readings, 20), None);
}

#[test]
fn a_missing_candle_without_a_leg_reports_an_unconfirmed_origin() {
    let candles: Vec<Candle> = (0..40)
        .map(|bar| {
            Candle::new(bar, bar as i64, 100.0, 100.0, 100.0, 100.0, Some(0.0))
                .expect("the test candle is valid")
        })
        .collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&[], &readings);
    let bar = series.get(20).expect("bar 20");

    // With no leg, the origin is unconfirmed even though the candle is also
    // missing; the no-leg condition is the axis's primary unavailability.
    assert_eq!(bar.leg_direction, None);
    assert_eq!(bar.progress, None);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::OriginUnconfirmed)
    );
}
