//! Golden-fixture, causality, determinism, and edge-case tests for Direction.
//!
//! The fixture is a checked-in CSV of 260 candles and the expected Direction
//! values on every bar. Its expected values were produced by an independent
//! Python implementation of contract §8.1 and §9 from the independently
//! generated readings fixture, not by the Rust code under test.
//!
//! The fixture has one row per bar. An empty Direction cell is an unavailable
//! measurement; a numeric cell is a measured value, and a measured zero is
//! written as `0.0`.

use quantrule_data::Candle;
use quantrule_model::axes::direction::DirectionSeries;
use quantrule_model::kernel::{Readiness, UnavailableReason, compact_state};
use quantrule_model::readings::{PivotSeries, ReadingSeries, Readings};

const FIXTURE: &str = include_str!("fixtures/direction_golden.csv");
const TOLERANCE: f64 = 1e-9;

const HEADER: &str = "bar,timestamp,open,high,low,close,volume,direction,direction_state_code,direction_size_percentile,direction_readiness_code,direction_unavailable_reason_code,direction_state_age_bars,direction_state_changes_50";

#[derive(Debug, Clone, Copy)]
struct GoldenRow {
    bar: usize,
    candle: Candle,
    direction: Option<f64>,
    state_code: Option<i8>,
    size_percentile: Option<f64>,
    readiness: u8,
    unavailable_reason: Option<u8>,
    state_age_bars: u64,
    state_changes_50: u64,
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

fn load() -> Vec<GoldenRow> {
    let mut lines = FIXTURE.lines();
    assert_eq!(lines.next(), Some(HEADER), "fixture header changed");

    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 14, "wrong column count in {line:?}");

        let bar = integer::<usize>(fields[0]);
        assert_eq!(bar, rows.len(), "fixture bars must be contiguous");

        let candle = Candle::new(
            bar as u64,
            integer::<i64>(fields[1]),
            number(fields[2]).expect("open"),
            number(fields[3]).expect("high"),
            number(fields[4]).expect("low"),
            number(fields[5]).expect("close"),
            number(fields[6]),
        )
        .expect("the fixture candle is valid");

        rows.push(GoldenRow {
            bar,
            candle,
            direction: number(fields[7]),
            state_code: optional_integer::<i8>(fields[8]),
            size_percentile: number(fields[9]),
            readiness: integer::<u8>(fields[10]),
            unavailable_reason: optional_integer::<u8>(fields[11]),
            state_age_bars: integer::<u64>(fields[12]),
            state_changes_50: integer::<u64>(fields[13]),
        });
    }
    rows
}

fn candles(rows: &[GoldenRow]) -> Vec<Candle> {
    rows.iter().map(|row| row.candle).collect()
}

fn assert_close(actual: Option<f64>, expected: Option<f64>, label: &str) {
    match (actual, expected) {
        (None, None) => {}
        (Some(actual), Some(expected)) => assert!(
            (actual - expected).abs() <= TOLERANCE,
            "{label}: expected {expected:.17}, got {actual:.17}"
        ),
        _ => panic!("{label}: expected {expected:?}, got {actual:?}"),
    }
}

fn assert_matches(actual: DirectionSeries, rows: &[GoldenRow]) {
    assert_eq!(actual.len(), rows.len(), "series length");

    for row in rows {
        let bar = actual.get(row.bar).expect("a bar for every row");
        assert_close(
            bar.direction,
            row.direction,
            &format!("direction {}", row.bar),
        );
        assert_eq!(bar.state_code, row.state_code, "state code {}", row.bar);
        assert_close(
            bar.size_percentile,
            row.size_percentile,
            &format!("size percentile {}", row.bar),
        );
        assert_eq!(bar.readiness.code(), row.readiness, "readiness {}", row.bar);
        assert_eq!(
            bar.unavailable_reason.map(UnavailableReason::code),
            row.unavailable_reason,
            "unavailable reason {}",
            row.bar
        );
        assert_eq!(
            bar.state_age_bars, row.state_age_bars,
            "state age {}",
            row.bar
        );
        assert_eq!(
            bar.state_changes_50, row.state_changes_50,
            "state changes {}",
            row.bar
        );
    }
}

/// Rebuilds the series with one candle moved by `price_move`.
fn replace_candle(candles: &[Candle], bar: usize, price_move: f64) -> Vec<Candle> {
    let mut changed = candles.to_vec();
    let old = changed[bar];
    let close = old.close() + price_move;
    let padding = price_move.abs();
    let high = old.high().max(close) + padding;
    let low = old.low().min(close) - padding;
    changed[bar] = Candle::new(
        old.series_index(),
        old.timestamp(),
        old.open(),
        high,
        low,
        close,
        old.volume(),
    )
    .expect("the changed candle is valid");
    changed
}

#[test]
fn the_fixture_covers_the_direction_lifecycle() {
    let rows = load();
    assert_eq!(rows.len(), 260);

    let nulls = rows.iter().filter(|row| row.direction.is_none()).count();
    let provisional = rows
        .iter()
        .filter(|row| row.readiness == Readiness::Provisional.code())
        .count();
    let ready = rows
        .iter()
        .filter(|row| row.readiness == Readiness::Ready.code())
        .count();
    let percentiles = rows
        .iter()
        .filter(|row| row.size_percentile.is_some())
        .count();
    let reason_one = rows
        .iter()
        .filter(|row| row.unavailable_reason == Some(UnavailableReason::InsufficientBars.code()))
        .count();

    assert_eq!(
        nulls, 199,
        "the fixture must exercise the unavailable prefix"
    );
    assert_eq!(
        reason_one, 199,
        "the unavailable prefix must carry reason 1"
    );
    assert_eq!(
        provisional, 50,
        "the fixture must exercise provisional readiness"
    );
    assert_eq!(ready, 11, "the fixture must exercise ready readiness");
    assert_eq!(percentiles, 11, "the fixture must exercise the percentile");
    assert!(rows[199].direction.is_some(), "first valid bar");
    assert_eq!(
        rows[249].readiness,
        Readiness::Ready.code(),
        "first ready bar"
    );
}

#[test]
fn every_direction_field_matches_the_golden_values() {
    let rows = load();
    let readings = Readings::compute(&candles(&rows));
    let actual = DirectionSeries::compute(&readings);

    assert_matches(actual, &rows);
}

#[test]
fn direction_is_causal_across_every_prefix() {
    let rows = load();
    let candles = candles(&rows);
    let full = DirectionSeries::compute(&Readings::compute(&candles));

    for length in 0..=candles.len() {
        let prefix = DirectionSeries::compute(&Readings::compute(&candles[..length]));
        assert_eq!(prefix.len(), length, "prefix length {length}");

        for bar in 0..length {
            assert_eq!(
                prefix.get(bar),
                full.get(bar),
                "prefix {length}, bar {bar} changed"
            );
        }
    }
}

#[test]
fn changing_a_later_candle_does_not_change_earlier_direction() {
    let rows = load();
    let candles = candles(&rows);
    let baseline = DirectionSeries::compute(&Readings::compute(&candles));

    let changed_candles = replace_candle(&candles, 250, 5.0);
    let changed = DirectionSeries::compute(&Readings::compute(&changed_candles));

    for bar in 0..250 {
        assert_eq!(
            baseline.get(bar),
            changed.get(bar),
            "bar {bar} changed after a later candle moved"
        );
    }
    assert_ne!(
        candles[250].close(),
        changed_candles[250].close(),
        "the later candle must actually change"
    );
}

#[test]
fn repeated_computation_is_deterministic() {
    let rows = load();
    let candles = candles(&rows);
    let readings = Readings::compute(&candles);

    let first = DirectionSeries::compute(&readings);
    let second = DirectionSeries::compute(&readings);
    let third = DirectionSeries::compute(&Readings::compute(&candles));

    assert_eq!(first, second);
    assert_eq!(first, third);
}

#[test]
fn timestamps_do_not_change_direction() {
    let rows = load();
    let candles = candles(&rows);
    let baseline = DirectionSeries::compute(&Readings::compute(&candles));

    let mut irregular = candles.clone();
    for (bar, candle) in irregular.iter_mut().enumerate() {
        // Deliberately uneven spacing: Direction consumes bar order, not wall
        // clock time, and the data engine owns timeframe alignment.
        let timestamp = (bar as i64) * 7_919;
        *candle = Candle::new(
            candle.series_index(),
            timestamp,
            candle.open(),
            candle.high(),
            candle.low(),
            candle.close(),
            candle.volume(),
        )
        .expect("the irregularly spaced candle is valid");
    }

    let changed = DirectionSeries::compute(&Readings::compute(&irregular));
    assert_eq!(baseline, changed);
}

#[test]
fn direction_ignores_rsi_rvol_and_pivots() {
    let rows = load();
    let readings = Readings::compute(&candles(&rows));
    let baseline = DirectionSeries::compute(&readings);

    let mut changed = readings.clone();
    changed.rsi14 = ReadingSeries::new(vec![Some(99.0); changed.len()]);
    changed.rvol20 = ReadingSeries::new(vec![Some(0.01); changed.len()]);
    changed.pivots = PivotSeries::compute(&[]);

    assert_eq!(baseline, DirectionSeries::compute(&changed));
}

#[test]
fn every_published_value_obeys_the_contract_bounds() {
    let rows = load();
    let series = DirectionSeries::compute(&Readings::compute(&candles(&rows)));

    for bar in 0..series.len() {
        let record = series.get(bar).expect("a bar");
        match record.direction {
            Some(direction) => {
                assert!(
                    (-1.0..=1.0).contains(&direction),
                    "bar {bar}: direction {direction} is outside [-1, +1]"
                );
                assert_eq!(
                    record.state_code,
                    compact_state(direction),
                    "bar {bar}: state code"
                );
                assert!(matches!(
                    record.readiness,
                    Readiness::Provisional | Readiness::Ready
                ));
                assert_eq!(record.unavailable_reason, None, "bar {bar}");
                assert!(record.state_age_bars >= 1, "bar {bar}");
                if let Some(percentile) = record.size_percentile {
                    assert!(
                        (0.0..=100.0).contains(&percentile),
                        "bar {bar}: percentile {percentile} is outside [0, 100]"
                    );
                }
            }
            None => {
                assert_eq!(record.state_code, None, "bar {bar}");
                assert_eq!(record.size_percentile, None, "bar {bar}");
                assert_eq!(record.readiness, Readiness::Unavailable, "bar {bar}");
                assert!(record.unavailable_reason.is_some(), "bar {bar}");
            }
        }
        assert!(record.state_changes_50 <= 50, "bar {bar}");
    }
}

#[test]
fn an_empty_series_has_no_bars() {
    let series = DirectionSeries::compute(&Readings::compute(&[]));

    assert!(series.is_empty());
    assert_eq!(series.len(), 0);
    assert_eq!(series.get(0), None);
    assert_eq!(series.latest(), None);
}

#[test]
fn a_single_candle_is_unavailable() {
    let rows = load();
    let series = DirectionSeries::compute(&Readings::compute(&candles(&rows)[..1]));
    let bar = series.get(0).expect("one bar");

    assert_eq!(bar.direction, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::InsufficientBars)
    );
    assert_eq!(bar.state_age_bars, 0);
}

#[test]
fn a_series_shorter_than_the_required_history_is_unavailable() {
    let rows = load();
    let candles = candles(&rows);

    let short = DirectionSeries::compute(&Readings::compute(&candles[..199]));
    assert_eq!(short.len(), 199);
    assert!(short.bars().iter().all(|bar| bar.direction.is_none()));

    let just_long_enough = DirectionSeries::compute(&Readings::compute(&candles[..200]));
    assert_eq!(
        just_long_enough.get(198).expect("a bar").direction,
        None,
        "EMA 200 is not available on bar 198"
    );
    assert!(
        just_long_enough
            .get(199)
            .expect("a bar")
            .direction
            .is_some(),
        "bar 199 is the first bar with EMA 200 and ATR 14"
    );
}

#[test]
fn a_gap_produces_a_finite_direction_after_the_required_history() {
    let rows = load();
    let mut candles = candles(&rows);
    let gap = candles[100];
    candles[100] = Candle::new(
        gap.series_index(),
        gap.timestamp(),
        gap.open(),
        gap.high() + 10.0,
        gap.low() - 10.0,
        gap.close() + 5.0,
        gap.volume(),
    )
    .expect("the gapped candle is valid");

    let series = DirectionSeries::compute(&Readings::compute(&candles));
    let bar = series.latest().expect("a latest bar");

    let Some(direction) = bar.direction else {
        panic!("the gapped series must publish a direction");
    };
    assert!(direction.is_finite());
}

#[test]
fn zero_volume_does_not_change_direction() {
    let rows = load();
    let mut changed = candles(&rows);
    let original = changed[150];
    let zero_volume = Candle::new(
        original.series_index(),
        original.timestamp(),
        original.open(),
        original.high(),
        original.low(),
        original.close(),
        Some(0.0),
    )
    .expect("the zero-volume candle is valid");
    changed[150] = zero_volume;

    let with_zero = DirectionSeries::compute(&Readings::compute(&changed));
    let with_missing = DirectionSeries::compute(&Readings::compute(&candles(&rows)));

    assert_eq!(with_zero, with_missing);
}

#[test]
fn a_flat_series_with_equal_highs_and_lows_has_zero_atr() {
    let candles: Vec<Candle> = (0..220)
        .map(|bar| {
            Candle::new(
                bar as u64,
                bar as i64,
                100.0,
                100.0,
                100.0,
                100.0,
                Some(10.0),
            )
            .expect("the flat candle is valid")
        })
        .collect();
    let series = DirectionSeries::compute(&Readings::compute(&candles));
    let bar = series.latest().expect("a latest bar");

    assert_eq!(bar.direction, None);
    assert_eq!(bar.state_code, None);
    assert_eq!(bar.size_percentile, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::InvalidDenominator)
    );
    assert_eq!(bar.state_age_bars, 0);
    assert_eq!(bar.state_changes_50, 0);
}
