//! Golden-fixture, causality, determinism, and edge-case tests for Progression.
//!
//! The per-bar fixture is a checked-in CSV of 260 candles and the expected
//! Progression values on every bar, including the full record of the latest
//! minor and major leg. The ledger fixture is the final record of every
//! retained leg in creation order. Both were produced by an independent Python
//! implementation of contract §8.4 and §9 from the independently generated ATR
//! and pivot columns of the readings fixture, not by the Rust code under test.
//!
//! An empty cell is an unavailable measurement; a numeric cell is a measured
//! value, and a measured zero is written as `0.0`.

use std::collections::HashMap;

use quantrule_data::Candle;
use quantrule_model::axes::progression::{ProgressionBar, ProgressionLeg, ProgressionSeries};
use quantrule_model::kernel::{Readiness, UnavailableReason};
use quantrule_model::readings::Readings;

const FIXTURE: &str = include_str!("fixtures/progression_golden.csv");
const LEDGER_FIXTURE: &str = include_str!("fixtures/progression_legs_golden.csv");
const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy)]
struct GoldenLeg {
    leg_id: u64,
    timeframe_secs: Option<u64>,
    scale_code: u8,
    origin_price: f64,
    origin_occurred_at: usize,
    origin_available_at: Option<usize>,
    origin_confirmed_code: u8,
    direction_code: i8,
    target_price: Option<f64>,
    target_basis_code: Option<u8>,
    target_version: u32,
    target_stable_code: u8,
    progress: Option<f64>,
    displacement_price: Option<f64>,
    displacement_atr: Option<f64>,
    lifecycle_code: u8,
    stage_code: Option<i8>,
    target_reached_code: Option<u8>,
}

#[derive(Debug, Clone)]
struct GoldenRow {
    candle: Candle,
    progress: Option<f64>,
    stage_code: Option<i8>,
    leg_direction: Option<i8>,
    leg_scale_code: Option<u8>,
    size_percentile: Option<f64>,
    readiness: Readiness,
    unavailable_reason: Option<UnavailableReason>,
    state_age_bars: u64,
    state_changes_50: u64,
    minor_leg: Option<GoldenLeg>,
    major_leg: Option<GoldenLeg>,
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

fn columns(header: &str) -> HashMap<&str, usize> {
    header
        .split(',')
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect()
}

fn field<'a>(fields: &[&'a str], index: &HashMap<&str, usize>, name: &str) -> &'a str {
    fields[*index
        .get(name)
        .unwrap_or_else(|| panic!("the fixture has no {name} column"))]
}

fn parse_leg(fields: &[&str], index: &HashMap<&str, usize>, prefix: &str) -> Option<GoldenLeg> {
    let value = |name: &str| -> &str {
        let column = if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}_{name}")
        };
        // The column name is rebuilt for each lookup so the returned slice
        // borrows from the fixture fields, not from the temporary name.
        field(fields, index, &column)
    };
    let leg_id = value("leg_id");
    if leg_id.is_empty() {
        return None;
    }

    Some(GoldenLeg {
        leg_id: integer(leg_id),
        timeframe_secs: optional_integer(value("timeframe_secs")),
        scale_code: integer(value("scale_code")),
        origin_price: number(value("origin_price")).expect("origin price"),
        origin_occurred_at: integer(value("origin_occurred_at")),
        origin_available_at: optional_integer(value("origin_available_at")),
        origin_confirmed_code: integer(value("origin_confirmed_code")),
        direction_code: integer(value("direction_code")),
        target_price: number(value("target_price")),
        target_basis_code: optional_integer(value("target_basis_code")),
        target_version: integer(value("target_version")),
        target_stable_code: integer(value("target_stable_code")),
        progress: number(value("progress")),
        displacement_price: number(value("displacement_price")),
        displacement_atr: number(value("displacement_atr")),
        lifecycle_code: integer(value("lifecycle_code")),
        stage_code: optional_integer(value("stage_code")),
        target_reached_code: optional_integer(value("target_reached_code")),
    })
}

fn load() -> Vec<GoldenRow> {
    let mut lines = FIXTURE.lines();
    let header = lines.next().expect("the fixture has a header");
    let index = columns(header);

    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), index.len(), "wrong column count in {line:?}");

        let bar = integer::<usize>(field(&fields, &index, "bar"));
        assert_eq!(bar, rows.len(), "fixture bars must be contiguous");
        let candle = Candle::new(
            bar as u64,
            integer::<i64>(field(&fields, &index, "timestamp")),
            number(field(&fields, &index, "open")).expect("open"),
            number(field(&fields, &index, "high")).expect("high"),
            number(field(&fields, &index, "low")).expect("low"),
            number(field(&fields, &index, "close")).expect("close"),
            number(field(&fields, &index, "volume")),
        )
        .expect("the fixture candle is valid");

        let readiness =
            Readiness::from_code(integer::<u8>(field(&fields, &index, "readiness_code")))
                .expect("a known readiness code");
        let unavailable_reason =
            optional_integer::<u8>(field(&fields, &index, "unavailable_reason_code"))
                .map(|code| UnavailableReason::from_code(code).expect("a known reason code"));

        rows.push(GoldenRow {
            candle,
            progress: number(field(&fields, &index, "progress")),
            stage_code: optional_integer(field(&fields, &index, "stage_code")),
            leg_direction: optional_integer(field(&fields, &index, "leg_direction")),
            leg_scale_code: optional_integer(field(&fields, &index, "leg_scale_code")),
            size_percentile: number(field(&fields, &index, "size_percentile")),
            readiness,
            unavailable_reason,
            state_age_bars: integer(field(&fields, &index, "state_age_bars")),
            state_changes_50: integer(field(&fields, &index, "state_changes_50")),
            minor_leg: parse_leg(&fields, &index, "minor"),
            major_leg: parse_leg(&fields, &index, "major"),
        });
    }

    rows
}

fn load_ledger() -> Vec<GoldenLeg> {
    let mut lines = LEDGER_FIXTURE.lines();
    let header = lines.next().expect("the ledger has a header");
    let index = columns(header);

    let mut legs = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(
            fields.len(),
            index.len(),
            "wrong ledger column count in {line:?}"
        );
        let leg = parse_leg(&fields, &index, "").expect("the ledger row has a leg");
        legs.push(leg);
    }

    legs
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

fn check_leg(actual: Option<ProgressionLeg>, expected: Option<GoldenLeg>, context: &str) {
    match (actual, expected) {
        (None, None) => {}
        (Some(actual), Some(expected)) => {
            assert_eq!(actual.leg_id, expected.leg_id, "{context}: leg_id");
            assert_eq!(
                actual.timeframe_secs, expected.timeframe_secs,
                "{context}: timeframe_secs"
            );
            assert_eq!(actual.scale_code, expected.scale_code, "{context}: scale");
            assert_close(
                Some(actual.origin_price),
                Some(expected.origin_price),
                &format!("{context}: origin_price"),
            );
            assert_eq!(
                actual.origin_occurred_at, expected.origin_occurred_at,
                "{context}: origin_occurred_at"
            );
            assert_eq!(
                actual.origin_available_at, expected.origin_available_at,
                "{context}: origin_available_at"
            );
            assert_eq!(
                actual.origin_confirmed_code, expected.origin_confirmed_code,
                "{context}: origin_confirmed_code"
            );
            assert_eq!(
                actual.direction_code, expected.direction_code,
                "{context}: direction_code"
            );
            assert_close(
                actual.target_price,
                expected.target_price,
                &format!("{context}: target_price"),
            );
            assert_eq!(
                actual.target_basis_code, expected.target_basis_code,
                "{context}: target_basis_code"
            );
            assert_eq!(
                actual.target_version, expected.target_version,
                "{context}: target_version"
            );
            assert_eq!(
                actual.target_stable_code, expected.target_stable_code,
                "{context}: target_stable_code"
            );
            assert_close(
                actual.progress,
                expected.progress,
                &format!("{context}: leg progress"),
            );
            assert_close(
                actual.displacement_price,
                expected.displacement_price,
                &format!("{context}: displacement_price"),
            );
            assert_close(
                actual.displacement_atr,
                expected.displacement_atr,
                &format!("{context}: displacement_atr"),
            );
            assert_eq!(
                actual.lifecycle_code, expected.lifecycle_code,
                "{context}: lifecycle_code"
            );
            assert_eq!(
                actual.stage_code, expected.stage_code,
                "{context}: leg stage_code"
            );
            assert_eq!(
                actual.target_reached_code, expected.target_reached_code,
                "{context}: target_reached_code"
            );
        }
        _ => panic!("{context}: expected {expected:?}, got {actual:?}"),
    }
}

fn check_bar(actual: &ProgressionBar, expected: &GoldenRow, context: &str) {
    assert_close(
        actual.progress,
        expected.progress,
        &format!("{context}: progress"),
    );
    assert_eq!(
        actual.stage_code, expected.stage_code,
        "{context}: stage_code"
    );
    assert_eq!(
        actual.leg_direction, expected.leg_direction,
        "{context}: leg_direction"
    );
    assert_eq!(
        actual.leg_scale_code, expected.leg_scale_code,
        "{context}: leg_scale_code"
    );
    assert_close(
        actual.size_percentile,
        expected.size_percentile,
        &format!("{context}: size_percentile"),
    );
    assert_eq!(actual.readiness, expected.readiness, "{context}: readiness");
    assert_eq!(
        actual.unavailable_reason, expected.unavailable_reason,
        "{context}: unavailable_reason"
    );
    assert_eq!(
        actual.state_age_bars, expected.state_age_bars,
        "{context}: state_age_bars"
    );
    assert_eq!(
        actual.state_changes_50, expected.state_changes_50,
        "{context}: state_changes_50"
    );
    check_leg(
        actual.minor_leg,
        expected.minor_leg,
        &format!("{context}: minor leg"),
    );
    check_leg(
        actual.major_leg,
        expected.major_leg,
        &format!("{context}: major leg"),
    );
}

#[test]
fn the_per_bar_fixture_matches_the_computed_series() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    assert_eq!(series.len(), rows.len());
    for (bar, expected) in rows.iter().enumerate() {
        let actual = series.get(bar).expect("a computed bar");
        check_bar(&actual, expected, &format!("bar {bar}"));
    }
}

#[test]
fn the_ledger_fixture_matches_the_retained_legs() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);
    let expected = load_ledger();

    assert_eq!(series.legs().len(), expected.len());
    for (position, golden) in expected.iter().enumerate() {
        let actual = series.legs().get(position).copied();
        check_leg(actual, Some(*golden), &format!("ledger leg {position}"));
    }
}

#[test]
fn every_prefix_matches_the_full_series() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let full = ProgressionSeries::compute(&candles, &readings);

    for length in 1..=candles.len() {
        let prefix =
            ProgressionSeries::compute(&candles[..length], &Readings::compute(&candles[..length]));
        assert_eq!(
            prefix.get(length - 1),
            full.get(length - 1),
            "bar {} changed when later bars were appended",
            length - 1
        );
    }
}

#[test]
fn repeated_runs_are_identical() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);

    assert_eq!(
        ProgressionSeries::compute(&candles, &readings),
        ProgressionSeries::compute(&candles, &readings)
    );
}

#[test]
fn timestamps_do_not_affect_the_output() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
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
fn an_empty_series_publishes_nothing() {
    let readings = Readings::compute(&[]);
    let series = ProgressionSeries::compute(&[], &readings);

    assert!(series.is_empty());
    assert!(series.legs().is_empty());
    assert_eq!(series.latest(), None);
}

#[test]
fn a_single_candle_publishes_an_unconfirmed_origin() {
    let candle =
        Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0)).expect("the test candle is valid");
    let readings = Readings::compute(&[candle]);
    let series = ProgressionSeries::compute(&[candle], &readings);
    let bar = series.latest().expect("one bar");

    assert_eq!(bar.progress, None);
    assert_eq!(bar.stage_code, None);
    assert_eq!(bar.leg_direction, None);
    assert_eq!(bar.leg_scale_code, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::OriginUnconfirmed)
    );
    assert_eq!(bar.state_age_bars, 0);
    assert_eq!(bar.state_changes_50, 0);
    assert!(series.legs().is_empty());
}

#[test]
fn a_series_shorter_than_the_pivot_history_still_aligns() {
    let candles: Vec<Candle> = (0..4)
        .map(|bar| {
            Candle::new(bar, bar as i64, 1.0, 1.0, 1.0, 1.0, Some(10.0))
                .expect("the test candle is valid")
        })
        .collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    assert_eq!(series.len(), candles.len());
    assert!(series.legs().is_empty());
}

#[test]
fn a_gap_and_a_zero_volume_candle_do_not_change_the_rules() {
    let mut candles: Vec<Candle> = (0..20)
        .map(|bar| {
            Candle::new(bar, bar as i64, 100.0, 100.5, 99.5, 100.0, Some(10.0))
                .expect("the test candle is valid")
        })
        .collect();
    candles[10] = Candle::new(10, 10, 120.0, 130.0, 110.0, 120.0, Some(0.0))
        .expect("the test candle is valid");
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    assert_eq!(series.len(), candles.len());
    assert!(series.latest().is_some());
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

    assert!(series.legs().is_empty());
    for bar in 0..series.len() {
        let bar = series.get(bar).expect("a bar");
        assert_eq!(bar.leg_direction, None);
        assert_eq!(
            bar.unavailable_reason,
            Some(UnavailableReason::OriginUnconfirmed)
        );
    }
}

#[test]
fn equal_highs_and_equal_lows_create_no_pivot() {
    let candles: Vec<Candle> = (0..24)
        .map(|bar| {
            let high = if bar % 8 == 4 { 110.0 } else { 100.0 };
            let low = if bar % 8 == 0 { 90.0 } else { 95.0 };
            Candle::new(bar, bar as i64, 100.0, high, low, 100.0, Some(10.0))
                .expect("the test candle is valid")
        })
        .collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    assert_eq!(series.len(), candles.len());
}

#[test]
fn a_zero_atr_leaves_the_leg_without_a_fallback_target() {
    let candles: Vec<Candle> = (0..40)
        .map(|bar| {
            Candle::new(bar, bar as i64, 100.0, 100.0, 100.0, 100.0, Some(0.0))
                .expect("the test candle is valid")
        })
        .collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    // The flat series produces no candidates and no legs; the ATR is a
    // measured zero, which is never used as a fallback distance.
    assert!(series.legs().is_empty());
}

#[test]
fn a_missing_candle_nulls_the_published_measurements() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&[], &readings);
    let bar = series.get(200).expect("bar 200");

    assert_eq!(bar.progress, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::InsufficientBars)
    );
}

#[test]
fn an_unconfirmed_origin_at_the_end_stays_forming() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    let forming = series
        .legs()
        .iter()
        .filter(|leg| leg.lifecycle_code == 0)
        .count();
    assert!(forming > 0, "the fixture must end with a forming leg");
    let bar = series.latest().expect("the last bar");
    assert!(bar.published_leg().is_some());
}

#[test]
fn every_lifecycle_code_is_reachable() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);
    let codes: std::collections::BTreeSet<u8> =
        series.legs().iter().map(|leg| leg.lifecycle_code).collect();

    assert_eq!(codes, std::collections::BTreeSet::from([0, 1, 2, 3, 4]));
}

#[test]
fn every_stage_code_is_reachable_in_the_scale_records() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);
    let mut codes = std::collections::BTreeSet::new();

    for bar in series.bars() {
        for leg in [bar.minor_leg, bar.major_leg].into_iter().flatten() {
            if let Some(stage) = leg.stage_code {
                codes.insert(stage);
            }
        }
    }

    // The published leg is forming on almost every bar, so the per-bar scale
    // records are what prove that every stage is reachable from the fixture.
    assert_eq!(codes, std::collections::BTreeSet::from([0, 1, 2, 3, 4, 5]));
}

#[test]
fn the_percentile_starts_at_fifty_prior_samples() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    let first = series
        .bars()
        .iter()
        .position(|bar| bar.size_percentile.is_some())
        .expect("the fixture reaches a percentile");
    let prior = series.bars()[..first]
        .iter()
        .filter(|bar| {
            bar.published_leg()
                .is_some_and(|leg| leg.displacement_atr.is_some())
        })
        .count();

    assert_eq!(prior, 50, "bar {first} is the first with 50 samples");
    assert!(
        series.bars()[first - 1].size_percentile.is_none(),
        "bar {} has only 49 samples and no percentile",
        first - 1
    );
}

#[test]
fn the_published_leg_is_one_of_the_two_scale_records() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let readings = Readings::compute(&candles);
    let series = ProgressionSeries::compute(&candles, &readings);

    for bar in 0..series.len() {
        let bar = series.get(bar).expect("a bar");
        match bar.published_leg() {
            None => {
                assert_eq!(bar.leg_direction, None);
                assert_eq!(bar.leg_scale_code, None);
            }
            Some(published) => {
                assert_eq!(bar.leg_direction, Some(published.direction_code));
                assert_eq!(bar.leg_scale_code, Some(published.scale_code));
                let is_minor = bar
                    .minor_leg
                    .is_some_and(|leg| leg.leg_id == published.leg_id);
                let is_major = bar
                    .major_leg
                    .is_some_and(|leg| leg.leg_id == published.leg_id);
                assert!(is_minor || is_major);
            }
        }
    }
}
