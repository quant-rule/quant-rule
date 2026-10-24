//! Golden-fixture, causality, determinism, and edge-case tests for Momentum.
//!
//! The fixture is a checked-in CSV of 260 candles and the expected Momentum
//! values on every bar. Its expected values were produced by an independent
//! Python implementation of contract §8.2 and §9 from the independently
//! generated `rsi14` column of the readings fixture, not by the Rust code under
//! test.
//!
//! The fixture has one row per bar. An empty Momentum cell is an unavailable
//! measurement; a numeric cell is a measured value, and a measured zero is
//! written as `0.0`.

use quantrule_data::Candle;
use quantrule_model::axes::momentum::MomentumSeries;
use quantrule_model::kernel::{Readiness, UnavailableReason, compact_state};
use quantrule_model::readings::{PivotSeries, ReadingSeries, Readings};

const FIXTURE: &str = include_str!("fixtures/momentum_golden.csv");
const TOLERANCE: f64 = 1e-9;

const HEADER: &str = "bar,timestamp,open,high,low,close,volume,momentum_force,momentum_acceleration,momentum_intensification,momentum_state_code,momentum_size_percentile,momentum_readiness_code,momentum_unavailable_reason_code,momentum_state_age_bars,momentum_state_changes_50";

#[derive(Debug, Clone, Copy)]
struct GoldenRow {
    bar: usize,
    candle: Candle,
    force: Option<f64>,
    acceleration: Option<f64>,
    intensification: Option<f64>,
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
        assert_eq!(fields.len(), 16, "wrong column count in {line:?}");

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
            force: number(fields[7]),
            acceleration: number(fields[8]),
            intensification: number(fields[9]),
            state_code: optional_integer::<i8>(fields[10]),
            size_percentile: number(fields[11]),
            readiness: integer::<u8>(fields[12]),
            unavailable_reason: optional_integer::<u8>(fields[13]),
            state_age_bars: integer::<u64>(fields[14]),
            state_changes_50: integer::<u64>(fields[15]),
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

fn assert_matches(actual: MomentumSeries, rows: &[GoldenRow]) {
    assert_eq!(actual.len(), rows.len(), "series length");

    for row in rows {
        let bar = actual.get(row.bar).expect("a bar for every row");
        assert_close(bar.force, row.force, &format!("force {}", row.bar));
        assert_close(
            bar.acceleration,
            row.acceleration,
            &format!("acceleration {}", row.bar),
        );
        assert_close(
            bar.intensification,
            row.intensification,
            &format!("intensification {}", row.bar),
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
fn the_fixture_covers_the_momentum_lifecycle() {
    let rows = load();
    assert_eq!(rows.len(), 260);

    let nulls = rows.iter().filter(|row| row.force.is_none()).count();
    let reason_one = rows
        .iter()
        .filter(|row| row.unavailable_reason == Some(UnavailableReason::InsufficientBars.code()))
        .count();
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
    let acceleration_nulls = rows.iter().filter(|row| row.acceleration.is_none()).count();
    let negative = rows.iter().filter(|row| row.state_code == Some(-1)).count();
    let neutral = rows.iter().filter(|row| row.state_code == Some(0)).count();
    let positive = rows.iter().filter(|row| row.state_code == Some(1)).count();
    let changes = rows.iter().filter(|row| row.state_changes_50 > 0).count();

    assert_eq!(
        nulls, 14,
        "the fixture must exercise the unavailable prefix"
    );
    assert_eq!(reason_one, 14, "the unavailable prefix must carry reason 1");
    assert_eq!(
        provisional, 50,
        "the fixture must exercise provisional readiness"
    );
    assert_eq!(ready, 196, "the fixture must exercise ready readiness");
    assert_eq!(percentiles, 196, "the fixture must exercise the percentile");
    assert_eq!(
        acceleration_nulls, 17,
        "the fixture must exercise the three-bar window"
    );
    assert_eq!(negative, 111, "the fixture must exercise negative force");
    assert_eq!(neutral, 84, "the fixture must exercise neutral force");
    assert_eq!(positive, 51, "the fixture must exercise positive force");
    assert!(changes > 0, "the fixture must exercise state changes");

    assert!(rows[13].force.is_none(), "last bar before RSI history");
    assert!(rows[14].force.is_some(), "first valid force bar");
    assert!(
        rows[16].acceleration.is_none(),
        "the comparison window is not complete yet"
    );
    assert!(
        rows[17].acceleration.is_some(),
        "bar 17 has the force at bar 14"
    );
    assert_eq!(
        rows[63].readiness,
        Readiness::Provisional.code(),
        "49 prior force samples"
    );
    assert_eq!(
        rows[64].readiness,
        Readiness::Ready.code(),
        "50 prior force samples"
    );
}

#[test]
fn every_momentum_field_matches_the_golden_values() {
    let rows = load();
    let readings = Readings::compute(&candles(&rows));
    let actual = MomentumSeries::compute(&readings);

    assert_matches(actual, &rows);
}

#[test]
fn momentum_is_causal_across_every_prefix() {
    let rows = load();
    let candles = candles(&rows);
    let full = MomentumSeries::compute(&Readings::compute(&candles));

    for length in 0..=candles.len() {
        let prefix = MomentumSeries::compute(&Readings::compute(&candles[..length]));
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
fn changing_a_later_candle_does_not_change_earlier_momentum() {
    let rows = load();
    let candles = candles(&rows);
    let baseline = MomentumSeries::compute(&Readings::compute(&candles));

    let changed_candles = replace_candle(&candles, 250, 5.0);
    let changed = MomentumSeries::compute(&Readings::compute(&changed_candles));

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

    let first = MomentumSeries::compute(&readings);
    let second = MomentumSeries::compute(&readings);
    let third = MomentumSeries::compute(&Readings::compute(&candles));

    assert_eq!(first, second);
    assert_eq!(first, third);
}

#[test]
fn timestamps_do_not_change_momentum() {
    let rows = load();
    let candles = candles(&rows);
    let baseline = MomentumSeries::compute(&Readings::compute(&candles));

    let mut irregular = candles.clone();
    for (bar, candle) in irregular.iter_mut().enumerate() {
        // Deliberately uneven spacing: Momentum consumes bar order, not wall
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

    let changed = MomentumSeries::compute(&Readings::compute(&irregular));
    assert_eq!(baseline, changed);
}

#[test]
fn momentum_ignores_ema_atr_rvol_and_pivots() {
    let rows = load();
    let readings = Readings::compute(&candles(&rows));
    let baseline = MomentumSeries::compute(&readings);

    let mut changed = readings.clone();
    changed.ema20 = ReadingSeries::new(vec![Some(1.0); changed.len()]);
    changed.ema50 = ReadingSeries::new(vec![Some(2.0); changed.len()]);
    changed.ema200 = ReadingSeries::new(vec![Some(3.0); changed.len()]);
    changed.atr14 = ReadingSeries::new(vec![Some(4.0); changed.len()]);
    changed.rvol20 = ReadingSeries::new(vec![Some(5.0); changed.len()]);
    changed.pivots = PivotSeries::compute(&[
        Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(1.0)).expect("valid test candle")
    ]);

    assert_eq!(baseline, MomentumSeries::compute(&changed));
}

#[test]
fn every_published_value_obeys_the_contract_bounds() {
    let rows = load();
    let series = MomentumSeries::compute(&Readings::compute(&candles(&rows)));

    for bar in 0..series.len() {
        let record = series.get(bar).expect("a bar");
        match record.force {
            Some(force) => {
                assert!(
                    (-1.0..=1.0).contains(&force),
                    "bar {bar}: force {force} is outside [-1, +1]"
                );
                assert_eq!(
                    record.state_code,
                    compact_state(force),
                    "bar {bar}: state code"
                );
                assert!(matches!(
                    record.readiness,
                    Readiness::Provisional | Readiness::Ready
                ));
                assert_eq!(record.unavailable_reason, None, "bar {bar}");
                assert!(record.state_age_bars >= 1, "bar {bar}");

                if let Some(acceleration) = record.acceleration {
                    assert!(
                        (-1.0..=1.0).contains(&acceleration),
                        "bar {bar}: acceleration {acceleration} is outside [-1, +1]"
                    );
                }
                if let Some(intensification) = record.intensification {
                    assert!(
                        (-1.0..=1.0).contains(&intensification),
                        "bar {bar}: intensification {intensification} is outside [-1, +1]"
                    );
                }
                if let Some(percentile) = record.size_percentile {
                    assert!(
                        (0.0..=100.0).contains(&percentile),
                        "bar {bar}: percentile {percentile} is outside [0, 100]"
                    );
                }
                if record.size_percentile.is_none() {
                    assert_eq!(
                        record.readiness,
                        Readiness::Provisional,
                        "bar {bar}: too few percentile samples is provisional"
                    );
                }
                if record.readiness == Readiness::Ready {
                    assert!(record.size_percentile.is_some(), "bar {bar}");
                }
            }
            None => {
                assert_eq!(record.acceleration, None, "bar {bar}");
                assert_eq!(record.intensification, None, "bar {bar}");
                assert_eq!(record.state_code, None, "bar {bar}");
                assert_eq!(record.size_percentile, None, "bar {bar}");
                assert_eq!(record.readiness, Readiness::Unavailable, "bar {bar}");
                assert!(record.unavailable_reason.is_some(), "bar {bar}");
            }
        }
        assert_eq!(
            record.acceleration.is_some(),
            record.intensification.is_some(),
            "bar {bar}: both change measures share one window"
        );
        assert!(record.state_changes_50 <= 50, "bar {bar}");
    }
}

#[test]
fn an_empty_series_has_no_bars() {
    let series = MomentumSeries::compute(&Readings::compute(&[]));

    assert!(series.is_empty());
    assert_eq!(series.len(), 0);
    assert_eq!(series.get(0), None);
    assert_eq!(series.latest(), None);
}

#[test]
fn a_single_candle_is_unavailable() {
    let rows = load();
    let series = MomentumSeries::compute(&Readings::compute(&candles(&rows)[..1]));
    let bar = series.get(0).expect("one bar");

    assert_eq!(bar.force, None);
    assert_eq!(bar.acceleration, None);
    assert_eq!(bar.intensification, None);
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

    let short = MomentumSeries::compute(&Readings::compute(&candles[..14]));
    assert_eq!(short.len(), 14);
    assert!(short.bars().iter().all(|bar| bar.force.is_none()));

    let first = MomentumSeries::compute(&Readings::compute(&candles[..15]));
    assert_eq!(
        first.get(13).expect("a bar").force,
        None,
        "RSI 14 is not available on bar 13"
    );
    let first_force = first.get(14).expect("a bar");
    assert!(
        first_force.force.is_some(),
        "bar 14 is the first bar with RSI 14"
    );
    assert_eq!(
        first_force.acceleration, None,
        "bar 11 has no RSI for the comparison"
    );

    let window = MomentumSeries::compute(&Readings::compute(&candles[..18]));
    assert_eq!(
        window.get(16).expect("a bar").acceleration,
        None,
        "bar 13 has no RSI for the comparison"
    );
    assert!(
        window.get(17).expect("a bar").acceleration.is_some(),
        "bar 17 has the force at bar 14"
    );
}

#[test]
fn a_gap_produces_a_finite_force_after_the_required_history() {
    let rows = load();
    let candles = replace_candle(&candles(&rows), 100, 10.0);

    let series = MomentumSeries::compute(&Readings::compute(&candles));
    let bar = series.latest().expect("a latest bar");

    let Some(force) = bar.force else {
        panic!("the gapped series must publish a force");
    };
    assert!(force.is_finite());
    assert!((-1.0..=1.0).contains(&force));
    assert_eq!(bar.state_code, compact_state(force));
}

#[test]
fn zero_volume_does_not_change_momentum() {
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

    let with_zero = MomentumSeries::compute(&Readings::compute(&changed));
    let with_missing = MomentumSeries::compute(&Readings::compute(&candles(&rows)));

    assert_eq!(with_zero, with_missing);
}

#[test]
fn a_flat_series_with_equal_highs_and_lows_has_unavailable_momentum() {
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
    let series = MomentumSeries::compute(&Readings::compute(&candles));
    let bar = series.latest().expect("a latest bar");

    assert_eq!(bar.force, None);
    assert_eq!(bar.acceleration, None);
    assert_eq!(bar.intensification, None);
    assert_eq!(bar.state_code, None);
    assert_eq!(bar.size_percentile, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::InsufficientBars)
    );
    assert_eq!(bar.state_age_bars, 0);
    assert_eq!(bar.state_changes_50, 0);
}
