//! Golden-fixture and causality tests for the indicator readings.
//!
//! The fixture is a checked-in CSV of 260 candles and the expected value of
//! every reading on every bar. It was produced by an independent reference
//! implementation of the formulas in the model contract, not by the Rust code
//! under test. The test parses it, recomputes the readings, and compares.
//!
//! The fixture has one row per bar and one column group per reading. An empty
//! cell is an unavailable measurement; a numeric cell is a measured value, and
//! a measured zero is written as `0.0`.

use quantrule_data::Candle;
use quantrule_model::readings::{Pivot, PivotKind, PivotScale, ReadingSeries, Readings};

const FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const TOLERANCE: f64 = 1e-9;

const HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

#[derive(Debug, Clone, Copy)]
struct ExpectedCandidate {
    kind: PivotKind,
    price: f64,
}

#[derive(Debug, Clone, Copy)]
struct ExpectedPivot {
    kind: PivotKind,
    price: f64,
    occurred_at: usize,
}

struct GoldenRow {
    bar: usize,
    candle: Candle,
    ema20: Option<f64>,
    ema50: Option<f64>,
    ema200: Option<f64>,
    rsi14: Option<f64>,
    atr14: Option<f64>,
    rvol20: Option<f64>,
    minor_candidate: Option<ExpectedCandidate>,
    minor_pivot: Option<ExpectedPivot>,
    major_candidate: Option<ExpectedCandidate>,
    major_pivot: Option<ExpectedPivot>,
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

fn index(field: &str) -> Option<usize> {
    if field.is_empty() {
        None
    } else {
        Some(
            field
                .parse::<usize>()
                .unwrap_or_else(|error| panic!("invalid index {field:?}: {error}")),
        )
    }
}

fn candidate(high: Option<f64>, low: Option<f64>, bar: usize) -> Option<ExpectedCandidate> {
    match (high, low) {
        (Some(price), None) => Some(ExpectedCandidate {
            kind: PivotKind::High,
            price,
        }),
        (None, Some(price)) => Some(ExpectedCandidate {
            kind: PivotKind::Low,
            price,
        }),
        (None, None) => None,
        (Some(_), Some(_)) => panic!("bar {bar} is both a candidate high and low"),
    }
}

fn pivot(
    high: Option<f64>,
    low: Option<f64>,
    occurred_at: Option<usize>,
    bar: usize,
) -> Option<ExpectedPivot> {
    match (high, low, occurred_at) {
        (Some(price), None, Some(occurred_at)) => Some(ExpectedPivot {
            kind: PivotKind::High,
            price,
            occurred_at,
        }),
        (None, Some(price), Some(occurred_at)) => Some(ExpectedPivot {
            kind: PivotKind::Low,
            price,
            occurred_at,
        }),
        (None, None, None) => None,
        (None, None, Some(_)) => panic!("bar {bar} has a pivot index without a pivot"),
        (Some(_), Some(_), _) => panic!("bar {bar} has two pivots"),
        _ => panic!("bar {bar} has a pivot without its occurrence bar"),
    }
}

fn load() -> Vec<GoldenRow> {
    let mut lines = FIXTURE.lines();
    assert_eq!(lines.next(), Some(HEADER), "fixture header changed");

    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23, "wrong column count in {line:?}");

        let bar = fields[0].parse::<usize>().expect("bar index");
        assert_eq!(bar, rows.len(), "fixture bars must be contiguous");

        let candle = Candle::new(
            bar as u64,
            fields[1].parse::<i64>().expect("timestamp"),
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
            ema20: number(fields[7]),
            ema50: number(fields[8]),
            ema200: number(fields[9]),
            rsi14: number(fields[10]),
            atr14: number(fields[11]),
            rvol20: number(fields[12]),
            minor_candidate: candidate(number(fields[13]), number(fields[14]), bar),
            minor_pivot: pivot(
                number(fields[15]),
                number(fields[16]),
                index(fields[17]),
                bar,
            ),
            major_candidate: candidate(number(fields[18]), number(fields[19]), bar),
            major_pivot: pivot(
                number(fields[20]),
                number(fields[21]),
                index(fields[22]),
                bar,
            ),
        });
    }
    rows
}

fn candles(rows: &[GoldenRow]) -> Vec<Candle> {
    rows.iter().map(|row| row.candle).collect()
}

/// Rebuilds the series with one later candle moved and its volume increased.
fn replace_candle(candles: &[Candle], bar: usize, price_move: f64) -> Vec<Candle> {
    let mut changed = candles.to_vec();
    let old = changed[bar];
    let close = old.close() + price_move;
    let high = old.high().max(close) + price_move;
    let low = old.low().min(close) - price_move;
    let volume = old.volume().map(|volume| volume + 1_000.0);
    changed[bar] = Candle::new(
        old.series_index(),
        old.timestamp(),
        old.open(),
        high,
        low,
        close,
        volume,
    )
    .expect("the changed candle is valid");
    changed
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

fn first_available(series: &ReadingSeries) -> Option<usize> {
    series.values().iter().position(Option::is_some)
}

fn expected_candidate(
    actual: Option<quantrule_model::readings::PivotCandidate>,
    label: &str,
) -> Option<ExpectedCandidate> {
    actual.map(|candidate| {
        assert_eq!(candidate.occurred_at, candidate.available_at, "{label}");
        ExpectedCandidate {
            kind: candidate.kind,
            price: candidate.price,
        }
    })
}

fn expected_pivot(actual: Option<Pivot>, scale: PivotScale, bar: usize) -> Option<ExpectedPivot> {
    actual.map(|pivot| {
        assert_eq!(pivot.scale, scale);
        assert_eq!(
            pivot.available_at, bar,
            "pivot stored at {bar} must be knowable there"
        );
        ExpectedPivot {
            kind: pivot.kind,
            price: pivot.price,
            occurred_at: pivot.occurred_at,
        }
    })
}

#[test]
fn the_fixture_covers_every_reading_and_both_pivot_scales() {
    let rows = load();
    assert_eq!(rows.len(), 260);

    let candles = candles(&rows);
    let readings = Readings::compute(&candles);
    assert_eq!(readings.len(), rows.len());

    assert_eq!(readings.pivots.candidate_count(PivotScale::Minor), 159);
    assert_eq!(readings.pivots.confirmed_count(PivotScale::Minor), 44);
    assert_eq!(readings.pivots.candidate_count(PivotScale::Major), 127);
    assert_eq!(readings.pivots.confirmed_count(PivotScale::Major), 24);

    for scale in PivotScale::ALL {
        assert!(
            readings.pivots.confirmed_count(scale) > 0,
            "the fixture must exercise {scale:?} pivots"
        );
    }
}

#[test]
fn every_scalar_reading_matches_the_golden_values() {
    let rows = load();
    let candles = candles(&rows);
    let readings = Readings::compute(&candles);

    for row in &rows {
        let bar = row.bar;
        assert_close(
            readings.ema20.get(bar),
            row.ema20,
            &format!("ema20 at {bar}"),
        );
        assert_close(
            readings.ema50.get(bar),
            row.ema50,
            &format!("ema50 at {bar}"),
        );
        assert_close(
            readings.ema200.get(bar),
            row.ema200,
            &format!("ema200 at {bar}"),
        );
        assert_close(
            readings.rsi14.get(bar),
            row.rsi14,
            &format!("rsi14 at {bar}"),
        );
        assert_close(
            readings.atr14.get(bar),
            row.atr14,
            &format!("atr14 at {bar}"),
        );
        assert_close(
            readings.rvol20.get(bar),
            row.rvol20,
            &format!("rvol20 at {bar}"),
        );
    }
}

#[test]
fn every_candidate_matches_the_golden_values() {
    let rows = load();
    let candles = candles(&rows);
    let readings = Readings::compute(&candles);

    for row in &rows {
        for (scale, column, expected) in [
            (PivotScale::Minor, "minor", row.minor_candidate),
            (PivotScale::Major, "major", row.major_candidate),
        ] {
            let actual = expected_candidate(
                readings.pivots.candidate(scale, row.bar),
                &format!("{column} candidate at {}", row.bar),
            );
            match (actual, expected) {
                (None, None) => {}
                (Some(actual), Some(expected)) => {
                    assert_eq!(actual.kind, expected.kind, "{column} kind at {}", row.bar);
                    assert!(
                        (actual.price - expected.price).abs() <= TOLERANCE,
                        "{column} price at {}: expected {}, got {}",
                        row.bar,
                        expected.price,
                        actual.price
                    );
                }
                _ => panic!(
                    "{column} candidate at {}: expected {expected:?}, got {actual:?}",
                    row.bar
                ),
            }
        }
    }
}

#[test]
fn every_confirmed_pivot_matches_the_golden_values() {
    let rows = load();
    let candles = candles(&rows);
    let readings = Readings::compute(&candles);

    let mut minor_pivots = 0;
    let mut major_pivots = 0;
    for row in &rows {
        for (scale, column, expected, count) in [
            (
                PivotScale::Minor,
                "minor",
                row.minor_pivot,
                &mut minor_pivots,
            ),
            (
                PivotScale::Major,
                "major",
                row.major_pivot,
                &mut major_pivots,
            ),
        ] {
            let actual =
                expected_pivot(readings.pivots.confirmed_at(scale, row.bar), scale, row.bar);
            match (actual, expected) {
                (None, None) => {}
                (Some(actual), Some(expected)) => {
                    *count += 1;
                    assert_eq!(actual.kind, expected.kind, "{column} kind at {}", row.bar);
                    assert_eq!(
                        actual.occurred_at, expected.occurred_at,
                        "{column} occurrence at {}",
                        row.bar
                    );
                    assert_eq!(
                        row.bar,
                        expected.occurred_at + scale.lookback(),
                        "{column} availability at {}",
                        row.bar
                    );
                    assert!(
                        (actual.price - expected.price).abs() <= TOLERANCE,
                        "{column} price at {}: expected {}, got {}",
                        row.bar,
                        expected.price,
                        actual.price
                    );
                }
                _ => panic!(
                    "{column} pivot at {}: expected {expected:?}, got {actual:?}",
                    row.bar
                ),
            }
        }
    }

    assert_eq!(minor_pivots, 44);
    assert_eq!(major_pivots, 24);
}

#[test]
fn the_first_available_bar_matches_the_required_history() {
    let rows = load();
    let candles = candles(&rows);
    let readings = Readings::compute(&candles);

    assert_eq!(first_available(&readings.ema20), Some(19));
    assert_eq!(first_available(&readings.ema50), Some(49));
    assert_eq!(first_available(&readings.ema200), Some(199));
    assert_eq!(first_available(&readings.rsi14), Some(14));
    assert_eq!(first_available(&readings.atr14), Some(14));
    assert_eq!(first_available(&readings.rvol20), Some(19));

    for (label, series) in [
        ("ema20", &readings.ema20),
        ("ema50", &readings.ema50),
        ("ema200", &readings.ema200),
        ("rsi14", &readings.rsi14),
        ("atr14", &readings.atr14),
        ("rvol20", &readings.rvol20),
    ] {
        let first = first_available(series).expect("the fixture reaches the required history");
        assert!(
            series.values()[first..].iter().all(Option::is_some),
            "{label} has an unexpected hole after bar {first}"
        );
    }
}

#[test]
fn confirmed_pivots_agree_with_their_candidates() {
    let rows = load();
    let candles = candles(&rows);
    let readings = Readings::compute(&candles);

    for scale in PivotScale::ALL {
        for pivot in readings.pivots.all_confirmed(scale) {
            let candidate = readings
                .pivots
                .candidate(scale, pivot.occurred_at)
                .expect("every confirmed pivot came from a candidate");
            assert_eq!(candidate.kind, pivot.kind);
            assert_eq!(candidate.price, pivot.price);
            assert_eq!(candidate.occurred_at, pivot.occurred_at);
            assert_eq!(pivot.available_at, pivot.occurred_at + scale.lookback());

            let candle = candles[pivot.occurred_at];
            let expected_price = match pivot.kind {
                PivotKind::High => candle.high(),
                PivotKind::Low => candle.low(),
            };
            assert_eq!(
                pivot.price, expected_price,
                "{scale:?} pivot at {} must use the candle extreme",
                pivot.occurred_at
            );
        }
    }
}

#[test]
fn no_pivot_is_visible_before_its_availability_bar() {
    let rows = load();
    let candles = candles(&rows);
    let readings = Readings::compute(&candles);

    for bar in 0..rows.len() {
        for scale in PivotScale::ALL {
            let visible: Vec<Pivot> = readings
                .pivots
                .confirmed_as_of(scale, bar)
                .copied()
                .collect();
            for pivot in &visible {
                assert!(
                    pivot.available_at <= bar,
                    "{scale:?} pivot became knowable at {} but was visible at {bar}",
                    pivot.available_at
                );
            }
            let expected = readings
                .pivots
                .all_confirmed(scale)
                .filter(|pivot| pivot.available_at <= bar)
                .count();
            assert_eq!(visible.len(), expected);
        }
    }
}

#[test]
fn appending_bars_does_not_change_earlier_readings() {
    let rows = load();
    let candles = candles(&rows);
    let full = Readings::compute(&candles);

    for length in 0..=candles.len() {
        let prefix = Readings::compute(&candles[..length]);
        for bar in 0..length {
            assert_eq!(
                prefix.ema20.get(bar),
                full.ema20.get(bar),
                "ema20 at {bar}/{length}"
            );
            assert_eq!(
                prefix.ema50.get(bar),
                full.ema50.get(bar),
                "ema50 at {bar}/{length}"
            );
            assert_eq!(
                prefix.ema200.get(bar),
                full.ema200.get(bar),
                "ema200 at {bar}/{length}"
            );
            assert_eq!(
                prefix.rsi14.get(bar),
                full.rsi14.get(bar),
                "rsi14 at {bar}/{length}"
            );
            assert_eq!(
                prefix.atr14.get(bar),
                full.atr14.get(bar),
                "atr14 at {bar}/{length}"
            );
            assert_eq!(
                prefix.rvol20.get(bar),
                full.rvol20.get(bar),
                "rvol20 at {bar}/{length}"
            );
            for scale in PivotScale::ALL {
                assert_eq!(
                    prefix.pivots.candidate(scale, bar),
                    full.pivots.candidate(scale, bar),
                    "{scale:?} candidate at {bar}/{length}"
                );
                assert_eq!(
                    prefix.pivots.confirmed_at(scale, bar),
                    full.pivots.confirmed_at(scale, bar),
                    "{scale:?} pivot at {bar}/{length}"
                );
            }
        }
    }
}

#[test]
fn changing_a_later_bar_does_not_change_earlier_readings() {
    let rows = load();
    let candles = candles(&rows);
    let baseline = Readings::compute(&candles);

    for changed_bar in [50_usize, 100, 150, 200, 259] {
        let changed = replace_candle(&candles, changed_bar, 3.0);
        let readings = Readings::compute(&changed);

        for bar in 0..changed_bar {
            assert_eq!(
                readings.ema20.get(bar),
                baseline.ema20.get(bar),
                "ema20 at {bar} changed when bar {changed_bar} moved"
            );
            assert_eq!(
                readings.ema50.get(bar),
                baseline.ema50.get(bar),
                "ema50 at {bar} changed when bar {changed_bar} moved"
            );
            assert_eq!(
                readings.ema200.get(bar),
                baseline.ema200.get(bar),
                "ema200 at {bar} changed when bar {changed_bar} moved"
            );
            assert_eq!(
                readings.rsi14.get(bar),
                baseline.rsi14.get(bar),
                "rsi14 at {bar} changed when bar {changed_bar} moved"
            );
            assert_eq!(
                readings.atr14.get(bar),
                baseline.atr14.get(bar),
                "atr14 at {bar} changed when bar {changed_bar} moved"
            );
            assert_eq!(
                readings.rvol20.get(bar),
                baseline.rvol20.get(bar),
                "rvol20 at {bar} changed when bar {changed_bar} moved"
            );
            for scale in PivotScale::ALL {
                assert_eq!(
                    readings.pivots.candidate(scale, bar),
                    baseline.pivots.candidate(scale, bar),
                    "{scale:?} candidate at {bar} changed when bar {changed_bar} moved"
                );
                assert_eq!(
                    readings.pivots.confirmed_at(scale, bar),
                    baseline.pivots.confirmed_at(scale, bar),
                    "{scale:?} pivot at {bar} changed when bar {changed_bar} moved"
                );
            }
        }
    }
}

#[test]
fn repeated_runs_are_identical() {
    let rows = load();
    let candles = candles(&rows);

    let first = Readings::compute(&candles);
    let second = Readings::compute(&candles);
    assert_eq!(first, second);

    let mut offset = vec![candles[0]];
    offset.extend_from_slice(&candles);
    let third = Readings::compute(&offset[1..]);
    assert_eq!(first, third);
}

#[test]
fn the_empty_and_single_candle_series_are_handled() {
    let empty = Readings::compute(&[]);
    assert!(empty.is_empty());
    assert_eq!(empty.ema20.get(0), None);
    assert_eq!(empty.pivots.len(), 0);

    let rows = load();
    let single = Readings::compute(&candles(&rows)[..1]);
    assert_eq!(single.len(), 1);
    for series in [
        &single.ema20,
        &single.ema50,
        &single.ema200,
        &single.rsi14,
        &single.atr14,
        &single.rvol20,
    ] {
        assert_eq!(series.get(0), None);
    }
    assert_eq!(single.pivots.confirmed_count(PivotScale::Minor), 0);
    assert_eq!(single.pivots.confirmed_count(PivotScale::Major), 0);
}
