//! Golden-fixture, causality, determinism, and edge-case tests for Structure.
//!
//! The per-bar fixture is a checked-in CSV of 260 candles and the expected
//! Structure values on every bar. Its expected values were produced by an
//! independent Python implementation of contract §8.3 and §9 from the
//! independently generated EMA, ATR, and pivot columns of the readings
//! fixture, not by the Rust code under test.
//!
//! The registry fixture is generated the same way from both pivot scales. It
//! records the confirmation order, the reference codes, and the occurrence and
//! availability bars.
//!
//! An empty cell is an unavailable measurement; a numeric cell is a measured
//! value, and a measured zero is written as `0.0`.

use quantrule_data::Candle;
use quantrule_model::axes::structure::StructureBar;
use quantrule_model::axes::structure::StructureSeries;
use quantrule_model::axes::structure::registry::ConfirmedLevel;
use quantrule_model::kernel::{Readiness, UnavailableReason};
use quantrule_model::readings::{PivotKind, PivotScale, Readings};

const FIXTURE: &str = include_str!("fixtures/structure_golden.csv");
const LEVELS_FIXTURE: &str = include_str!("fixtures/structure_levels_golden.csv");
const TOLERANCE: f64 = 1e-9;

const HEADER: &str = "bar,timestamp,open,high,low,close,volume,high_relation_code,low_relation_code,structure_score,structure_code,structure_scale_code,ema_distance_atr_20,ema_cross_code_20,ema_retest_code_20,ema_distance_atr_50,ema_cross_code_50,ema_retest_code_50,ema_distance_atr_200,ema_cross_code_200,ema_retest_code_200,structure_size_percentile,structure_readiness_code,structure_unavailable_reason_code,structure_state_age_bars,structure_state_changes_50";
const LEVELS_HEADER: &str =
    "sequence,reference_code,scale_code,kind,price,occurred_at,available_at";

#[derive(Debug, Clone, Copy)]
struct GoldenRow {
    bar: usize,
    candle: Candle,
    high_relation: Option<i8>,
    low_relation: Option<i8>,
    score: Option<f64>,
    state_code: Option<i8>,
    scale_code: u8,
    distance: [Option<f64>; 3],
    cross: [Option<i8>; 3],
    retest: [Option<i8>; 3],
    size_percentile: Option<f64>,
    readiness: u8,
    unavailable_reason: Option<u8>,
    state_age_bars: u64,
    state_changes_50: u64,
}

#[derive(Debug, Clone, Copy)]
struct GoldenLevel {
    sequence: usize,
    reference_code: u32,
    scale_code: u8,
    kind: PivotKind,
    price: f64,
    occurred_at: usize,
    available_at: usize,
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
        assert_eq!(fields.len(), 26, "wrong column count in {line:?}");

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
            high_relation: optional_integer::<i8>(fields[7]),
            low_relation: optional_integer::<i8>(fields[8]),
            score: number(fields[9]),
            state_code: optional_integer::<i8>(fields[10]),
            scale_code: integer::<u8>(fields[11]),
            distance: [number(fields[12]), number(fields[15]), number(fields[18])],
            cross: [
                optional_integer::<i8>(fields[13]),
                optional_integer::<i8>(fields[16]),
                optional_integer::<i8>(fields[19]),
            ],
            retest: [
                optional_integer::<i8>(fields[14]),
                optional_integer::<i8>(fields[17]),
                optional_integer::<i8>(fields[20]),
            ],
            size_percentile: number(fields[21]),
            readiness: integer::<u8>(fields[22]),
            unavailable_reason: optional_integer::<u8>(fields[23]),
            state_age_bars: integer::<u64>(fields[24]),
            state_changes_50: integer::<u64>(fields[25]),
        });
    }
    rows
}

fn load_levels() -> Vec<GoldenLevel> {
    let mut lines = LEVELS_FIXTURE.lines();
    assert_eq!(
        lines.next(),
        Some(LEVELS_HEADER),
        "levels fixture header changed"
    );

    let mut levels = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 7, "wrong column count in {line:?}");

        let sequence = integer::<usize>(fields[0]);
        assert_eq!(
            sequence,
            levels.len() + 1,
            "level sequences must be contiguous and 1-based"
        );
        let scale_code = integer::<u8>(fields[2]);
        assert!(
            matches!(scale_code, 1 | 2),
            "unknown scale code {scale_code}"
        );
        let kind = match fields[3] {
            "high" => PivotKind::High,
            "low" => PivotKind::Low,
            other => panic!("unknown pivot kind {other:?}"),
        };

        levels.push(GoldenLevel {
            sequence,
            reference_code: integer::<u32>(fields[1]),
            scale_code,
            kind,
            price: number(fields[4]).expect("level price"),
            occurred_at: integer::<usize>(fields[5]),
            available_at: integer::<usize>(fields[6]),
        });
    }
    levels
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

fn assert_matches(actual: &StructureSeries, rows: &[GoldenRow]) {
    assert_eq!(actual.len(), rows.len(), "series length");

    for row in rows {
        let bar = actual.get(row.bar).expect("a bar for every row");
        let label = |field: &str| format!("{field} {}", row.bar);

        assert_eq!(
            bar.high_relation_code,
            row.high_relation,
            "{}",
            label("high relation")
        );
        assert_eq!(
            bar.low_relation_code,
            row.low_relation,
            "{}",
            label("low relation")
        );
        assert_close(bar.score, row.score, &label("score"));
        assert_eq!(bar.state_code, row.state_code, "{}", label("state code"));
        assert_eq!(bar.scale_code, row.scale_code, "{}", label("scale code"));

        for (index, period) in [20, 50, 200].into_iter().enumerate() {
            assert_close(
                bar_ema_distance(bar, index),
                row.distance[index],
                &label(&format!("EMA {period} distance")),
            );
            assert_eq!(
                bar_ema_cross(bar, index),
                row.cross[index],
                "{}",
                label(&format!("EMA {period} cross"))
            );
            assert_eq!(
                bar_ema_retest(bar, index),
                row.retest[index],
                "{}",
                label(&format!("EMA {period} retest"))
            );
        }

        assert_close(
            bar.size_percentile,
            row.size_percentile,
            &label("size percentile"),
        );
        assert_eq!(
            bar.readiness.code(),
            row.readiness,
            "{}",
            label("readiness")
        );
        assert_eq!(
            bar.unavailable_reason.map(UnavailableReason::code),
            row.unavailable_reason,
            "{}",
            label("unavailable reason")
        );
        assert_eq!(
            bar.state_age_bars,
            row.state_age_bars,
            "{}",
            label("state age")
        );
        assert_eq!(
            bar.state_changes_50,
            row.state_changes_50,
            "{}",
            label("state changes")
        );
    }
}

fn bar_ema_distance(bar: StructureBar, index: usize) -> Option<f64> {
    [
        bar.ema_distance_atr_20,
        bar.ema_distance_atr_50,
        bar.ema_distance_atr_200,
    ][index]
}

fn bar_ema_cross(bar: StructureBar, index: usize) -> Option<i8> {
    [
        bar.ema_cross_code_20,
        bar.ema_cross_code_50,
        bar.ema_cross_code_200,
    ][index]
}

fn bar_ema_retest(bar: StructureBar, index: usize) -> Option<i8> {
    [
        bar.ema_retest_code_20,
        bar.ema_retest_code_50,
        bar.ema_retest_code_200,
    ][index]
}

fn assert_registry_matches(actual: &StructureSeries, levels: &[GoldenLevel]) {
    assert_eq!(actual.registry().len(), levels.len(), "registry length");

    for expected in levels {
        let level = actual
            .registry()
            .get(expected.sequence - 1)
            .expect("a level for every fixture row");
        assert_eq!(
            level.reference_code, expected.reference_code,
            "level {} reference code",
            expected.sequence
        );
        assert_eq!(
            level.scale.code(),
            expected.scale_code,
            "level {} scale",
            expected.sequence
        );
        assert_eq!(
            level.kind, expected.kind,
            "level {} kind",
            expected.sequence
        );
        assert!(
            (level.price - expected.price).abs() <= TOLERANCE,
            "level {} price: expected {}, got {}",
            expected.sequence,
            expected.price,
            level.price
        );
        assert_eq!(
            level.occurred_at, expected.occurred_at,
            "level {} occurrence",
            expected.sequence
        );
        assert_eq!(
            level.available_at, expected.available_at,
            "level {} availability",
            expected.sequence
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
fn the_fixture_covers_the_structure_lifecycle() {
    let rows = load();
    let levels = load_levels();
    assert_eq!(rows.len(), 260);

    let nulls = rows.iter().filter(|row| row.score.is_none()).count();
    let valid = rows.iter().filter(|row| row.score.is_some()).count();
    let reason_two = rows
        .iter()
        .filter(|row| {
            row.unavailable_reason == Some(UnavailableReason::InsufficientConfirmedPivots.code())
        })
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
    let one_relation_only = rows
        .iter()
        .filter(|row| row.high_relation.is_some() != row.low_relation.is_some())
        .count();

    assert_eq!(nulls, 34, "the fixture must exercise the incomplete prefix");
    assert_eq!(valid, 226, "the fixture must exercise a valid score");
    assert_eq!(
        reason_two, 34,
        "an incomplete pair must report insufficient confirmed pivots"
    );
    assert_eq!(
        provisional, 50,
        "the fixture must exercise provisional readiness"
    );
    assert_eq!(ready, 176, "the fixture must exercise ready readiness");
    assert_eq!(percentiles, 176, "the fixture must exercise the percentile");
    assert_eq!(
        one_relation_only, 5,
        "the fixture must retain an individually available relation"
    );

    let mut score_values: Vec<f64> = rows.iter().filter_map(|row| row.score).collect();
    score_values.sort_by(f64::total_cmp);
    score_values.dedup();
    assert_eq!(score_values, [-1.0, 0.0, 1.0]);

    let mut code_values: Vec<i8> = rows.iter().filter_map(|row| row.state_code).collect();
    code_values.sort_unstable();
    code_values.dedup();
    assert_eq!(code_values, [-2, 0, 2]);

    // The fixture does not reach the ±1 codes or an equal relation; the unit
    // tests in the module cover those paths.
    assert!(rows.iter().all(|row| row.high_relation != Some(0)));
    assert!(rows.iter().all(|row| row.low_relation != Some(0)));

    let code_transitions = rows
        .windows(2)
        .filter(|pair| {
            pair[0].state_code.is_some()
                && pair[1].state_code.is_some()
                && pair[0].state_code != pair[1].state_code
        })
        .count();
    assert_eq!(
        code_transitions, 13,
        "the fixture must exercise code changes"
    );

    assert_eq!(levels.len(), 68);
    assert_eq!(
        levels
            .iter()
            .filter(|level| level.scale_code == PivotScale::Minor.code())
            .count(),
        44
    );
    assert_eq!(
        levels
            .iter()
            .filter(|level| level.scale_code == PivotScale::Major.code())
            .count(),
        24
    );
    assert_eq!(
        levels
            .iter()
            .filter(|level| level.scale_code == PivotScale::Major.code()
                && level.kind == PivotKind::High)
            .count(),
        12
    );
    assert_eq!(
        levels
            .iter()
            .filter(|level| level.scale_code == PivotScale::Major.code()
                && level.kind == PivotKind::Low)
            .count(),
        12
    );
    assert_eq!(levels[0].reference_code, 1001);
    assert_eq!(levels[67].reference_code, 1068);
}

#[test]
fn every_structure_field_matches_the_golden_values() {
    let rows = load();
    let readings = Readings::compute(&candles(&rows));
    let actual = StructureSeries::compute(&candles(&rows), &readings);

    assert_matches(&actual, &rows);
    assert_registry_matches(&actual, &load_levels());
}

#[test]
fn structure_is_causal_across_every_prefix() {
    let rows = load();
    let candles = candles(&rows);
    let full = StructureSeries::compute(&candles, &Readings::compute(&candles));

    for length in 0..=candles.len() {
        let prefix_candles = &candles[..length];
        let prefix = StructureSeries::compute(prefix_candles, &Readings::compute(prefix_candles));
        assert_eq!(prefix.len(), length, "prefix length {length}");

        for bar in 0..length {
            assert_eq!(
                prefix.get(bar),
                full.get(bar),
                "prefix {length}, bar {bar} changed"
            );
        }

        let expected: Vec<ConfirmedLevel> = if length == 0 {
            Vec::new()
        } else {
            full.registry().as_of(length - 1).copied().collect()
        };
        assert_eq!(
            prefix.registry().levels(),
            expected.as_slice(),
            "prefix {length} registry changed"
        );
    }
}

#[test]
fn changing_a_later_candle_does_not_change_earlier_structure() {
    let rows = load();
    let candles = candles(&rows);
    let baseline = StructureSeries::compute(&candles, &Readings::compute(&candles));

    let changed_candles = replace_candle(&candles, 250, 5.0);
    let changed = StructureSeries::compute(&changed_candles, &Readings::compute(&changed_candles));

    for bar in 0..250 {
        assert_eq!(
            baseline.get(bar),
            changed.get(bar),
            "bar {bar} changed after a later candle moved"
        );
    }
    assert_eq!(
        baseline.registry().as_of(249).collect::<Vec<_>>(),
        changed.registry().as_of(249).collect::<Vec<_>>(),
        "the earlier registry view changed after a later candle moved"
    );
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

    let first = StructureSeries::compute(&candles, &readings);
    let second = StructureSeries::compute(&candles, &readings);
    let third = StructureSeries::compute(&candles, &Readings::compute(&candles));

    assert_eq!(first, second);
    assert_eq!(first, third);
}

#[test]
fn timestamps_do_not_change_structure() {
    let rows = load();
    let candles = candles(&rows);
    let baseline = StructureSeries::compute(&candles, &Readings::compute(&candles));

    let mut irregular = candles.clone();
    for (bar, candle) in irregular.iter_mut().enumerate() {
        // Structure consumes bar order, not wall-clock time; the data engine
        // owns timeframe alignment.
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

    let changed = StructureSeries::compute(&irregular, &Readings::compute(&irregular));
    assert_eq!(baseline, changed);
}

#[test]
fn every_published_value_obeys_the_contract_bounds() {
    let rows = load();
    let candles = candles(&rows);
    let series = StructureSeries::compute(&candles, &Readings::compute(&candles));

    for bar in 0..series.len() {
        let record = series.get(bar).expect("a bar");
        assert!(matches!(record.scale_code, 1 | 2), "bar {bar}");
        assert!(
            matches!(record.high_relation_code, Some(-1..=1) | None),
            "bar {bar}: high relation"
        );
        assert!(
            matches!(record.low_relation_code, Some(-1..=1) | None),
            "bar {bar}: low relation"
        );

        match record.score {
            Some(score) => {
                assert!(
                    (-1.0..=1.0).contains(&score),
                    "bar {bar}: score {score} is outside [-1, +1]"
                );
                assert!(
                    record.high_relation_code.is_some() && record.low_relation_code.is_some(),
                    "bar {bar}: a score requires both relations"
                );
                let expected_code = (2.0 * score).round() as i8;
                assert_eq!(record.state_code, Some(expected_code), "bar {bar}");
                assert!((-2..=2).contains(&expected_code), "bar {bar}");
                assert_eq!(record.unavailable_reason, None, "bar {bar}");
                assert!(matches!(
                    record.readiness,
                    Readiness::Provisional | Readiness::Ready
                ));
                assert!(record.state_age_bars >= 1, "bar {bar}");
                if let Some(percentile) = record.size_percentile {
                    assert!(
                        (0.0..=100.0).contains(&percentile),
                        "bar {bar}: percentile outside [0, 100]"
                    );
                    assert_eq!(record.readiness, Readiness::Ready, "bar {bar}");
                } else {
                    assert_eq!(
                        record.readiness,
                        Readiness::Provisional,
                        "bar {bar}: too few percentile samples is provisional"
                    );
                }
            }
            None => {
                assert_eq!(record.state_code, None, "bar {bar}");
                assert_eq!(record.size_percentile, None, "bar {bar}");
                assert_eq!(record.readiness, Readiness::Unavailable, "bar {bar}");
                assert_eq!(
                    record.unavailable_reason,
                    Some(UnavailableReason::InsufficientConfirmedPivots),
                    "bar {bar}"
                );
                assert!(
                    record.high_relation_code.is_none() || record.low_relation_code.is_none(),
                    "bar {bar}: a score is null only on an incomplete pair"
                );
            }
        }

        for (label, distance, cross, retest) in [
            (
                "EMA 20",
                record.ema_distance_atr_20,
                record.ema_cross_code_20,
                record.ema_retest_code_20,
            ),
            (
                "EMA 50",
                record.ema_distance_atr_50,
                record.ema_cross_code_50,
                record.ema_retest_code_50,
            ),
            (
                "EMA 200",
                record.ema_distance_atr_200,
                record.ema_cross_code_200,
                record.ema_retest_code_200,
            ),
        ] {
            if let Some(distance) = distance {
                assert!(distance.is_finite(), "bar {bar}: {label} distance");
            }
            assert!(
                matches!(cross, Some(-1..=1) | None),
                "bar {bar}: {label} cross"
            );
            assert!(
                matches!(retest, Some(-1..=1) | None),
                "bar {bar}: {label} retest"
            );
        }

        assert!(record.state_changes_50 <= 50, "bar {bar}");
    }

    for (position, level) in series.registry().levels().iter().enumerate() {
        assert!(
            level.available_at >= level.occurred_at,
            "a level cannot be knowable before it occurred"
        );
        assert_eq!(
            level.available_at,
            level.occurred_at + level.scale.lookback(),
            "a level is knowable exactly at occurrence plus the scale lookback"
        );
        assert_eq!(
            level.reference_code,
            1001 + position as u32,
            "reference codes are contiguous from 1001"
        );
    }
}

#[test]
fn an_empty_series_has_no_bars() {
    let series = StructureSeries::compute(&[], &Readings::compute(&[]));

    assert!(series.is_empty());
    assert_eq!(series.len(), 0);
    assert_eq!(series.get(0), None);
    assert_eq!(series.latest(), None);
    assert!(series.registry().is_empty());
}

#[test]
fn a_single_candle_is_unavailable() {
    let rows = load();
    let candles = candles(&rows);
    let series = StructureSeries::compute(&candles[..1], &Readings::compute(&candles[..1]));
    let bar = series.get(0).expect("one bar");

    assert_eq!(bar.score, None);
    assert_eq!(bar.state_code, None);
    assert_eq!(bar.scale_code, 2);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::InsufficientConfirmedPivots)
    );
    assert_eq!(bar.ema_distance_atr_20, None);
    assert_eq!(bar.ema_cross_code_20, None);
    assert_eq!(bar.ema_retest_code_20, None);
    assert_eq!(bar.state_age_bars, 0);
    assert_eq!(bar.state_changes_50, 0);
}

#[test]
fn a_series_shorter_than_the_required_pivot_history_is_unavailable() {
    let rows = load();
    let candles = candles(&rows);

    let short = StructureSeries::compute(&candles[..20], &Readings::compute(&candles[..20]));
    assert_eq!(short.len(), 20);
    assert!(
        short.bars().iter().all(|bar| bar.score.is_none()),
        "no complete pair exists in the first 20 bars"
    );
    assert!(short.bars().iter().all(|bar| {
        bar.readiness == Readiness::Unavailable
            && bar.unavailable_reason == Some(UnavailableReason::InsufficientConfirmedPivots)
    }));
}

#[test]
fn a_gap_keeps_structure_well_formed() {
    let rows = load();
    let candles = replace_candle(&candles(&rows), 100, 10.0);

    let series = StructureSeries::compute(&candles, &Readings::compute(&candles));
    let bar = series.latest().expect("a latest bar");

    let Some(score) = bar.score else {
        panic!("the gapped series must publish a score");
    };
    assert!(score.is_finite());
    assert!((-1.0..=1.0).contains(&score));
    assert_eq!(bar.state_code, Some((2.0 * score).round() as i8));
}

#[test]
fn zero_volume_does_not_change_structure() {
    let rows = load();
    let mut changed = candles(&rows);
    let original = changed[150];
    changed[150] = Candle::new(
        original.series_index(),
        original.timestamp(),
        original.open(),
        original.high(),
        original.low(),
        original.close(),
        Some(0.0),
    )
    .expect("the zero-volume candle is valid");

    let with_zero = StructureSeries::compute(&changed, &Readings::compute(&changed));
    let baseline = StructureSeries::compute(&candles(&rows), &Readings::compute(&candles(&rows)));

    assert_eq!(with_zero, baseline);
}

#[test]
fn a_flat_series_has_no_swing_score_and_a_zero_atr() {
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
    let series = StructureSeries::compute(&candles, &Readings::compute(&candles));
    let bar = series.latest().expect("a latest bar");

    assert_eq!(bar.score, None);
    assert_eq!(bar.state_code, None);
    assert_eq!(bar.high_relation_code, None);
    assert_eq!(bar.low_relation_code, None);
    assert_eq!(bar.size_percentile, None);
    assert_eq!(bar.readiness, Readiness::Unavailable);
    assert_eq!(
        bar.unavailable_reason,
        Some(UnavailableReason::InsufficientConfirmedPivots)
    );
    assert_eq!(bar.state_age_bars, 0);
    assert_eq!(bar.state_changes_50, 0);
    assert_eq!(bar.ema_distance_atr_20, None, "ATR 0 is invalid");
    assert_eq!(bar.ema_retest_code_20, None, "ATR 0 is invalid");
    assert_eq!(
        bar.ema_cross_code_20,
        Some(0),
        "the cross does not use ATR and a flat EMA is not crossed"
    );
    assert!(series.registry().is_empty());
}

#[test]
fn a_bar_that_is_both_candidates_is_discarded() {
    let mids = [100.0, 100.0, 100.0, 100.0, 100.0];
    let mut candles: Vec<Candle> = mids
        .iter()
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
        .collect();
    candles.push(
        Candle::new(5, 5, 100.0, 110.0, 90.0, 100.0, Some(10.0)).expect("the wide candle is valid"),
    );
    for bar in 6..11 {
        candles.push(
            Candle::new(bar, bar as i64, 100.0, 100.5, 99.5, 100.0, Some(10.0))
                .expect("the test candle is valid"),
        );
    }

    let series = StructureSeries::compute(&candles, &Readings::compute(&candles));

    assert!(
        series.registry().is_empty(),
        "a bar that is both a candidate high and a candidate low is discarded"
    );
    assert!(series.bars().iter().all(|bar| bar.score.is_none()));
}

#[test]
fn a_missing_ema_nulls_only_its_own_features() {
    let rows = load();
    let candles = candles(&rows);
    let mut readings = Readings::compute(&candles);
    readings.ema20 = quantrule_model::readings::ReadingSeries::new(vec![None; readings.len()]);

    let series = StructureSeries::compute(&candles, &readings);
    let bar = series.get(210).expect("bar 210");

    assert!(bar.score.is_some());
    assert_eq!(bar.ema_distance_atr_20, None);
    assert_eq!(bar.ema_cross_code_20, None);
    assert_eq!(bar.ema_retest_code_20, None);
    assert!(bar.ema_distance_atr_50.is_some());
    assert!(bar.ema_cross_code_50.is_some());
    assert!(bar.ema_retest_code_50.is_some());
    assert!(bar.ema_distance_atr_200.is_some());
    assert!(bar.ema_cross_code_200.is_some());
    assert!(bar.ema_retest_code_200.is_some());
}
