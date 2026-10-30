//! Golden-fixture test for the dashboard Structure evidence.
//!
//! The expected relation, scale, EMA-distance/cross/retest, score, and
//! swing-alignment values come from the independently generated
//! `structure_golden.csv`. The expected reference prices come from the
//! independently generated EMA columns of `readings_golden.csv` and the
//! confirmation order in `structure_levels_golden.csv`.
//!
//! The test exercises only the published dashboard view: it never calls the
//! Structure axis or its registry directly.

use quantrule_data::Candle;
use quantrule_model::dashboard::{AxisDetail, TimeframeState};
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::roles::AxisCode;
use quantrule_model::timeframe::TimeframeModel;

const STRUCTURE_FIXTURE: &str = include_str!("fixtures/structure_golden.csv");
const LEVELS_FIXTURE: &str = include_str!("fixtures/structure_levels_golden.csv");
const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");

const STRUCTURE_HEADER: &str = "bar,timestamp,open,high,low,close,volume,high_relation_code,low_relation_code,structure_score,structure_code,structure_scale_code,ema_distance_atr_20,ema_cross_code_20,ema_retest_code_20,ema_distance_atr_50,ema_cross_code_50,ema_retest_code_50,ema_distance_atr_200,ema_cross_code_200,ema_retest_code_200,structure_size_percentile,structure_readiness_code,structure_unavailable_reason_code,structure_state_age_bars,structure_state_changes_50";
const LEVELS_HEADER: &str =
    "sequence,reference_code,scale_code,kind,price,occurred_at,available_at";
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy)]
struct StructureRow {
    candle: Candle,
    high_relation: Option<i8>,
    low_relation: Option<i8>,
    score: Option<f64>,
    scale_code: u8,
    distance: [Option<f64>; 3],
    cross: [Option<i8>; 3],
    retest: [Option<i8>; 3],
    size_percentile: Option<f64>,
    readiness_code: u8,
}

#[derive(Debug, Clone, Copy)]
struct LevelRow {
    sequence: u32,
    scale_code: u8,
    kind: &'static str,
    price: f64,
    available_at: usize,
}

#[derive(Debug, Clone, Copy)]
struct EmaRow {
    ema: [Option<f64>; 3],
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

fn load_structure() -> Vec<StructureRow> {
    let mut lines = STRUCTURE_FIXTURE.lines();
    assert_eq!(lines.next(), Some(STRUCTURE_HEADER));
    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 26);
        let bar = integer::<usize>(fields[0]);
        assert_eq!(bar, rows.len());
        let candle = Candle::new(
            u64::try_from(bar).expect("the fixture bar fits in u64"),
            integer(fields[1]),
            integer(fields[2]),
            integer(fields[3]),
            integer(fields[4]),
            integer(fields[5]),
            number(fields[6]),
        )
        .expect("a valid fixture candle");
        rows.push(StructureRow {
            candle,
            high_relation: optional_integer(fields[7]),
            low_relation: optional_integer(fields[8]),
            score: number(fields[9]),
            scale_code: integer(fields[11]),
            distance: [number(fields[12]), number(fields[15]), number(fields[18])],
            cross: [
                optional_integer(fields[13]),
                optional_integer(fields[16]),
                optional_integer(fields[19]),
            ],
            retest: [
                optional_integer(fields[14]),
                optional_integer(fields[17]),
                optional_integer(fields[20]),
            ],
            size_percentile: number(fields[21]),
            readiness_code: integer(fields[22]),
        });
    }
    rows
}

fn load_levels() -> Vec<LevelRow> {
    let mut lines = LEVELS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(LEVELS_HEADER));
    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 7);
        rows.push(LevelRow {
            sequence: integer(fields[0]),
            scale_code: integer(fields[2]),
            kind: match fields[3] {
                "high" => "high",
                "low" => "low",
                other => panic!("unexpected level kind {other:?}"),
            },
            price: number(fields[4]).expect("a level price"),
            available_at: integer(fields[6]),
        });
    }
    rows
}

fn load_emas() -> Vec<EmaRow> {
    let mut lines = READINGS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(READINGS_HEADER));
    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23);
        let bar = integer::<usize>(fields[0]);
        assert_eq!(bar, rows.len());
        rows.push(EmaRow {
            ema: [number(fields[7]), number(fields[8]), number(fields[9])],
        });
    }
    rows
}

fn reference_kind(scale_code: u8, kind: &str) -> u8 {
    match (scale_code, kind) {
        (1, "high") => 4,
        (1, "low") => 5,
        (2, "high") => 6,
        (2, "low") => 7,
        _ => panic!("unexpected level {scale_code}/{kind}"),
    }
}

fn assert_close(actual: Option<f64>, expected: Option<f64>, what: &str) {
    match (actual, expected) {
        (Some(actual), Some(expected)) => assert!(
            (actual - expected).abs() < TOLERANCE,
            "{what}: expected {expected}, got {actual}"
        ),
        (None, None) => {}
        (actual, expected) => {
            panic!("{what}: presence differs, actual {actual:?}, expected {expected:?}")
        }
    }
}

#[test]
fn the_dashboard_structure_evidence_matches_the_golden_fixtures() {
    let structure = load_structure();
    let levels = load_levels();
    let emas = load_emas();
    assert_eq!(structure.len(), emas.len());
    let candles: Vec<Candle> = structure.iter().map(|row| row.candle).collect();
    let timeframe = TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: 60,
            candles: &candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the fixture is non-empty");

    let mut saw_swing_reference = false;
    let mut saw_confirmed_level = false;

    for (bar, row) in structure.iter().enumerate() {
        let state = TimeframeState::at(&timeframe, bar).expect("a state");
        let structure_axis = state
            .axes
            .iter()
            .find(|axis| axis.axis == AxisCode::Structure)
            .expect("one Structure state");
        let AxisDetail::Structure(detail) = &structure_axis.detail else {
            panic!("the Structure axis carries Structure detail");
        };

        assert_eq!(
            detail.high_relation_code, row.high_relation,
            "bar {bar} high relation"
        );
        assert_eq!(
            detail.low_relation_code, row.low_relation,
            "bar {bar} low relation"
        );
        assert_close(detail.score, row.score, &format!("bar {bar} score"));
        assert_eq!(detail.scale_code, row.scale_code, "bar {bar} scale");
        for (index, expected) in row.distance.iter().enumerate() {
            let actual = match index {
                0 => detail.ema_distance_atr_20,
                1 => detail.ema_distance_atr_50,
                2 => detail.ema_distance_atr_200,
                _ => unreachable!(),
            };
            assert_close(
                actual,
                *expected,
                &format!("bar {bar} EMA distance {index}"),
            );
        }
        for (index, expected) in row.cross.iter().enumerate() {
            let actual = match index {
                0 => detail.ema_cross_code_20,
                1 => detail.ema_cross_code_50,
                2 => detail.ema_cross_code_200,
                _ => unreachable!(),
            };
            assert_eq!(actual, *expected, "bar {bar} EMA cross {index}");
        }
        for (index, expected) in row.retest.iter().enumerate() {
            let actual = match index {
                0 => detail.ema_retest_code_20,
                1 => detail.ema_retest_code_50,
                2 => detail.ema_retest_code_200,
                _ => unreachable!(),
            };
            assert_eq!(actual, *expected, "bar {bar} EMA retest {index}");
        }
        assert_eq!(
            structure_axis.readiness_code, row.readiness_code,
            "bar {bar} readiness"
        );
        match row.size_percentile {
            Some(expected) => {
                let actual = structure_axis
                    .size
                    .as_rank()
                    .unwrap_or_else(|| panic!("bar {bar} has a size percentile"));
                assert!(
                    (actual - expected).abs() < TOLERANCE,
                    "bar {bar}: expected size {expected}, got {actual}"
                );
            }
            None => assert!(structure_axis.size.is_undefined(), "bar {bar} has no size"),
        }

        let mut expected_references: Vec<(u8, f64, usize)> = Vec::new();
        for (index, kind) in [1_u8, 2, 3].into_iter().enumerate() {
            if let Some(price) = emas[bar].ema[index] {
                expected_references.push((kind, price, bar));
            }
        }
        for (scale_code, kind) in [(1_u8, "high"), (1, "low"), (2, "high"), (2, "low")] {
            let latest = levels
                .iter()
                .filter(|level| {
                    level.scale_code == scale_code
                        && level.kind == kind
                        && level.available_at <= bar
                })
                .max_by_key(|level| level.sequence);
            if let Some(level) = latest {
                expected_references.push((
                    reference_kind(level.scale_code, level.kind),
                    level.price,
                    level.available_at,
                ));
            }
        }

        assert_eq!(
            detail.level_references.len(),
            expected_references.len(),
            "bar {bar} reference count"
        );
        for (actual, expected) in detail.level_references.iter().zip(&expected_references) {
            assert_eq!(
                actual.reference_kind_code, expected.0,
                "bar {bar} reference order"
            );
            assert!(
                (actual.price - expected.1).abs() < TOLERANCE,
                "bar {bar}: expected reference price {}, got {}",
                expected.1,
                actual.price
            );
            assert_eq!(
                actual.available_bar, expected.2,
                "bar {bar} reference availability"
            );
            saw_swing_reference |= actual.reference_kind_code >= 4;
        }

        let expected_count = levels
            .iter()
            .filter(|level| level.available_at <= bar)
            .count();
        assert_eq!(
            detail.confirmed_level_count, expected_count,
            "bar {bar} confirmed level count"
        );
        saw_confirmed_level |= expected_count > 0;
    }

    assert!(
        saw_swing_reference,
        "the fixture must publish a confirmed swing reference"
    );
    assert!(
        saw_confirmed_level,
        "the fixture must publish a confirmed level"
    );
}
