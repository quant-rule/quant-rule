//! Golden-fixture test for the dashboard Direction tenure view.
//!
//! The tenure fixture is a checked-in CSV of 260 rows. Its expected values were
//! produced by an independent Python replay of the approved tenure state machine
//! over the independently generated `direction_golden.csv` column, not by the
//! Rust code under test. The magnitude column comes from that same direction
//! fixture and must remain detail-only.
//!
//! An empty cell is an unavailable measurement or a percentile with fewer than
//! 50 prior samples; a numeric cell is a measured value.

use quantrule_data::Candle;
use quantrule_model::dashboard::TimeframeState;
use quantrule_model::dashboard::{
    AxisDetail, AxisValue, BiasContinuity, BiasState, TenureOriginKind,
};
use quantrule_model::readings::roles::AxisCode;
use quantrule_model::run::TimeframeModel;
use quantrule_model::state::{MarketStateConfig, TimeframeInput};

const DIRECTION_FIXTURE: &str = include_str!("fixtures/direction_golden.csv");
const TENURE_FIXTURE: &str = include_str!("fixtures/dashboard_direction_tenure_golden.csv");

const DIRECTION_HEADER: &str = "bar,timestamp,open,high,low,close,volume,direction,direction_state_code,direction_size_percentile,direction_readiness_code,direction_unavailable_reason_code,direction_state_age_bars,direction_state_changes_50";
const TENURE_HEADER: &str = "bar,direction,bias_state_code,bias_continuity_code,bias_tenure_bars,tenure_origin_kind_code,tenure_origin_bar,tenure_percentile";
const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy)]
struct FixtureRow {
    candle: Candle,
    magnitude: Option<f64>,
    bias_state: u8,
    bias_continuity: u8,
    tenure: Option<u64>,
    origin_kind: u8,
    origin_bar: Option<usize>,
    tenure_percentile: Option<f64>,
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

fn load() -> Vec<FixtureRow> {
    let mut direction_lines = DIRECTION_FIXTURE.lines();
    assert_eq!(direction_lines.next(), Some(DIRECTION_HEADER));
    let mut tenure_lines = TENURE_FIXTURE.lines();
    assert_eq!(tenure_lines.next(), Some(TENURE_HEADER));

    let mut rows = Vec::new();
    loop {
        let (Some(direction), Some(tenure)) = (direction_lines.next(), tenure_lines.next()) else {
            assert!(
                direction_lines.next().is_none() && tenure_lines.next().is_none(),
                "both fixtures must have the same number of rows"
            );
            break;
        };
        let fields: Vec<&str> = direction.split(',').collect();
        assert_eq!(fields.len(), 14);
        let tenure_fields: Vec<&str> = tenure.split(',').collect();
        assert_eq!(tenure_fields.len(), 8);

        let bar = integer::<usize>(fields[0]);
        assert_eq!(bar, rows.len());
        assert_eq!(tenure_fields[0], fields[0]);
        assert_eq!(tenure_fields[1], fields[7]);

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

        rows.push(FixtureRow {
            candle,
            magnitude: number(fields[9]),
            bias_state: integer(tenure_fields[2]),
            bias_continuity: integer(tenure_fields[3]),
            tenure: optional_integer(tenure_fields[4]),
            origin_kind: integer(tenure_fields[5]),
            origin_bar: optional_integer(tenure_fields[6]),
            tenure_percentile: number(tenure_fields[7]),
        });
    }
    rows
}

fn state_code(state: BiasState) -> u8 {
    match state {
        BiasState::Undefined => 0,
        BiasState::Bullish => 1,
        BiasState::Bearish => 2,
        BiasState::Neutral => 3,
    }
}

fn continuity_code(continuity: BiasContinuity) -> u8 {
    match continuity {
        BiasContinuity::Undefined => 0,
        BiasContinuity::Continuous => 1,
        BiasContinuity::Interrupted => 2,
    }
}

fn origin_code(kind: TenureOriginKind) -> u8 {
    match kind {
        TenureOriginKind::None => 0,
        TenureOriginKind::FirstEstablishment => 1,
        TenureOriginKind::Reversal => 2,
        TenureOriginKind::NewOriginAfterUnavailable => 3,
    }
}

#[test]
fn the_dashboard_direction_view_matches_the_tenure_golden() {
    let rows = load();
    let candles: Vec<Candle> = rows.iter().map(|row| row.candle).collect();
    let timeframe = TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: 60,
            candles: &candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the fixture is non-empty");

    let mut saw_tenure = false;
    let mut saw_no_tenure = false;
    let mut saw_magnitude = false;
    let mut saw_percentile = false;
    let mut saw_no_percentile = false;

    for (bar, row) in rows.iter().enumerate() {
        let state = TimeframeState::at(&timeframe, bar).expect("a state");
        let direction = state
            .axes
            .iter()
            .find(|axis| axis.axis == AxisCode::Direction)
            .expect("one Direction state");
        let AxisDetail::Direction(detail) = &direction.detail else {
            panic!("the Direction axis carries Direction detail");
        };

        assert_eq!(
            state_code(detail.bias_state),
            row.bias_state,
            "bar {bar} bias state"
        );
        assert_eq!(
            continuity_code(detail.bias_continuity),
            row.bias_continuity,
            "bar {bar} continuity"
        );
        assert_eq!(detail.bias_tenure_bars, row.tenure, "bar {bar} tenure");
        assert_eq!(
            origin_code(detail.tenure_origin_kind),
            row.origin_kind,
            "bar {bar} origin kind"
        );
        assert_eq!(
            detail.tenure_origin_bar, row.origin_bar,
            "bar {bar} origin bar"
        );

        match row.tenure_percentile {
            Some(expected) => {
                let actual = direction
                    .size
                    .as_rank()
                    .unwrap_or_else(|| panic!("bar {bar} has a tenure percentile"));
                assert!(
                    (actual - expected).abs() < TOLERANCE,
                    "bar {bar}: expected tenure percentile {expected}, got {actual}"
                );
                saw_percentile = true;
            }
            None => {
                assert!(direction.size.is_undefined(), "bar {bar} has no size");
                saw_no_percentile = true;
            }
        }

        match (detail.magnitude_pct, row.magnitude) {
            (Some(actual), Some(expected)) => {
                assert!(
                    (actual - expected).abs() < TOLERANCE,
                    "bar {bar}: expected magnitude {expected}, got {actual}"
                );
                saw_magnitude = true;
            }
            (None, None) => {}
            (actual, expected) => panic!(
                "bar {bar}: magnitude presence differs, actual {actual:?}, expected {expected:?}"
            ),
        }

        saw_tenure |= detail.bias_tenure_bars.is_some();
        saw_no_tenure |= detail.bias_tenure_bars.is_none();
        assert!(
            matches!(
                direction.size,
                AxisValue::Undefined { .. } | AxisValue::Rank { .. }
            ),
            "the Direction size is a rank or a typed absence"
        );
    }

    assert!(saw_tenure, "the fixture must publish a tenure");
    assert!(saw_no_tenure, "the fixture must publish a null tenure");
    assert!(saw_magnitude, "the fixture must publish a magnitude detail");
    assert!(saw_percentile, "the fixture must exercise the percentile");
    assert!(
        saw_no_percentile,
        "the fixture must cover fewer than 50 prior samples"
    );
}
